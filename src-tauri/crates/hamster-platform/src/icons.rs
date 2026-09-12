//! exe/dll/lnk 目标图标提取：优先 SHIL_JUMBO 系统图像列表（256×256，
//! 任务栏/应用页高分辨率显示），无 jumbo 资源的目标回退 SHGFI_LARGEICON（32×32）。
//! HICON → GetDIBits RGBA → PNG

use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC,
    SelectObject, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
};
use windows::Win32::UI::Controls::IImageList;
use windows::Win32::UI::Shell::{
    SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_SYSICONINDEX,
    SHIL_JUMBO,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

const ILD_TRANSPARENT: u32 = 0x00000001;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 提取资源图标并保存为 PNG。成功返回 true（失败由上层用字母占位图标兜底）。
pub fn extract_icon_png(source: &Path, out_png: &Path) -> bool {
    let wide = to_wide(&source.to_string_lossy());
    unsafe {
        // ① 系统图像列表 jumbo（256px）：按 sysiconindex 取，多数现代应用有高分辨率资源
        let mut shfi = SHFILEINFOW::default();
        let ok = SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            Default::default(),
            Some(&mut shfi),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX,
        );
        if ok != 0 && shfi.iIcon >= 0 {
            if let Ok(list) = SHGetImageList::<IImageList>(SHIL_JUMBO as i32) {
                if let Ok(hicon) = list.GetIcon(shfi.iIcon, ILD_TRANSPARENT) {
                    if !hicon.is_invalid() {
                        let result = hicon_to_png(hicon, out_png);
                        let _ = DestroyIcon(hicon);
                        // jumbo 位图尺寸过小（目标无 256 资源时列表会给 32px 缩放）也接受，
                        // 仍显著优于纯 LARGEICON；失败才走回退
                        if result {
                            return true;
                        }
                    }
                }
            }
        }

        // ② 回退：SHGFI_LARGEICON（32px）
        let mut shfi = SHFILEINFOW::default();
        let ok = SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            Default::default(),
            Some(&mut shfi),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if ok == 0 || shfi.hIcon.is_invalid() {
            return false;
        }
        let result = hicon_to_png(shfi.hIcon, out_png);
        let _ = DestroyIcon(shfi.hIcon);
        result
    }
}

/// HICON → PNG 字节（内存版，托盘图标提取用；跨进程 HICON 可直接传）
///
/// # Safety
/// `hicon` 必须是有效图标句柄（跨进程 GDI 图标对象可直接使用）
pub unsafe fn hicon_png_bytes(hicon: HICON) -> Option<Vec<u8>> {
    let mut ii = ICONINFO::default();
    if GetIconInfo(hicon, &mut ii).is_err() {
        return None;
    }

    let mut bmp = BITMAP::default();
    if GetObjectW(
        ii.hbmColor.into(),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bmp as *mut BITMAP as *mut core::ffi::c_void),
    ) == 0
    {
        cleanup_iconinfo(&ii);
        return None;
    }
    let w = bmp.bmWidth.unsigned_abs();
    let h = bmp.bmHeight.unsigned_abs();
    if w == 0 || h == 0 || w > 256 || h > 256 {
        cleanup_iconinfo(&ii);
        return None;
    }

    let hdc = GetDC(None);
    let memdc = CreateCompatibleDC(Some(hdc));

    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = w as i32;
    bmi.bmiHeader.biHeight = -(h as i32);
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;

    let mut buf = vec![0u8; (w * h * 4) as usize];
    let old = SelectObject(memdc, ii.hbmColor.into());
    let got = GetDIBits(
        memdc,
        ii.hbmColor,
        0,
        h,
        Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    SelectObject(memdc, old);
    let _ = DeleteDC(memdc);
    ReleaseDC(None, hdc);
    cleanup_iconinfo(&ii);
    if got == 0 {
        return None;
    }

    // BGRA → RGBA
    for px in buf.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
    if !buf.as_chunks::<4>().0.iter().any(|p| p[3] != 0) {
        for px in buf.as_chunks_mut::<4>().0 {
            px[3] = 255;
        }
    }

    let mut out = Vec::new();
    image::RgbaImage::from_raw(w, h, buf)?
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .ok()?;
    Some(out)
}

unsafe fn hicon_to_png(hicon: HICON, out_png: &Path) -> bool {
    let mut ii = ICONINFO::default();
    if GetIconInfo(hicon, &mut ii).is_err() {
        return false;
    }

    let mut bmp = BITMAP::default();
    if GetObjectW(
        ii.hbmColor.into(),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bmp as *mut BITMAP as *mut core::ffi::c_void),
    ) == 0
    {
        cleanup_iconinfo(&ii);
        return false;
    }
    let w = bmp.bmWidth.unsigned_abs();
    let h = bmp.bmHeight.unsigned_abs();
    if w == 0 || h == 0 || w > 512 || h > 512 {
        cleanup_iconinfo(&ii);
        return false;
    }

    let hdc = GetDC(None);
    let memdc = CreateCompatibleDC(Some(hdc));

    // biCompression 保持 0(BI_RGB)；biHeight 取负 = 自上而下扫描行
    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = w as i32;
    bmi.bmiHeader.biHeight = -(h as i32);
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;

    let mut buf = vec![0u8; (w * h * 4) as usize];
    let old = SelectObject(memdc, ii.hbmColor.into());
    let got = GetDIBits(
        memdc,
        ii.hbmColor,
        0,
        h,
        Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    SelectObject(memdc, old);
    let _ = DeleteDC(memdc);
    ReleaseDC(None, hdc);
    cleanup_iconinfo(&ii);
    if got == 0 {
        return false;
    }

    // BGRA → RGBA
    for px in buf.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
    // 部分旧格式图标无 alpha 通道（全 0），视为不透明
    if !buf.as_chunks::<4>().0.iter().any(|p| p[3] != 0) {
        for px in buf.as_chunks_mut::<4>().0 {
            px[3] = 255;
        }
    }

    // SHIL_JUMBO 对没有 256px 资源的目标，返回的是 256×256 透明画布 + 32px 小图标
    // 原尺寸画在左上角（不缩放），前端缩小渲染后几乎不可见；内容全贴左上角的
    // 视为提取失败，回退 SHGFI_LARGEICON
    if w >= 96 {
        let mut max_x = 0usize;
        let mut max_y = 0usize;
        for (i, px) in buf.as_chunks::<4>().0.iter().enumerate() {
            if px[3] != 0 {
                max_x = max_x.max(i % w as usize);
                max_y = max_y.max(i / w as usize);
            }
        }
        if max_x <= 64 && max_y <= 64 {
            return false;
        }
    }

    match image::RgbaImage::from_raw(w, h, buf) {
        Some(img) => img.save(out_png).is_ok(),
        None => false,
    }
}

unsafe fn cleanup_iconinfo(ii: &ICONINFO) {
    if !ii.hbmColor.is_invalid() {
        let _ = DeleteObject(ii.hbmColor.into());
    }
    if !ii.hbmMask.is_invalid() {
        let _ = DeleteObject(ii.hbmMask.into());
    }
}
