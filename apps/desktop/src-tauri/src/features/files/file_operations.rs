use std::{
    fs::{self, Metadata, OpenOptions},
    io,
    path::{Path, PathBuf},
};

use tempfile::{Builder, NamedTempFile};

pub(crate) struct Item {
    pub path: PathBuf,
    metadata: Metadata,
}

impl Item {
    pub fn read(path: &Path) -> Result<Self, String> {
        validate_path(path)?;
        let metadata = fs::symlink_metadata(path).map_err(|error| path_error(path, error))?;
        require_regular(&metadata).map_err(|error| path_error(path, error))?;
        let path = dunce::canonicalize(path).map_err(|error| path_error(path, error))?;
        Ok(Self { path, metadata })
    }

    pub fn is_dir(&self) -> bool {
        self.metadata.is_dir()
    }

    pub fn check_movable(&self, current: &Path) -> Result<(), String> {
        if self.path.parent().is_none() || current.starts_with(&self.path) {
            return Err("请先切换目录。卷根目录、当前目录及其上级目录受到保护。".into());
        }
        Ok(())
    }

    pub fn ensure_unchanged(&self) -> Result<(), String> {
        let current = Self::read(&self.path)?;
        let before = &self.metadata;
        let after = &current.metadata;
        if self.path != current.path
            || before.is_dir() != after.is_dir()
            || before.len() != after.len()
            || before.modified().ok() != after.modified().ok()
            || before.created().ok() != after.created().ok()
        {
            return Err("目标已发生变化，请重新检查后再次操作。".into());
        }
        Ok(())
    }
}

pub(crate) fn make_directory(path: &Path) -> Result<String, String> {
    let target = resolve_target(path)?;
    ensure_missing(&target).map_err(|error| path_error(&target, error))?;
    fs::create_dir(&target).map_err(|error| path_error(&target, error))?;
    Ok(format!("已创建目录「{}」。", target.display()))
}

pub(crate) fn copy(source: &Path, destination: &Path, recursive: bool) -> Result<String, String> {
    let source = Item::read(source)?;
    if source.is_dir() && !recursive {
        return Err("复制目录请使用 cp -r。".into());
    }
    let target = transfer_target(&source, destination)?;
    copy_item(&source, &target).map_err(|error| path_error(&target, error))?;
    Ok(format!("已复制到「{}」。", target.display()))
}

pub(crate) fn move_item(
    source: &Path,
    destination: &Path,
    current: &Path,
) -> Result<String, String> {
    let source = Item::read(source)?;
    source.check_movable(current)?;
    let target = transfer_target(&source, destination)?;
    if let Err(error) = rename_without_overwrite(&source.path, &target) {
        if error.kind() != io::ErrorKind::CrossesDevices {
            return Err(path_error(&target, error));
        }
        if source.is_dir() {
            return Err("跨盘移动目录可以先用 cp -r 复制，再用 rm -r 确认回收原目录。".into());
        }
        copy_item(&source, &target).map_err(|error| path_error(&target, error))?;
        source
            .ensure_unchanged()
            .and_then(|()| {
                fs::remove_file(&source.path).map_err(|error| path_error(&source.path, error))
            })
            .map_err(|error| format!("已复制到「{}」，原文件仍保留。{error}", target.display()))?;
    }
    Ok(format!("已移动到「{}」。", target.display()))
}

fn transfer_target(source: &Item, destination: &Path) -> Result<PathBuf, String> {
    let name = source.path.file_name().ok_or("操作目标不能是卷根目录。")?;
    let target = match fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.is_dir() => {
            let directory = Item::read(destination)?;
            resolve_target(&directory.path.join(name))?
        }
        Ok(_) => return Err(conflict(destination)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => resolve_target(destination)?,
        Err(error) => return Err(path_error(destination, error)),
    };
    ensure_missing(&target).map_err(|error| path_error(&target, error))?;
    if source.is_dir() && target.starts_with(&source.path) {
        return Err("目标目录位于源目录内部，请指定其他位置。".into());
    }
    Ok(target)
}

fn resolve_target(path: &Path) -> Result<PathBuf, String> {
    validate_path(path)?;
    let name = path.file_name().ok_or("请输入文件或目录名称。")?;
    let parent = path.parent().ok_or("目标目录不可用。")?;
    #[cfg(windows)]
    {
        let name = name.to_string_lossy();
        if name.ends_with(['.', ' '])
            || name
                .chars()
                .any(|character| r#"<>:"|?*"#.contains(character))
        {
            return Err("目标名称包含 Windows 不支持的字符。".into());
        }
    }
    let parent = dunce::canonicalize(parent).map_err(|error| path_error(parent, error))?;
    Ok(parent.join(name))
}

fn validate_path(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        if matches!(
            path.components().next(),
            Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::DeviceNS(_) | Prefix::Verbatim(_))
        ) || path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().contains(':'))
        {
            return Err("请选择普通的文件或目录路径。".into());
        }
    }
    #[cfg(not(windows))]
    let _ = path;
    Ok(())
}

