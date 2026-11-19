use std::{ffi::OsStr, io};

#[cfg(windows)]
pub(crate) fn open_with_system(target: &OsStr) -> io::Result<()> {
    use std::{iter, os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::{
        System::Com::{
            CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
        },
        UI::{
            Shell::{ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    };

    let target: Vec<u16> = target.encode_wide().chain(iter::once(0)).collect();
    let mut request = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        lpFile: target.as_ptr(),
        nShow: SW_SHOWNORMAL,
        ..Default::default()
    };

    // The blocking worker has no message loop. Keep the UTF-16 target alive until handoff.
    unsafe {
        let initialized = CoInitializeEx(
            ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        ) >= 0;
        let result = if ShellExecuteExW(&mut request) != 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        };
        if initialized {
            CoUninitialize();
        }
        result
    }
}

#[cfg(not(windows))]
pub(crate) fn open_with_system(target: &OsStr) -> io::Result<()> {
    open::that_detached(target)
}
