//! Built-in background Win32 hardware-simulator window (`SW_SHOWNOACTIVATE`) used by
//! `ui-qa-cli self-test`, the Live QA Dashboard demo mode, and automated integration tests.

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
    SW_SHOWNOACTIVATE, WINDOW_EX_STYLE, WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_PAINT, WNDCLASSW, WS_CHILD, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

#[derive(Debug, Default)]
pub struct SelfTestState {
    pub clicks_received: AtomicU32,
    pub closed: AtomicBool,
}

pub struct SelfTestWindow {
    pub hwnd_raw: isize,
    pub state: Arc<SelfTestState>,
    join_handle: Option<JoinHandle<()>>,
}

impl SelfTestWindow {
    pub fn spawn() -> Result<Self, String> {
        let state = Arc::new(SelfTestState::default());
        let state_clone = Arc::clone(&state);
        let (tx, rx) = mpsc::channel::<Result<isize, String>>();

        let join_handle = thread::spawn(move || {
            // SAFETY: GetModuleHandleW(None) retrieves the current process module handle.
            let hinstance = match unsafe { GetModuleHandleW(None) } {
                Ok(h) => h,
                Err(e) => {
                    let _ = tx.send(Err(format!("GetModuleHandleW failed: {e}")));
                    return;
                }
            };

            let class_name: PCWSTR = w!("UiQaSelfTestHardwareWindowClass");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(selftest_wnd_proc),
                hInstance: hinstance.into(),
                lpszClassName: class_name,
                ..Default::default()
            };

            // SAFETY: Registering window class on local GUI thread.
            unsafe {
                let _ = RegisterClassW(&wc);
            }

            // SAFETY: Creating top-level overlapped window for background capture & UIA inspection.
            let hwnd = match unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    class_name,
                    w!("Hardware Controller Demo (Target App)"),
                    WS_OVERLAPPEDWINDOW,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    680,
                    440,
                    None,
                    None,
                    hinstance,
                    None,
                )
            } {
                Ok(h) => h,
                Err(e) => {
                    let _ = tx.send(Err(format!("CreateWindowExW failed: {e}")));
                    return;
                }
            };

            // SAFETY: Creating child BUTTON controls on the owning window thread.
            unsafe {
                let _ = CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("BUTTON"),
                    w!("Connect COM3"),
                    WS_CHILD | WS_VISIBLE,
                    32,
                    40,
                    180,
                    42,
                    hwnd,
                    HMENU(201 as *mut _),
                    hinstance,
                    None,
                );
                let _ = CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("BUTTON"),
                    w!("Calibrate Sensor"),
                    WS_CHILD | WS_VISIBLE,
                    236,
                    40,
                    180,
                    42,
                    hwnd,
                    HMENU(202 as *mut _),
                    hinstance,
                    None,
                );
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }

            let _ = tx.send(Ok(hwnd.0 as isize));

            let mut msg = MSG::default();
            // SAFETY: Running Win32 message loop for windows created on this thread.
            unsafe {
                while GetMessageW(&mut msg, None, 0, 0).into() {
                    if msg.message == WM_LBUTTONDOWN || msg.message == WM_COMMAND {
                        state_clone.clicks_received.fetch_add(1, Ordering::SeqCst);
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            state_clone.closed.store(true, Ordering::SeqCst);
        });

        let hwnd_raw = rx
            .recv()
            .map_err(|e| format!("SelfTestWindow channel closed: {e}"))??;

        Ok(Self {
            hwnd_raw,
            state,
            join_handle: Some(join_handle),
        })
    }

    pub fn close(&mut self) {
        if !self.state.closed.load(Ordering::SeqCst) {
            let hwnd = HWND(self.hwnd_raw as *mut _);
            // SAFETY: Posting WM_CLOSE to close the background window cleanly.
            unsafe {
                let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(h) = self.join_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for SelfTestWindow {
    fn drop(&mut self) {
        self.close();
    }
}

// SAFETY: Standard Win32 window procedure called by DispatchMessageW on the window thread.
unsafe extern "system" fn selftest_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let bg = CreateSolidBrush(COLORREF(0x002B_1E16));
            let panel = CreateSolidBrush(COLORREF(0x004A_3525));
            let status_bar = CreateSolidBrush(COLORREF(0x0050_B428));
            let r_bg = RECT {
                left: 0,
                top: 0,
                right: 680,
                bottom: 440,
            };
            let r_panel = RECT {
                left: 24,
                top: 110,
                right: 632,
                bottom: 340,
            };
            let r_status = RECT {
                left: 24,
                top: 356,
                right: 632,
                bottom: 392,
            };
            let _ = FillRect(hdc, &r_bg, bg);
            let _ = FillRect(hdc, &r_panel, panel);
            let _ = FillRect(hdc, &r_status, status_bar);
            let _ = DeleteObject(bg);
            let _ = DeleteObject(panel);
            let _ = DeleteObject(status_bar);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_LBUTTONDOWN | WM_LBUTTONUP | WM_COMMAND => DefWindowProcW(hwnd, msg, wparam, lparam),
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
