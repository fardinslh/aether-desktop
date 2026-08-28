use base64::Engine;
use image::ImageEncoder;
use std::path::Path;

pub struct IconExtractor;

impl IconExtractor {
    pub fn extract_base64(exe_path: &str) -> Option<String> {
        extract_icon_base64(exe_path)
    }
}

#[cfg(windows)]
pub fn extract_icon_base64(exe_path: &str) -> Option<String> {
    if !Path::new(exe_path).exists() {
        return None;
    }

    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::UI::Shell::ExtractIconExW;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    let wide_path: Vec<u16> = OsStr::new(exe_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut large_icon: HICON = std::ptr::null_mut();
        let count = ExtractIconExW(
            wide_path.as_ptr(),
            0,
            &mut large_icon,
            std::ptr::null_mut(),
            1,
        );
        if count == 0 || large_icon.is_null() {
            return None;
        }

        let mut icon_info = ICONINFO {
            fIcon: 0,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: std::ptr::null_mut(),
            hbmColor: std::ptr::null_mut(),
        };

        if GetIconInfo(large_icon, &mut icon_info) == 0 {
            DestroyIcon(large_icon);
            return None;
        }

        let hdc = GetDC(std::ptr::null_mut());
        let mem_dc = CreateCompatibleDC(hdc);

        let mut bmp = BITMAP {
            bmType: 0,
            bmWidth: 0,
            bmHeight: 0,
            bmWidthBytes: 0,
            bmPlanes: 0,
            bmBitsPixel: 0,
            bmBits: std::ptr::null_mut(),
        };

        GetObjectW(
            icon_info.hbmColor,
            std::mem::size_of::<BITMAP>() as i32,
            &mut bmp as *mut _ as *mut _,
        );

        let width = bmp.bmWidth as u32;
        let height = bmp.bmHeight as u32;

        if width == 0 || height == 0 {
            DeleteDC(mem_dc);
            ReleaseDC(std::ptr::null_mut(), hdc);
            if !icon_info.hbmColor.is_null() {
                DeleteObject(icon_info.hbmColor);
            }
            if !icon_info.hbmMask.is_null() {
                DeleteObject(icon_info.hbmMask);
            }
            DestroyIcon(large_icon);
            return None;
        }

        let mut bi = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // top-down orientation
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: (width * height * 4) as u32,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };

        let mut bgra_data = vec![0u8; (width * height * 4) as usize];
        GetDIBits(
            mem_dc,
            icon_info.hbmColor,
            0,
            height,
            bgra_data.as_mut_ptr() as *mut _,
            &mut bi as *mut _ as *mut _,
            DIB_RGB_COLORS,
        );

        // Convert BGRA byte order to standard RGBA
        let mut rgba_data = vec![0u8; (width * height * 4) as usize];
        for i in (0..bgra_data.len()).step_by(4) {
            rgba_data[i] = bgra_data[i + 2]; // Red
            rgba_data[i + 1] = bgra_data[i + 1]; // Green
            rgba_data[i + 2] = bgra_data[i]; // Blue
            rgba_data[i + 3] = bgra_data[i + 3]; // Alpha
        }

        // Clean up all Win32 GDI handles
        DeleteDC(mem_dc);
        ReleaseDC(std::ptr::null_mut(), hdc);
        if !icon_info.hbmColor.is_null() {
            DeleteObject(icon_info.hbmColor);
        }
        if !icon_info.hbmMask.is_null() {
            DeleteObject(icon_info.hbmMask);
        }
        DestroyIcon(large_icon);

        // Encode RGBA pixels to PNG format
        let mut png_bytes = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
        if encoder
            .write_image(&rgba_data, width, height, image::ExtendedColorType::Rgba8)
            .is_ok()
        {
            let base64_str = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
            Some(format!("data:image/png;base64,{}", base64_str))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
pub fn extract_icon_base64(_exe_path: &str) -> Option<String> {
    None
}
