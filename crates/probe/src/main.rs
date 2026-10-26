//! Milestone 0 diagnostic probe (`ui-qa-probe`) for background window capture (`M0.2`)
//! and non-intrusive AccessKit / `IUIAutomation` + `PostMessageW` actuation (`M0.3`).

mod capture_probe;
mod dummy_window;
mod input_probe;

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

#[derive(Debug, Parser)]
#[command(
    name = "ui-qa-probe",
    about = "M0 diagnostic probe for background window capture and non-intrusive UIA/HWND input"
)]
struct Cli {
    /// Run self-contained M0 spike verification against a background dummy window
    #[arg(long, default_value_t = false)]
    self_test: bool,

    /// Target an existing HWND (decimal or hex)
    #[arg(long)]
    hwnd: Option<isize>,

    /// Optional output path for captured PNG frame
    #[arg(long)]
    save_png: Option<PathBuf>,

    /// Number of capture iterations for p50/p95 benchmark
    #[arg(long, default_value_t = 10)]
    iterations: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct FullProbeResult {
    pub version: String,
    pub capture: capture_probe::CaptureProbeReport,
    pub input: input_probe::InputProbeReport,
    pub background_events_received_by_window: Option<DummyTelemetrySummary>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DummyTelemetrySummary {
    pub mouse_move_count: u32,
    pub lbutton_down_count: u32,
    pub lbutton_up_count: u32,
    pub char_count: u32,
    pub command_count: u32,
    pub last_click_xy: [u32; 2],
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.self_test || cli.hwnd.is_none() {
        let mut dummy = dummy_window::DummyWindowHandle::spawn()?;
        // Give the window message loop a brief moment to complete initial paint in background
        thread::sleep(Duration::from_millis(80));

        let capture_report = capture_probe::run_capture_benchmark(dummy.hwnd_raw, cli.iterations)?;

        if let Some(ref png_path) = cli.save_png {
            let (img, _) = capture_probe::capture_window_background(dummy.hwnd_raw)?;
            img.save(png_path)?;
        }

        let input_report = input_probe::inspect_and_test_background_input(
            dummy.hwnd_raw,
            Some(dummy.button_hwnd_raw),
            (120, 80),
        )?;

        // Wait briefly for queued PostMessageW events to be processed by the window loop
        thread::sleep(Duration::from_millis(50));

        let telemetry_summary = DummyTelemetrySummary {
            mouse_move_count: dummy.telemetry.mouse_move_count.load(Ordering::SeqCst),
            lbutton_down_count: dummy.telemetry.lbutton_down_count.load(Ordering::SeqCst),
            lbutton_up_count: dummy.telemetry.lbutton_up_count.load(Ordering::SeqCst),
            char_count: dummy.telemetry.char_count.load(Ordering::SeqCst),
            command_count: dummy.telemetry.command_count.load(Ordering::SeqCst),
            last_click_xy: [
                dummy.telemetry.last_click_x.load(Ordering::SeqCst),
                dummy.telemetry.last_click_y.load(Ordering::SeqCst),
            ],
        };

        dummy.close();

        let full = FullProbeResult {
            version: ui_qa_core::workspace_version().to_string(),
            capture: capture_report,
            input: input_report,
            background_events_received_by_window: Some(telemetry_summary),
        };

        println!("{}", serde_json::to_string_pretty(&full)?);
        return Ok(());
    }

    let hwnd_raw = cli.hwnd.expect("checked above");
    let capture_report = capture_probe::run_capture_benchmark(hwnd_raw, cli.iterations)?;
    if let Some(ref png_path) = cli.save_png {
        let (img, _) = capture_probe::capture_window_background(hwnd_raw)?;
        img.save(png_path)?;
    }
    let input_report = input_probe::inspect_and_test_background_input(hwnd_raw, None, (100, 100))?;

    let full = FullProbeResult {
        version: ui_qa_core::workspace_version().to_string(),
        capture: capture_report,
        input: input_report,
        background_events_received_by_window: None,
    };
    println!("{}", serde_json::to_string_pretty(&full)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_test_background_capture_and_input_probe_passes() {
        let mut dummy = dummy_window::DummyWindowHandle::spawn().expect("spawn dummy window");
        thread::sleep(Duration::from_millis(80));

        let cap =
            capture_probe::run_capture_benchmark(dummy.hwnd_raw, 3).expect("capture benchmark");
        assert!(cap.width >= 600);
        assert!(cap.height >= 400);
        assert!(cap.non_black_pixel_ratio > 0.5);
        assert!(cap.focus_unchanged_during_capture);

        let inp = input_probe::inspect_and_test_background_input(
            dummy.hwnd_raw,
            Some(dummy.button_hwnd_raw),
            (120, 80),
        )
        .expect("input probe");
        thread::sleep(Duration::from_millis(50));

        assert!(!inp.discovered_elements.is_empty());
        assert!(inp.uia_invoke_succeeded);
        assert!(inp.post_message_click_sent);
        assert!(inp.post_message_char_sent);
        assert!(inp.cursor_position_unchanged);
        assert!(inp.foreground_window_unchanged);

        assert!(dummy.telemetry.mouse_move_count.load(Ordering::SeqCst) >= 1);
        assert!(dummy.telemetry.lbutton_down_count.load(Ordering::SeqCst) >= 1);
        assert!(dummy.telemetry.lbutton_up_count.load(Ordering::SeqCst) >= 1);
        assert!(dummy.telemetry.char_count.load(Ordering::SeqCst) >= 1);
        assert!(dummy.telemetry.command_count.load(Ordering::SeqCst) >= 1);

        dummy.close();
    }
}
