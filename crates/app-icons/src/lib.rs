use parking_lot::Mutex;
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessMetadata {
    pub executable_path: Option<String>,
    pub executable_name: Option<String>,
    pub aumid: Option<String>,
}

#[derive(Default)]
pub struct IconCache {
    values: Mutex<HashMap<String, Option<String>>>,
}

impl IconCache {
    pub fn icon_for_path(&self, path: &Path) -> Option<String> {
        let metadata = path.metadata().ok()?;
        let modified = metadata
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let key = format!(
            "{}::{modified}::{}",
            canonical.to_string_lossy().to_lowercase(),
            metadata.len()
        );
        if let Some(value) = self.values.lock().get(&key) {
            return value.clone();
        }
        let value = platform::extract_icon(&canonical);
        self.values.lock().insert(key, value.clone());
        value
    }
}

pub fn metadata_for_process(process_id: u32) -> ProcessMetadata {
    platform::metadata_for_process(process_id)
}

#[cfg(not(windows))]
mod platform {
    use super::ProcessMetadata;
    use std::path::Path;

    pub fn metadata_for_process(_process_id: u32) -> ProcessMetadata {
        ProcessMetadata {
            executable_path: None,
            executable_name: None,
            aumid: None,
        }
    }

    pub fn extract_icon(_path: &Path) -> Option<String> {
        None
    }
}

#[cfg(windows)]
mod platform {
    use super::ProcessMetadata;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use image::{DynamicImage, ImageFormat, RgbaImage};
    use std::{ffi::c_void, io::Cursor, path::Path};
    use windows::{
        core::{PCWSTR, PWSTR},
        Win32::{
            Foundation::{CloseHandle, HANDLE},
            Graphics::Gdi::{
                CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject,
                BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
            },
            Storage::{
                FileSystem::FILE_FLAGS_AND_ATTRIBUTES, Packaging::Appx::GetApplicationUserModelId,
            },
            System::Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
            UI::{
                Shell::{ExtractIconExW, SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON},
                WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL, HICON},
            },
        },
    };

    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    struct Icon(HICON);
    impl Drop for Icon {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyIcon(self.0);
            }
        }
    }

    pub fn metadata_for_process(process_id: u32) -> ProcessMetadata {
        unsafe {
            let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id)
            else {
                return ProcessMetadata {
                    executable_path: None,
                    executable_name: None,
                    aumid: None,
                };
            };
            let handle = Handle(handle);
            let mut path = vec![0u16; 32768];
            let mut length = path.len() as u32;
            let executable_path = QueryFullProcessImageNameW(
                handle.0,
                PROCESS_NAME_FORMAT(0),
                PWSTR(path.as_mut_ptr()),
                &mut length,
            )
            .ok()
            .map(|_| String::from_utf16_lossy(&path[..length as usize]));
            let executable_name = executable_path
                .as_ref()
                .and_then(|value| Path::new(value).file_name())
                .map(|value| value.to_string_lossy().into_owned());
            let mut aumid_length = 0u32;
            let first = GetApplicationUserModelId(handle.0, &mut aumid_length, None);
            let aumid = if first.0 == 122 || first.0 == 15703 {
                let mut value = vec![0u16; aumid_length as usize];
                GetApplicationUserModelId(
                    handle.0,
                    &mut aumid_length,
                    Some(PWSTR(value.as_mut_ptr())),
                )
                .ok()
                .ok()
                .map(|_| {
                    String::from_utf16_lossy(
                        &value[..value.iter().position(|ch| *ch == 0).unwrap_or(value.len())],
                    )
                })
            } else {
                None
            };
            ProcessMetadata {
                executable_path,
                executable_name,
                aumid,
            }
        }
    }

    pub fn extract_icon(path: &Path) -> Option<String> {
        unsafe {
            let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let mut info = SHFILEINFOW::default();
            let result = SHGetFileInfoW(
                PCWSTR(wide.as_ptr()),
                FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut info),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            );
            let icon = if result != 0 && !info.hIcon.is_invalid() {
                Icon(info.hIcon)
            } else {
                let mut large = HICON::default();
                if ExtractIconExW(PCWSTR(wide.as_ptr()), 0, Some(&mut large), None, 1) == 0
                    || large.is_invalid()
                {
                    return None;
                }
                Icon(large)
            };
            icon_to_data_url(icon.0)
        }
    }

    /// Releases the memory device context on every exit path, including the
    /// early return when the bitmap cannot be created. This runs once per icon
    /// per session refresh, so a leaked context accumulates.
    struct MemoryDc(windows::Win32::Graphics::Gdi::HDC);

    impl Drop for MemoryDc {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteDC(self.0);
            }
        }
    }

    /// Releases the DIB section regardless of how the conversion ends.
    struct DibSection(windows::Win32::Graphics::Gdi::HBITMAP);

    impl Drop for DibSection {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(self.0 .0));
            }
        }
    }

    unsafe fn icon_to_data_url(icon: HICON) -> Option<String> {
        let raw_dc = CreateCompatibleDC(None);
        if raw_dc.is_invalid() {
            return None;
        }
        let dc_guard = MemoryDc(raw_dc);
        let dc = dc_guard.0;

        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 32,
                biHeight: -32,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap =
            DibSection(CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).ok()?);
        let previous = SelectObject(dc, HGDIOBJ(bitmap.0 .0));
        let drawn = DrawIconEx(dc, 0, 0, icon, 32, 32, 0, None, DI_NORMAL).is_ok();
        SelectObject(dc, previous);
        let mut bgra = vec![0u8; 32 * 32 * 4];
        if drawn {
            std::ptr::copy_nonoverlapping(bits.cast::<u8>(), bgra.as_mut_ptr(), bgra.len());
        }
        // The pixels are copied out, so both GDI objects can go now; the rest of
        // this function only touches `bgra`.
        drop(bitmap);
        drop(dc_guard);
        if !drawn {
            return None;
        }
        for pixel in bgra.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        let image = RgbaImage::from_raw(32, 32, bgra)?;
        let mut png = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(image)
            .write_to(&mut png, ImageFormat::Png)
            .ok()?;
        Some(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(png.into_inner())
        ))
    }

    use std::os::windows::ffi::OsStrExt;
}