fn ensure_missing(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(io::ErrorKind::AlreadyExists.into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn conflict(path: &Path) -> String {
    format!("目标「{}」已存在，请指定其他名称。", path.display())
}

fn path_error(path: &Path, error: io::Error) -> String {
    match error.kind() {
        io::ErrorKind::AlreadyExists => conflict(path),
        io::ErrorKind::NotFound => format!("找不到「{}」。", path.display()),
        _ => format!("无法处理「{}」（{error}）。", path.display()),
    }
}

fn require_regular(metadata: &Metadata) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::Error::other("此命令暂不处理链接或联接点"));
        }
    }
    if !metadata.is_dir() && !metadata.is_file() {
        return Err(io::Error::other("请选择普通文件或目录"));
    }
    Ok(())
}

fn copy_item(source: &Item, destination: &Path) -> io::Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::other("目标目录不可用"))?;
    if !source.is_dir() {
        return copy_file(&source.path, destination);
    }

    let staging = Builder::new().prefix(".blis-copy-").tempdir_in(parent)?;
    let mut directories = vec![(source.path.clone(), staging.path().to_path_buf())];
    while let Some((source, destination)) = directories.pop() {
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            require_regular(&metadata)?;
            let target = destination.join(entry.file_name());
            if metadata.is_dir() {
                fs::create_dir(&target)?;
                directories.push((entry.path(), target));
            } else {
                copy_file(&entry.path(), &target)?;
            }
        }
    }
    rename_without_overwrite(staging.path(), destination)?;
    let _ = staging.keep();
    Ok(())
}

fn copy_file(source: &Path, destination: &Path) -> io::Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::other("目标目录不可用"))?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let mut source = options.open(source)?;
    let metadata = source.metadata()?;
    require_regular(&metadata)?;
    if !metadata.is_file() {
        return Err(io::Error::other("源路径不是文件"));
    }
    let mut staging = NamedTempFile::new_in(parent)?;
    io::copy(&mut source, &mut staging)?;
    staging.as_file().sync_all()?;
    staging.persist_noclobber(destination)?;
    Ok(())
}

#[cfg(windows)]
fn rename_without_overwrite(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

    let source: Vec<_> = source.as_os_str().encode_wide().chain([0]).collect();
    let destination: Vec<_> = destination.as_os_str().encode_wide().chain([0]).collect();
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0) } != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(windows))]
fn rename_without_overwrite(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "当前系统暂不支持此操作",
    ))
}

pub(crate) fn recycle(item: &Item) -> Result<String, String> {
    recycle_path(&item.path)
        .map_err(|error| format!("无法将「{}」送入回收站（{error}）。", item.path.display()))?;
    Ok(format!("已将「{}」送入回收站。", item.path.display()))
}

#[cfg(windows)]
fn recycle_path(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use windows::{
        core::HSTRING,
        Win32::{
            System::Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
                COINIT_APARTMENTTHREADED,
            },
            UI::Shell::{
                FileOperation, IFileOperation, IShellItem, SHCreateItemFromParsingName,
                FOFX_ADDUNDORECORD, FOFX_EARLYFAILURE, FOFX_RECYCLEONDELETE, FOF_NOCONFIRMATION,
                FOF_NOERRORUI, FOF_SILENT,
            },
        },
    };

    struct ComApartment;
    impl Drop for ComApartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }

    // Each call runs on a blocking worker and requests recycling through Windows Shell.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
    let _com = ComApartment;
    let operation: IFileOperation =
        unsafe { CoCreateInstance(&FileOperation, None, CLSCTX_INPROC_SERVER)? };
    let item: IShellItem =
        unsafe { SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None)? };
    unsafe {
        operation.SetOperationFlags(
            FOFX_RECYCLEONDELETE
                | FOFX_ADDUNDORECORD
                | FOFX_EARLYFAILURE
                | FOF_NOCONFIRMATION
                | FOF_NOERRORUI
                | FOF_SILENT,
        )?;
        operation.DeleteItem(&item, None)?;
        operation.PerformOperations()?;
        if operation.GetAnyOperationsAborted()?.as_bool() {
            return Err("Windows 已取消回收".into());
        }
    }
    ensure_missing(path)?;
    Ok(())
}

#[cfg(not(windows))]
fn recycle_path(_: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Err("当前系统暂不支持回收站".into())
}
