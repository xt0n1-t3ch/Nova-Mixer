use parking_lot::Mutex;
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessMetadata {
    pub executable_path: Option<String>,
    pub executable_name: Option<String>,
    pub aumid: Option<String>,
    pub version_name: Option<String>,
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

    /// Returns the shell icon of a packaged app, looked up by its AUMID.
    pub fn icon_for_aumid(&self, aumid: &str) -> Option<String> {
        let key = format!("aumid::{}", aumid.to_lowercase());
        if let Some(value) = self.values.lock().get(&key) {
            return value.clone();
        }
        let value = platform::extract_aumid_icon(aumid);
        self.values.lock().insert(key, value.clone());
        value
    }
}

pub fn metadata_for_process(process_id: u32) -> ProcessMetadata {
    platform::metadata_for_process(process_id)
}

pub fn metadata_for_path(path: &Path) -> ProcessMetadata {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    ProcessMetadata {
        executable_name: canonical
            .file_name()
            .map(|name| name.to_string_lossy().into_owned()),
        executable_path: Some(canonical.to_string_lossy().into_owned()),
        aumid: None,
        version_name: platform::version_name(&canonical),
    }
}

pub fn fallback_name(path_or_name: &str) -> String {
    let stem = Path::new(path_or_name)
        .file_stem()
        .unwrap_or_else(|| path_or_name.as_ref())
        .to_string_lossy();
    let mut capitalize = true;
    stem.chars()
        .map(|character| {
            if character.is_alphanumeric() {
                let mapped = if capitalize {
                    character.to_uppercase().next().unwrap_or(character)
                } else {
                    character
                };
                capitalize = false;
                mapped
            } else {
                capitalize = true;
                character
            }
        })
        .collect()
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
            version_name: None,
        }
    }

    pub fn version_name(_path: &Path) -> Option<String> {
        None
    }

    pub fn extract_icon(_path: &Path) -> Option<String> {
        None
    }

    pub fn extract_aumid_icon(_aumid: &str) -> Option<String> {
        None
    }
}

