//! M0.2 Spike — Background Window Capture probe comparing `PrintWindow(PW_RENDERFULLCONTENT)`
//! and `Windows.Graphics.Capture` (WGC) availability on occluded / background windows.

use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use windows::Graphics::Capture::GraphicsCaptureSession;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetForegroundWindow, GetWindowRect, IsIconic, ShowWindow, SW_SHOWNOACTIVATE,
};

const PW_RENDERFULLCONTENT: PRINT_WINDOW_FLAGS = PRINT_WINDOW_FLAGS(0x00000002);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureProbeReport {
    pub hwnd: isize,
    pub width: u32,
    pub height: u32,
    pub wgc_supported_by_os: bool,
    pub wgc_item_created_for_hwnd: bool,
    pub print_window_success: bool,
    pub capture_latency_ms_p50: f64,
    pub capture_latency_ms_p95: f64,
    pub non_black_pixel_ratio: f64,
    pub focus_unchanged_during_capture: bool,
}

/// Checks whether `Windows.Graphics.Capture` can bind to the target `HWND` via `IGraphicsCaptureItemInterop`.
pub fn check_wgc_interop(hwnd_raw: isize) -> (bool, bool) {
    // SAFETY: RoInitialize initializes the Windows Runtime on the current thread; RPC_E_CHANGED_MODE is benign.
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    // SAFETY: GraphicsCaptureSession::IsSupported() is a safe WinRT static query.
    let supported = GraphicsCaptureSession::IsSupported().unwrap_or(false);
    if !supported {
        return (false, false);
    }

    let hwnd = HWND(hwnd_raw as *mut _);
    // SAFETY: RoGetActivationFactory for IGraphicsCaptureItemInterop is a standard WinRT interop call;
    // CreateForWindow reads the HWND without mutating window focus or thread state.
    let item_ok = unsafe {
        windows::core::factory::<
            windows::Graphics::Capture::GraphicsCaptureItem,
            IGraphicsCaptureItemInterop,
        >()
        .and_then(|interop| {
            let _item: windows::Graphics::Capture::GraphicsCaptureItem =
                interop.CreateForWindow(hwnd)?;
            Ok(())
        })
        .is_ok()
    };

    (supported, item_ok)
}

/// Captures a window in the background using `PrintWindow(PW_RENDERFULLCONTENT)` with `BitBlt` fallback.
/// Never activates or brings the target window to the foreground.
pub fn capture_window_background(hwnd_raw: isize) -> anyhow::Result<(RgbaImage, bool)> {
    let hwnd = HWND(hwnd_raw as *mut _);

    // Ensure the window is not minimized (DWM suspends surfaces for minimized windows)
    // SAFETY: IsIconic and ShowWindow(SW_SHOWNOACTIVATE) inspect/restore without focus stealing.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
    }

    let mut rect = RECT::default();
    // SAFETY: GetWindowRect reads window bounds into stack-allocated RECT.
    unsafe {
        if GetWindowRect(hwnd, &mut rect).is_err() {
            GetClientRect(hwnd, &mut rect)?;
        }
    }

    let width = (rect.right - rect.left).max(1) as u32;
    let height = (rect.bottom - rect.top).max(1) as u32;

    // SAFETY: Acquiring window DC, creating compatible memory DC and bitmap, calling PrintWindow
    // with PW_RENDERFULLCONTENT (0x2) so DWM renders GPU/DirectComposition content even when occluded,
    // extracting BGRA pixels via GetDIBits, and releasing all GDI handles before return.
    unsafe {
        let hdc_window = GetDC(hwnd);
        if hdc_window.is_invalid() {
            anyhow::bail!("GetDC returned invalid HDC for HWND {hwnd_raw}");
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
                // Negative height requests top-down DIB so row 0 is the top of the window
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
            anyhow::bail!("GetDIBits returned 0 scanlines for HWND {hwnd_raw}");
        }

        // Convert BGRA to RGBA
        for px in bgra_buf.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
            px[3] = 255;
        }

        let img = RgbaImage::from_raw(width, height, bgra_buf)
            .ok_or_else(|| anyhow::anyhow!("Failed to construct RgbaImage"))?;
        Ok((img, pw_ok))
    }
}

pub fn run_capture_benchmark(
    hwnd_raw: isize,
    iterations: usize,
) -> anyhow::Result<CaptureProbeReport> {
    let (wgc_supported, wgc_item_ok) = check_wgc_interop(hwnd_raw);

    // SAFETY: GetForegroundWindow has no preconditions.
    let fg_before = unsafe { GetForegroundWindow().0 as isize };

    let mut latencies_ms = Vec::with_capacity(iterations.max(1));
    let mut last_img: Option<RgbaImage> = None;
    let mut pw_success = false;

    for _ in 0..iterations.max(1) {
        let start = Instant::now();
        let (img, ok) = capture_window_background(hwnd_raw)?;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        latencies_ms.push(elapsed_ms);
        pw_success = ok;
        last_img = Some(img);
    }

    // SAFETY: GetForegroundWindow has no preconditions.
    let fg_after = unsafe { GetForegroundWindow().0 as isize };

    latencies_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p50 = latencies_ms[latencies_ms.len() / 2];
    let p95_idx = ((latencies_ms.len() as f64 * 0.95).floor() as usize).min(latencies_ms.len() - 1);
    let p95 = latencies_ms[p95_idx];

    let img = last_img.expect("at least one iteration executed");
    let total_pixels = (img.width() * img.height()).max(1) as usize;
    let non_black_pixels = img
        .pixels()
        .filter(|p| p.0[0] > 5 || p.0[1] > 5 || p.0[2] > 5)
        .count();
    let non_black_ratio = non_black_pixels as f64 / total_pixels as f64;

    Ok(CaptureProbeReport {
        hwnd: hwnd_raw,
        width: img.width(),
        height: img.height(),
        wgc_supported_by_os: wgc_supported,
        wgc_item_created_for_hwnd: wgc_item_ok,
        print_window_success: pw_success,
        capture_latency_ms_p50: (p50 * 100.0).round() / 100.0,
        capture_latency_ms_p95: (p95 * 100.0).round() / 100.0,
        non_black_pixel_ratio: (non_black_ratio * 1000.0).round() / 1000.0,
        focus_unchanged_during_capture: fg_before == fg_after,
    })
}
