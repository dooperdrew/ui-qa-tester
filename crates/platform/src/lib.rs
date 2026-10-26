//! Windows platform integration (`crates/platform`):
//! - `process`: target `.exe` launcher, real-time `stdout`/`stderr` & file log tailer, `IsHungAppWindow` watchdog
//! - `capture`: background `PrintWindow(PW_RENDERFULLCONTENT)` + `BitBlt` window capture
//! - `actuator`: `IUIAutomation` (AccessKit) inspector, window enumerator, and non-intrusive input dispatcher
//! - `selftest_window`: built-in background Win32 hardware-simulator window for end-to-end self-testing

pub mod actuator;
pub mod capture;
pub mod process;
pub mod selftest_window;

pub fn platform_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_enumerates_windows_and_runs_selftest_window() {
        assert!(platform_ready());
        let mut win = selftest_window::SelfTestWindow::spawn().expect("spawn selftest window");
        std::thread::sleep(std::time::Duration::from_millis(80));

        let cap =
            capture::capture_hwnd_background(win.hwnd_raw).expect("background capture succeeds");
        assert!(cap.image.width() >= 600);
        assert!(cap.latency_ms < 250.0);

        let elements = actuator::inspect_window_uia_elements(win.hwnd_raw)
            .expect("inspect UIA elements succeeds");
        assert!(!elements.is_empty());

        win.close();
    }
}
