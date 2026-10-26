//! Milestone 5 Hardening, NFR Budget Verification (`M5.1`), and Soak/Edge-Case Tests (`M5.2`).

use std::time::Instant;
use ui_qa_ai::UnifiedVisionClient;
use ui_qa_core::config::{AiProviderConfig, HardwareTelemetryConfig, QaSessionConfig};
use ui_qa_core::correlator::HardwareLogCorrelator;
use ui_qa_core::model::{LogStreamKind, TimestampedLogLine};
use ui_qa_core::vision_heuristics::analyze_screen_heuristics;
use ui_qa_platform::actuator::inspect_window_uia_elements;
use ui_qa_platform::capture::capture_hwnd_background;
use ui_qa_platform::orchestrator::run_qa_session;
use ui_qa_platform::process::query_process_working_set;
use ui_qa_platform::selftest_window::SelfTestWindow;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, ShowWindow, SW_MINIMIZE};

#[test]
fn m5_nfr_budgets_and_edge_case_soak_suite() {
    let mut win = SelfTestWindow::spawn().expect("spawn background self-test window");
    std::thread::sleep(std::time::Duration::from_millis(80));

    // 1. NFR-P1: Background capture latency p95 <= 50 ms
    let mut cap_latencies = Vec::new();
    let mut last_frame = None;
    for _ in 0..10 {
        let cap = capture_hwnd_background(win.hwnd_raw).expect("capture background");
        cap_latencies.push(cap.latency_ms);
        last_frame = Some(cap.image);
    }
    cap_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p95_cap = cap_latencies[(cap_latencies.len() * 95 / 100).min(cap_latencies.len() - 1)];
    assert!(
        p95_cap <= 50.0,
        "NFR-P1 budget violated: p95 capture latency {p95_cap:.2} ms > 50 ms"
    );

    // 2. NFR-P2: UIA tree extraction + heuristic geometry check <= 40 ms
    let frame_ref = last_frame.as_ref().expect("frame");
    let start_uia = Instant::now();
    let elements = inspect_window_uia_elements(win.hwnd_raw).expect("UIA elements");
    let _heuristics = analyze_screen_heuristics(
        0,
        frame_ref.width(),
        frame_ref.height(),
        &elements,
        Some(frame_ref),
    );
    let uia_plus_heuristics_ms = start_uia.elapsed().as_secs_f64() * 1000.0;
    assert!(
        uia_plus_heuristics_ms <= 40.0,
        "NFR-P2 budget violated: UIA + heuristics took {uia_plus_heuristics_ms:.2} ms > 40 ms"
    );

    // 3. NFR-P3 & M5.2 Soak: 10,000 log lines flood correlation
    let correlator =
        HardwareLogCorrelator::new(&HardwareTelemetryConfig::default()).expect("correlator");
    let flood_logs: Vec<TimestampedLogLine> = (0..10_000)
        .map(|i| TimestampedLogLine {
            timestamp_ms: i,
            stream: LogStreamKind::Stdout,
            text: format!("[SOAK] Sensor telemetry packet #{i} ACK v=3.30V"),
        })
        .collect();
    let start_corr = Instant::now();
    let outcome = correlator.correlate_action(1, None, 5000, &flood_logs, true, None);
    let corr_ms = start_corr.elapsed().as_secs_f64() * 1000.0;
    assert!(!outcome.ack_lines.is_empty());
    assert!(
        corr_ms < 50.0,
        "10,000-line log flood correlation too slow: {corr_ms:.2} ms"
    );

    // 4. QA-SYS-04 Edge Case: Minimized target window restoration without focus stealing
    // SAFETY: GetForegroundWindow and ShowWindow(SW_MINIMIZE) on our owned test HWND.
    let fg_before = unsafe {
        let fg = GetForegroundWindow().0 as isize;
        let _ = ShowWindow(HWND(win.hwnd_raw as *mut _), SW_MINIMIZE);
        fg
    };
    let restored_cap =
        capture_hwnd_background(win.hwnd_raw).expect("capture restores minimized window");
    // SAFETY: Reading foreground window after SW_SHOWNOACTIVATE restore.
    let fg_after = unsafe { GetForegroundWindow().0 as isize };
    assert!(restored_cap.image.width() >= 600);
    assert_eq!(
        fg_before, fg_after,
        "QA-SYS-04 violated: restoring minimized window changed foreground focus"
    );

    // 5. QA-SYS-05 Edge Case: Unreachable AI Vision endpoint degrades gracefully to local heuristics
    let offline_client = UnifiedVisionClient::new(AiProviderConfig::OpenAiCompatible {
        endpoint_url: "http://127.0.0.1:9/v1".to_string(),
        model: "offline-vlm".to_string(),
        api_key_env: None,
    });
    let fallback_findings = offline_client.critique_screen(1, frame_ref, &elements, &[]);
    // Should not panic or error; returns deterministic findings (if any)
    let _ = fallback_findings;

    win.close();

    // 6. NFR-P4: Measure engine working-set RAM during an autonomous crawl session
    let temp_dir = std::env::temp_dir().join(format!("ui-qa-m5-soak-{}", std::process::id()));
    let cfg = QaSessionConfig {
        session_name: "m5-budget-check".to_string(),
        output_dir: temp_dir.clone(),
        max_steps: 3,
        ..Default::default()
    };
    let bundle = run_qa_session(&cfg, true).expect("run m5 session");
    let ws_mb = (query_process_working_set(std::process::id()) as f64) / (1024.0 * 1024.0);
    println!(
        "M5 Empirical Measurements: p95_cap={p95_cap:.2}ms, uia_heuristics={uia_plus_heuristics_ms:.2}ms, 10k_log_corr={corr_ms:.2}ms, ram={ws_mb:.2}MB, steps={}",
        bundle.report.total_steps
    );
    assert!(
        ws_mb <= 120.0,
        "NFR-P4 budget violated: working set {ws_mb:.2} MB > 120 MB"
    );
    let _ = std::fs::remove_dir_all(&temp_dir);
}
