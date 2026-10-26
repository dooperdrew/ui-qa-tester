//! Self-contained background Win32 test window (`SW_SHOWNOACTIVATE`) used to empirically verify
//! M0.2 (background window capture without focus stealing) and M0.3 (UIA inspection + background
//! `PostMessageW` input without moving the physical cursor or changing foreground window).

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, PostMessageW,
    PostQuitMessage, RegisterClassW, ShowWindow, TranslateMessage, CW_USEDEFAULT, HMENU, MSG,
    SW_SHOWNOACTIVATE, WINDOW_EX_STYLE, WM_CHAR, WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WNDCLASSW, WS_CHILD, WS_OVERLAPPEDWINDOW,
    WS_VISIBLE,
};

#[derive(Debug, Default)]
pub struct DummyWindowTelemetry {
    pub mouse_move_count: AtomicU32,
    pub lbutton_down_count: AtomicU32,
    pub lbutton_up_count: AtomicU32,
    pub char_count: AtomicU32,
    pub command_count: AtomicU32,
    pub last_click_x: AtomicU32,
    pub last_click_y: AtomicU32,
    pub closed: AtomicBool,
}

static mut GLOBAL_TELEMETRY: Option<*const DummyWindowTelemetry> = None;

pub struct DummyWindowHandle {
    pub hwnd_raw: isize,
    pub button_hwnd_raw: isize,
    pub telemetry: Arc<DummyWindowTelemetry>,
    join_handle: Option<JoinHandle<()>>,
}

impl DummyWindowHandle {
    pub fn spawn() -> anyhow::Result<Self> {
        let telemetry = Arc::new(DummyWindowTelemetry::default());
        let telemetry_clone = Arc::clone(&telemetry);
        let (tx, rx) = mpsc::channel::<anyhow::Result<(isize, isize)>>();

        let join_handle = thread::spawn(move || {
            // SAFETY: Storing a raw pointer to an Arc-backed struct that outlives the window thread.
            unsafe {
                GLOBAL_TELEMETRY = Some(Arc::as_ptr(&telemetry_clone));
            }

            // SAFETY: GetModuleHandleW(None) returns the current process module handle.
            let hinstance = unsafe { GetModuleHandleW(None) };
            let hinstance = match hinstance {
                Ok(h) => h,
                Err(e) => {
                    let _ = tx.send(Err(anyhow::anyhow!("GetModuleHandleW failed: {e}")));
                    return;
                }
            };

            let class_name: PCWSTR = w!("UiQaProbeDummyWindowClass");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(dummy_wnd_proc),
                hInstance: hinstance.into(),
                lpszClassName: class_name,
                ..Default::default()
            };

            // SAFETY: Registering a local window class with a valid static window procedure.
            unsafe {
                let _ = RegisterClassW(&wc);
            }

            // SAFETY: Creating an overlapped top-level window without activating it.
            let hwnd_res = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    class_name,
                    w!("UI-QA-Probe Background Target"),
                    WS_OVERLAPPEDWINDOW,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    640,
                    420,
                    None,
                    None,
                    hinstance,
                    None,
                )
            };

            let hwnd = match hwnd_res {
                Ok(h) => h,
                Err(e) => {
                    let _ = tx.send(Err(anyhow::anyhow!("CreateWindowExW failed: {e}")));
                    return;
                }
            };

            // Create a native child BUTTON so IUIAutomation discovers an interactive control
            // SAFETY: Valid parent `hwnd` and `hinstance` on the owning GUI thread.
            let btn_hwnd = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("BUTTON"),
                    w!("Calibrate Sensor"),
                    WS_CHILD | WS_VISIBLE,
                    40,
                    60,
                    180,
                    44,
                    hwnd,
                    HMENU(101 as *mut _),
                    hinstance,
                    None,
                )
                .unwrap_or_default()
            };

            // Show the window in the background WITHOUT stealing foreground focus (SW_SHOWNOACTIVATE)
            // SAFETY: `hwnd` is a valid window handle created on this thread.
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }

            let _ = tx.send(Ok((hwnd.0 as isize, btn_hwnd.0 as isize)));

            let mut msg = MSG::default();
            // SAFETY: Standard Win32 thread message loop for windows created on this thread.
            unsafe {
                while GetMessageW(&mut msg, None, 0, 0).into() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            telemetry_clone.closed.store(true, Ordering::SeqCst);
        });

        let (hwnd_raw, button_hwnd_raw) = rx
            .recv()
            .map_err(|e| anyhow::anyhow!("Window thread channel closed: {e}"))??;

        Ok(Self {
            hwnd_raw,
            button_hwnd_raw,
            telemetry,
            join_handle: Some(join_handle),
        })
    }

    pub fn close(&mut self) {
        if !self.telemetry.closed.load(Ordering::SeqCst) {
            let hwnd = HWND(self.hwnd_raw as *mut _);
            // SAFETY: Posting WM_CLOSE to the thread-owned window is thread-safe in Win32.
            unsafe {
                let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for DummyWindowHandle {
    fn drop(&mut self) {
        self.close();
    }
}

// SAFETY: Win32 callback invoked by DispatchMessageW on the window thread.
unsafe extern "system" fn dummy_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let telemetry_opt = GLOBAL_TELEMETRY.and_then(|ptr| ptr.as_ref());

    match msg {
        WM_MOUSEMOVE => {
            if let Some(t) = telemetry_opt {
                t.mouse_move_count.fetch_add(1, Ordering::SeqCst);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONDOWN => {
            if let Some(t) = telemetry_opt {
                t.lbutton_down_count.fetch_add(1, Ordering::SeqCst);
                let x = (lparam.0 as u32) & 0xFFFF;
                let y = ((lparam.0 as u32) >> 16) & 0xFFFF;
                t.last_click_x.store(x, Ordering::SeqCst);
                t.last_click_y.store(y, Ordering::SeqCst);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONUP => {
            if let Some(t) = telemetry_opt {
                t.lbutton_up_count.fetch_add(1, Ordering::SeqCst);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_KEYDOWN | WM_CHAR => {
            if msg == WM_CHAR {
                if let Some(t) = telemetry_opt {
                    t.char_count.fetch_add(1, Ordering::SeqCst);
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_COMMAND => {
            if let Some(t) = telemetry_opt {
                t.command_count.fetch_add(1, Ordering::SeqCst);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let bg_brush = CreateSolidBrush(COLORREF(0x0038_241E)); // Distinct dark slate blue/teal
            let accent_brush = CreateSolidBrush(COLORREF(0x0040_A028)); // Distinct green bar
            let client_rect = RECT {
                left: 0,
                top: 0,
                right: 640,
                bottom: 420,
            };
            let accent_rect = RECT {
                left: 20,
                top: 140,
                right: 600,
                bottom: 260,
            };
            let _ = FillRect(hdc, &client_rect, bg_brush);
            let _ = FillRect(hdc, &accent_rect, accent_brush);
            let _ = DeleteObject(bg_brush);
            let _ = DeleteObject(accent_brush);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