#[cfg(windows)]
mod platform {
    use super::ProcessMetadata;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use image::{imageops::FilterType, DynamicImage, ImageFormat, RgbaImage};
    use std::{ffi::c_void, io::Cursor, path::Path};
    use windows::{
        core::{PCWSTR, PWSTR},
        Win32::{
            Foundation::{CloseHandle, HANDLE, SIZE},
            Graphics::Gdi::{
                CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDIBits,
                GetObjectW, SelectObject, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
                DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
            },
            Storage::{
                FileSystem::FILE_FLAGS_AND_ATTRIBUTES, Packaging::Appx::GetApplicationUserModelId,
            },
            System::{
                Com::{
                    CoInitializeEx, CoUninitialize, IBindCtx, COINIT_APARTMENTTHREADED,
                    COINIT_DISABLE_OLE1DDE,
                },
                Threading::{
                    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
                    PROCESS_QUERY_LIMITED_INFORMATION,
                },
            },
            UI::{
                Shell::{
                    ExtractIconExW, FOLDERID_AppsFolder, IShellItemImageFactory,
                    SHCreateItemFromParsingName, SHCreateItemInKnownFolder, SHGetFileInfoW,
                    KF_FLAG_DEFAULT, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SIIGBF_BIGGERSIZEOK,
                    SIIGBF_ICONONLY,
                },
                WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL, HICON},
            },
        },
    };

    /// Size requested from the shell. 256 selects the jumbo icon frame.
    const REQUEST_SIZE: u32 = 256;
    /// Largest edge stored in the data URL. The UI draws icons at up to 44 CSS
    /// px on 2x displays, so 128 px stays sharp while keeping settings small.
    const OUTPUT_SIZE: u32 = 128;

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
                    version_name: None,
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
            let version_name = executable_path
                .as_deref()
                .and_then(|path| version_name(Path::new(path)));
            ProcessMetadata {
                executable_path,
                executable_name,
                aumid,
                version_name,
            }
        }
    }

    pub fn version_name(path: &Path) -> Option<String> {
        use windows::Win32::Storage::FileSystem::{
            GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
        };
        unsafe {
            let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let size = GetFileVersionInfoSizeW(PCWSTR(wide.as_ptr()), None);
            if size == 0 {
                return None;
            }
            let mut data = vec![0u8; size as usize];
            GetFileVersionInfoW(PCWSTR(wide.as_ptr()), None, size, data.as_mut_ptr().cast())
                .ok()?;
            let translation_key: Vec<u16> = "\\VarFileInfo\\Translation\0".encode_utf16().collect();
            let mut translation = std::ptr::null_mut();
            let mut translation_len = 0;
            let (language, codepage) = if VerQueryValueW(
                data.as_ptr().cast(),
                PCWSTR(translation_key.as_ptr()),
                &mut translation,
                &mut translation_len,
            )
            .as_bool()
                && translation_len >= 4
            {
                let values = std::slice::from_raw_parts(translation.cast::<u16>(), 2);
                (values[0], values[1])
            } else {
                (0x0409, 0x04b0)
            };
            for field in ["FileDescription", "ProductName"] {
                let query: Vec<u16> =
                    format!("\\StringFileInfo\\{language:04x}{codepage:04x}\\{field}\0")
                        .encode_utf16()
                        .collect();
                let mut value = std::ptr::null_mut();
                let mut len = 0;
                if VerQueryValueW(
                    data.as_ptr().cast(),
                    PCWSTR(query.as_ptr()),
                    &mut value,
                    &mut len,
                )
                .as_bool()
                    && len > 1
                {
                    let raw = std::slice::from_raw_parts(value.cast::<u16>(), len as usize);
                    let end = raw
                        .iter()
                        .position(|character| *character == 0)
                        .unwrap_or(raw.len());
                    let text = String::from_utf16_lossy(&raw[..end]);
                    let text = text.trim();
                    if !text.is_empty() && !text.chars().any(char::is_control) {
                        return Some(text.to_owned());
                    }
                }
            }
            None
        }
    }

    /// Pairs a successful `CoInitializeEx` with `CoUninitialize`. A thread that
    /// already joined another apartment (`RPC_E_CHANGED_MODE`, e.g. the MTA
    /// audio worker) keeps its apartment and is not uninitialized here.
    struct ComScope(bool);

    impl ComScope {
        fn enter() -> Self {
            let result =
                unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
            // S_OK and S_FALSE both take a reference that must be released.
            Self(result.is_ok())
        }
    }

    impl Drop for ComScope {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }

    /// `canonicalize` yields `\\?\` verbatim paths, which the shell namespace
    /// parser rejects. Converts them back to the ordinary Win32 form.
    fn shell_parsing_name(path: &Path) -> Vec<u16> {
        let text = path.as_os_str().to_string_lossy();
        let plain = if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{unc}")
        } else if let Some(local) = text.strip_prefix(r"\\?\") {
            local.to_owned()
        } else {
            text.into_owned()
        };
        plain.encode_utf16().chain(Some(0)).collect()
    }

    pub fn extract_icon(path: &Path) -> Option<String> {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let parsing_name = shell_parsing_name(path);
        let high_resolution = {
            let _com = ComScope::enter();
            unsafe {
                SHCreateItemFromParsingName::<_, _, IShellItemImageFactory>(
                    PCWSTR(parsing_name.as_ptr()),
                    None::<&IBindCtx>,
                )
                .ok()
                .and_then(|factory| factory_to_data_url(&factory))
            }
        };
        high_resolution.or_else(|| legacy_icon(&wide))
    }

    /// Packaged apps expose their tile logo through the shell `AppsFolder`
    /// item named by the AUMID; their executables often carry no icon.
    pub fn extract_aumid_icon(aumid: &str) -> Option<String> {
        let wide: Vec<u16> = aumid.encode_utf16().chain(Some(0)).collect();
        let _com = ComScope::enter();
        unsafe {
            SHCreateItemInKnownFolder::<_, IShellItemImageFactory>(
                &FOLDERID_AppsFolder,
                KF_FLAG_DEFAULT,
                PCWSTR(wide.as_ptr()),
            )
            .ok()
            .and_then(|factory| factory_to_data_url(&factory))
        }
    }

    /// Owns the bitmap `IShellItemImageFactory::GetImage` hands to the caller.
    struct OwnedBitmap(HBITMAP);

    impl Drop for OwnedBitmap {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(self.0 .0));
            }
        }
    }

    unsafe fn factory_to_data_url(factory: &IShellItemImageFactory) -> Option<String> {
        let size = SIZE {
            cx: REQUEST_SIZE as i32,
            cy: REQUEST_SIZE as i32,
        };
        let bitmap = OwnedBitmap(
            factory
                .GetImage(size, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK)
                .ok()?,
        );
        let (width, height, bgra) = read_bitmap(bitmap.0)?;
        drop(bitmap);
        let (image, premultiplied) = shell_pixels_to_rgba(width, height, bgra, true)?;
        encode_png(finish_icon_pixels(image, premultiplied)?)
    }

    /// Crops and resizes while the pixels are still premultiplied, then
    /// returns straight alpha. `image::imageops::resize` assumes premultiplied
    /// input; resizing straight alpha blends the black RGB of transparent
    /// pixels into every soft edge and leaves a dark fringe.
    fn finish_icon_pixels(image: RgbaImage, premultiplied: bool) -> Option<RgbaImage> {
        let mut image = finish_icon(image)?;
        if premultiplied {
            unpremultiply(&mut image);
        }
        Some(image)
    }

    fn unpremultiply(image: &mut RgbaImage) {
        for pixel in image.pixels_mut() {
            let alpha = u32::from(pixel[3]);
            if alpha != 0 && alpha != 255 {
                for channel in &mut pixel.0[..3] {
                    *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
                }
            }
        }
    }

    /// Copies a 32bpp HBITMAP into a top-down BGRA buffer.
    unsafe fn read_bitmap(bitmap: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
        let mut header = BITMAP::default();
        let written = GetObjectW(
            HGDIOBJ(bitmap.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some((&mut header as *mut BITMAP).cast()),
        );
        if written == 0 || header.bmWidth <= 0 || header.bmHeight == 0 {
            return None;
        }
        let width = header.bmWidth as u32;
        let height = header.bmHeight.unsigned_abs();
        let raw_dc = CreateCompatibleDC(None);
        if raw_dc.is_invalid() {
            return None;
        }
        let dc = MemoryDc(raw_dc);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bgra = vec![0u8; width as usize * height as usize * 4];
        let lines = GetDIBits(
            dc.0,
            bitmap,
            0,
            height,
            Some(bgra.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        );
        drop(dc);
        (lines == height as i32).then_some((width, height, bgra))
    }

    /// Converts shell BGRA pixels to RGBA without changing the alpha model and
    /// reports whether the pixels are premultiplied. The shell returns
    /// premultiplied alpha for icons; the caller divides it back out after any
    /// resampling. With `opaque_without_alpha`, bitmaps that carry no alpha at
    /// all are treated as opaque.
    fn shell_pixels_to_rgba(
        width: u32,
        height: u32,
        mut bgra: Vec<u8>,
        opaque_without_alpha: bool,
    ) -> Option<(RgbaImage, bool)> {
        let has_alpha = bgra.as_chunks::<4>().0.iter().any(|pixel| pixel[3] != 0);
        let premultiplied =
            has_alpha
                && bgra.as_chunks::<4>().0.iter().all(|pixel| {
                    pixel[0] <= pixel[3] && pixel[1] <= pixel[3] && pixel[2] <= pixel[3]
                });
        for pixel in bgra.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
            if !has_alpha && opaque_without_alpha {
                pixel[3] = 255;
            }
        }
        Some((RgbaImage::from_raw(width, height, bgra)?, premultiplied))
    }

    fn legacy_icon(wide: &[u16]) -> Option<String> {
        unsafe {
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
        // DrawIconEx leaves alpha at zero for masked icons without an alpha
        // channel; keep the historical behavior for this fallback. A 32 px
        // bitmap is never resampled, so straight alpha can be produced here.
        let (mut image, premultiplied) = shell_pixels_to_rgba(32, 32, bgra, false)?;
        if premultiplied {
            unpremultiply(&mut image);
        }
        encode_png(image)
    }

    /// Normalizes a shell bitmap to at most `OUTPUT_SIZE` square pixels.
    ///
    /// When an executable only ships small icons, the shell centers them in the
    /// requested frame. Such content is cropped back to its own square so a
    /// 48 px icon is not stored as a mostly empty 256 px frame. Icons that fill
    /// most of the frame keep their designed padding. Content is never scaled up.
    fn finish_icon(image: RgbaImage) -> Option<RgbaImage> {
        let (width, height) = image.dimensions();
        let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
        for (x, y, pixel) in image.enumerate_pixels() {
            if pixel[3] != 0 {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x + 1);
                bottom = bottom.max(y + 1);
            }
        }
        if right <= left || bottom <= top {
            return None;
        }
        let frame = width.max(height);
        let content = (right - left).max(bottom - top);
        let image = if content * 4 <= frame * 3 {
            let side = content.min(width).min(height);
            let center_x = (left + right) / 2;
            let center_y = (top + bottom) / 2;
            let x = center_x.saturating_sub(side / 2).min(width - side);
            let y = center_y.saturating_sub(side / 2).min(height - side);
            image::imageops::crop_imm(&image, x, y, side, side).to_image()
        } else {
            image
        };
        let (width, height) = image.dimensions();
        if width.max(height) <= OUTPUT_SIZE {
            return Some(image);
        }
        let scale = f64::from(OUTPUT_SIZE) / f64::from(width.max(height));
        let target_width = ((f64::from(width) * scale).round() as u32).max(1);
        let target_height = ((f64::from(height) * scale).round() as u32).max(1);
        Some(image::imageops::resize(
            &image,
            target_width,
            target_height,
            FilterType::Lanczos3,
        ))
    }

    fn encode_png(image: RgbaImage) -> Option<String> {
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

    #[cfg(test)]
    mod pixel_tests {
        use super::{finish_icon_pixels, RgbaImage};

        /// A white disc with a one-pixel anti-aliased rim, stored premultiplied
        /// the way the shell returns it: rim pixels are white at half alpha,
        /// so their premultiplied RGB is 128.
        fn premultiplied_disc() -> RgbaImage {
            RgbaImage::from_fn(256, 256, |x, y| {
                let dx = f64::from(x) - 127.5;
                let dy = f64::from(y) - 127.5;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance < 110.0 {
                    image::Rgba([255, 255, 255, 255])
                } else if distance < 111.0 {
                    image::Rgba([128, 128, 128, 128])
                } else {
                    image::Rgba([0, 0, 0, 0])
                }
            })
        }

        #[test]
        fn downscaled_edges_keep_their_colour() {
            let image = finish_icon_pixels(premultiplied_disc(), true).expect("icon");
            assert_eq!(image.dimensions(), (128, 128));
            // Every visible pixel of a white icon must stay white once alpha is
            // straight. Resizing straight alpha darkens the rim towards grey.
            let darkest = image
                .pixels()
                .filter(|pixel| pixel[3] >= 16)
                .map(|pixel| pixel[0].min(pixel[1]).min(pixel[2]))
                .min()
                .expect("visible pixels");
            assert!(darkest >= 240, "edge darkened to {darkest}");
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::IconCache;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::path::{Path, PathBuf};

    fn system_executable(name: &str) -> PathBuf {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        Path::new(&root).join(name)
    }

    fn decode(url: &str) -> image::RgbaImage {
        let payload = url
            .strip_prefix("data:image/png;base64,")
            .expect("icon is a PNG data URL");
        let bytes = STANDARD.decode(payload).expect("valid base64");
        image::load_from_memory(&bytes)
            .expect("valid PNG")
            .to_rgba8()
    }

    fn assert_high_resolution(url: &str) {
        let image = decode(url);
        assert!(
            image.width() >= 96 && image.height() >= 96,
            "icon is {}x{}, expected at least 96x96",
            image.width(),
            image.height()
        );
        assert!(image.width() <= 128 && image.height() <= 128);
        assert!(image.pixels().any(|pixel| pixel[3] > 0));
    }

    #[test]
    fn executable_icon_is_high_resolution() {
        let icon = IconCache::default()
            .icon_for_path(&system_executable("explorer.exe"))
            .expect("explorer.exe has an icon");
        assert_high_resolution(&icon);
    }

    #[test]
    fn executable_icon_works_on_a_multithreaded_apartment() {
        // The audio worker joins the MTA before it resolves icons.
        std::thread::spawn(|| unsafe {
            use windows::Win32::System::Com::{
                CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED,
            };
            CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
            let icon = IconCache::default().icon_for_path(&system_executable("notepad.exe"));
            CoUninitialize();
            assert_high_resolution(&icon.expect("notepad.exe has an icon"));
        })
        .join()
        .unwrap();
    }
}
