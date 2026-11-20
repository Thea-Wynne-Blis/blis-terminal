use std::{io, path::Path, process::Command};

#[cfg(windows)]
use std::path::PathBuf;

const CODE_NOT_FOUND: &str =
    "找不到 VS Code。请安装 VS Code，或将其 bin 目录加入 PATH 后重启 Blis。";

pub(crate) fn open_in_code(target: &Path) -> Result<(), String> {
    #[cfg(windows)]
    let executable = installed_code().ok_or(CODE_NOT_FOUND)?;
    #[cfg(not(windows))]
    let executable = "code";

    Command::new(executable)
        .arg(target)
        .env_remove("ELECTRON_RUN_AS_NODE")
        .spawn()
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => CODE_NOT_FOUND.to_owned(),
            _ => format!("无法启动 VS Code（{error}）。"),
        })?;
    Ok(())
}

#[cfg(windows)]
fn installed_code() -> Option<PathBuf> {
    use std::env;

    let path = env::var_os("PATH").unwrap_or_default();
    let standard_directories = [
        ("LOCALAPPDATA", "Programs/Microsoft VS Code"),
        ("ProgramFiles", "Microsoft VS Code"),
        ("ProgramFiles(x86)", "Microsoft VS Code"),
    ]
    .into_iter()
    .filter_map(|(variable, suffix)| Some(PathBuf::from(env::var_os(variable)?).join(suffix)));
    find_code(env::split_paths(&path).chain(standard_directories))
}

#[cfg(windows)]
fn find_code(directories: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    directories
        .into_iter()
        .filter(|directory| directory.is_absolute())
        .flat_map(|directory| {
            // The Windows installer adds VS Code's bin subdirectory to PATH.
            let from_cli_bin = directory
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("bin"));
            let install_directory = directory.parent().filter(|_| from_cli_bin);
            std::iter::once(directory.join("Code.exe"))
                .chain(install_directory.map(|root| root.join("Code.exe")))
        })
        .find(|candidate| candidate.is_file())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn finds_code_next_to_the_cli_bin_or_directly_in_the_install_directory() {
        let sandbox = tempfile::tempdir().unwrap();
        let install = sandbox.path().join("Microsoft VS Code");
        let bin = install.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let executable = install.join("Code.exe");
        std::fs::write(&executable, "").unwrap();
        assert_eq!(find_code([bin]), Some(executable.clone()));
        assert_eq!(find_code([install]), Some(executable));
    }
}
