use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

#[repr(C)]
#[allow(non_snake_case)]
struct OPENFILENAMEW {
    lStructSize: u32,
    hwndOwner: *mut std::ffi::c_void,
    hInstance: *mut std::ffi::c_void,
    lpstrFilter: *const u16,
    lpstrCustomFilter: *mut u16,
    nMaxCustFilter: u32,
    nFilterIndex: u32,
    lpstrFile: *mut u16,
    nMaxFile: u32,
    lpstrFileTitle: *mut u16,
    nMaxFileTitle: u32,
    lpstrInitialDir: *const u16,
    lpstrTitle: *const u16,
    Flags: u32,
    nFileOffset: u16,
    nFileExtension: u16,
    lpstrDefExt: *const u16,
    lCustData: isize,
    lpfnHook: *mut std::ffi::c_void,
    lpTemplateName: *const u16,
    pvReserved: *mut std::ffi::c_void,
    dwReserved: u32,
    FlagsEx: u32,
}

#[cfg(windows)]
extern "system" {
    fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
    fn GetForegroundWindow() -> *mut std::ffi::c_void;
}

pub fn pick_windows_executable() -> Option<String> {
    #[cfg(windows)]
    unsafe {
        let mut file_buf = vec![0u16; 1024];

        // Filter: "Applications (*.exe)\0*.exe\0All Files (*.*)\0*.*\0\0"
        let filter: Vec<u16> = "Applications (*.exe)\0*.exe\0All Files (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect();

        let title: Vec<u16> = "Select Application Executable\0".encode_utf16().collect();
        let def_ext: Vec<u16> = "exe\0".encode_utf16().collect();

        // Own dialog to current foreground window so it stays modal on top of Aether Desktop
        let owner_hwnd = GetForegroundWindow();

        let mut ofn = OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
            hwndOwner: owner_hwnd,
            hInstance: std::ptr::null_mut(),
            lpstrFilter: filter.as_ptr(),
            lpstrCustomFilter: std::ptr::null_mut(),
            nMaxCustFilter: 0,
            nFilterIndex: 1,
            lpstrFile: file_buf.as_mut_ptr(),
            nMaxFile: file_buf.len() as u32,
            lpstrFileTitle: std::ptr::null_mut(),
            nMaxFileTitle: 0,
            lpstrInitialDir: std::ptr::null_mut(),
            lpstrTitle: title.as_ptr(),
            // OFN_PATHMUSTEXIST (0x800) | OFN_FILEMUSTEXIST (0x1000) | OFN_EXPLORER (0x80000)
            Flags: 0x00000800 | 0x00001000 | 0x00080000,
            nFileOffset: 0,
            nFileExtension: 0,
            lpstrDefExt: def_ext.as_ptr(),
            lCustData: 0,
            lpfnHook: std::ptr::null_mut(),
            lpTemplateName: std::ptr::null_mut(),
            pvReserved: std::ptr::null_mut(),
            dwReserved: 0,
            FlagsEx: 0,
        };

        if GetOpenFileNameW(&mut ofn) != 0 {
            let len = file_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(file_buf.len());
            let os_str = OsString::from_wide(&file_buf[..len]);
            return os_str.into_string().ok();
        }
        None
    }

    #[cfg(not(windows))]
    None
}
