//! Production background window capture engine (`M2.2`) built on the M0-verified
//! `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)` pipeline with `BitBlt` fallback.

use image::RgbaImage;
use std::time::Instant;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowRect, IsIconic, ShowWindow, SW_SHOWNOACTIVATE,
};

const PW_RENDERFULLCONTENT: PRINT_WINDOW_FLAGS = PRINT_WINDOW_FLAGS(0x00000002);

#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub image: RgbaImage,
    pub latency_ms: f64,
    pub used_print_window: bool,
}

/// Captures `hwnd_raw` in the background without stealing focus.
/// Automatically restores minimized (`IsIconic`) windows using `SW_SHOWNOACTIVATE`.
pub fn capture_hwnd_background(hwnd_raw: isize) -> Result<CapturedFrame, String> {
    let start = Instant::now();
    let hwnd = HWND(hwnd_raw as *mut _);

    // SAFETY: IsIconic and ShowWindow(SW_SHOWNOACTIVATE) inspect/restore without focus stealing.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
    }

    let mut rect = RECT::default();
    // SAFETY: GetWindowRect / GetClientRect populate a stack-allocated RECT for valid HWND.
    unsafe {
        if GetWindowRect(hwnd, &mut rect).is_err() {
            GetClientRect(hwnd, &mut rect).map_err(|e| format!("GetClientRect failed: {e}"))?;
        }
    }

    let width = (rect.right - rect.left).max(1) as u32;
    let height = (rect.bottom - rect.top).max(1) as u32;

    // SAFETY: Acquiring window DC, rendering via PrintWindow(PW_RENDERFULLCONTENT) into a memory DC,
    // copying scanlines with GetDIBits, and releasing all GDI objects before return.
    unsafe {
        let hdc_window = GetDC(hwnd);
        if hdc_window.is_invalid() {
            return Err(format!("GetDC returned invalid HDC for HWND {hwnd_raw}"));
        }

        let hdc_mem = CreateCompatibleDC(hdc_window);
        let hbitmap = CreateCompatibleBitmap(hdc_window, width as i32, height as i32);
        let old_obj = SelectObject(hdc_mem, hbitmap);

        let pw_ok = PrintWindow(hwnd, hdc_mem, PW_RENDERFULLCONTENT).as_bool();
        if !pw_ok {
            let _ = BitBlt(
                hdc_mem,
                0,
                0,
                width as i32,
                height as i32,
                hdc_window,
                0,
                0,
                SRCCOPY,
            );
        }

        let mut bmi = BITMAPINFO {
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

        let mut bgra_buf = vec![0u8; (width * height * 4) as usize];
        let lines = GetDIBits(
            hdc_mem,
            hbitmap,
            0,
            height,
            Some(bgra_buf.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        let _ = SelectObject(hdc_mem, old_obj);
        let _ = DeleteObject(hbitmap);
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(hwnd, hdc_window);

        if lines == 0 {
            return Err(format!("GetDIBits returned 0 lines for HWND {hwnd_raw}"));
        }

        for px in bgra_buf.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
            px[3] = 255;
        }

        let image = RgbaImage::from_raw(width, height, bgra_buf)
            .ok_or_else(|| "Failed to construct RgbaImage".to_string())?;
        let latency_ms = (start.elapsed().as_secs_f64() * 1000.0 * 100.0).round() / 100.0;

        Ok(CapturedFrame {
            image,
            latency_ms,
            used_print_window: pw_ok,
        })
    }
}
