//! End-to-end QA Session Orchestrator shared by `ui-qa-cli` (`M3.1`) and `ui-qa-ui` (`M4.1`).
//! Drives the observe → correlate → critique → act → report loop in the background without
//! stealing physical cursor or foreground focus.

use crate::actuator::{
    dispatch_action_background, inspect_window_uia_elements, list_top_level_windows,
};
use crate::capture::capture_hwnd_background;
use crate::process::MonitoredTargetProcess;
use crate::selftest_window::SelfTestWindow;
use chrono::Utc;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use ui_qa_ai::UnifiedVisionClient;
use ui_qa_core::config::{ExecutionMode, QaSessionConfig};
use ui_qa_core::correlator::HardwareLogCorrelator;
use ui_qa_core::crawler::AutonomousCrawler;
use ui_qa_core::diff::{annotate_frame, compare_frames, compute_dhash, dhash_distance};
use ui_qa_core::model::{
    BoundingRect, Finding, FindingCategory, LogStreamKind, QaStepRecord, Severity, UiAction,
};
use ui_qa_core::report::QaSessionReport;

#[derive(Debug, Clone)]
pub struct SessionOutputBundle {
    pub report: QaSessionReport,
    pub report_md_path: PathBuf,
    pub report_json_path: PathBuf,
    pub screenshots_dir: PathBuf,
}

/// Runs a complete QA session against either a configured target (`.exe` / `HWND` / window title)
/// or the built-in background `SelfTestWindow` when `use_selftest_window` is true.
pub fn run_qa_session(
    config: &QaSessionConfig,
    use_selftest_window: bool,
) -> Result<SessionOutputBundle, String> {
    let started_at = Utc::now();
    std::fs::create_dir_all(&config.output_dir)
        .map_err(|e| format!("Failed to create output dir {:?}: {e}", config.output_dir))?;
    let screenshots_dir = config.output_dir.join("screenshots");
    std::fs::create_dir_all(&screenshots_dir)
        .map_err(|e| format!("Failed to create screenshots dir: {e}"))?;

    let mut selftest_guard: Option<SelfTestWindow> = None;
    let (hwnd_raw, mut proc_mon, target_desc) = if use_selftest_window {
        let win = SelfTestWindow::spawn()?;
        thread::sleep(Duration::from_millis(80));
        let raw = win.hwnd_raw;
        selftest_guard = Some(win);
        let mon = MonitoredTargetProcess::attach_hwnd(
            raw,
            std::process::id(),
            &config.hardware.log_files,
        );
        mon.inject_synthetic_log(
            LogStreamKind::Stdout,
            "[BOOT] Hardware Controller Demo initialized; awaiting COM3 handshake.",
        );
        (
            raw,
            mon,
            "Built-in Self-Test Hardware Controller Window".to_string(),
        )
    } else if let Some(hwnd) = config.target_hwnd {
        let mon = MonitoredTargetProcess::attach_hwnd(
            hwnd,
            std::process::id(),
            &config.hardware.log_files,
        );
        (hwnd, mon, format!("Attached HWND {hwnd}"))
    } else if let Some(ref title_sub) = config.target_window_title {
        let lower = title_sub.to_ascii_lowercase();
        let win = list_top_level_windows()
            .into_iter()
            .find(|w| w.title.to_ascii_lowercase().contains(&lower))
            .ok_or_else(|| format!("No visible window matching title '{title_sub}'"))?;
        let mon =
            MonitoredTargetProcess::attach_hwnd(win.hwnd, win.pid, &config.hardware.log_files);
        (
            win.hwnd,
            mon,
            format!("Window '{}' (PID {})", win.title, win.pid),
        )
    } else if let Some(ref exe) = config.target_exe {
        let mut mon =
            MonitoredTargetProcess::spawn_exe(exe, &config.target_args, &config.hardware.log_files)
                .map_err(|e| format!("Failed to spawn {:?}: {e}", exe))?;
        thread::sleep(Duration::from_millis(450));
        let health = mon.sample_health();
        let win = list_top_level_windows()
            .into_iter()
            .find(|w| w.pid == health.pid)
            .ok_or_else(|| {
                format!(
                    "Spawned {:?} (PID {}) but found no top-level HWND",
                    exe, health.pid
                )
            })?;
        mon.set_hwnd(win.hwnd);
        (
            win.hwnd,
            mon,
            format!("{} (PID {})", exe.display(), health.pid),
        )
    } else {
        let win = SelfTestWindow::spawn()?;
        thread::sleep(Duration::from_millis(80));
        let raw = win.hwnd_raw;
        selftest_guard = Some(win);
        let mon = MonitoredTargetProcess::attach_hwnd(
            raw,
            std::process::id(),
            &config.hardware.log_files,
        );
        (raw, mon, "Default Self-Test Window".to_string())
    };

    let correlator =
        HardwareLogCorrelator::new(&config.hardware).map_err(|e| format!("Regex error: {e}"))?;
    let mut crawler = AutonomousCrawler::new(&config.destructive_control_regex)
        .map_err(|e| format!("Destructive regex error: {e}"))?;
    let vision = UnifiedVisionClient::new(config.ai.clone());

    let max_steps = config.max_steps.clamp(1, 200);
    let mut all_findings: Vec<Finding> = Vec::new();
    let mut step_records: Vec<QaStepRecord> = Vec::new();
    let mut prev_state_hash: Option<String> = None;
    let mut prev_dhash: Option<u64> = None;
    let mut last_action: Option<UiAction> = None;

    for step_idx in 0..max_steps {
        let action_ts_ms = proc_mon.elapsed_ms();
        let cap = capture_hwnd_background(hwnd_raw)?;
        let elements = inspect_window_uia_elements(hwnd_raw).unwrap_or_default();
        let dhash = compute_dhash(&cap.image);
        let state_hash = AutonomousCrawler::compute_state_hash(&elements, dhash);

        let raw_shot_path = screenshots_dir.join(format!("step-{step_idx:03}.png"));
        let _ = cap.image.save(&raw_shot_path);

        // Baseline visual regression diff if baseline_dir contains step-{step_idx:03}.png
        if let Some(ref base_dir) = config.baseline_dir {
            let base_file = base_dir.join(format!("step-{step_idx:03}.png"));
            if let Ok(base_dyn) = image::open(&base_file) {
                let base_rgba = base_dyn.to_rgba8();
                let (diff_sum, diff_img) = compare_frames(&base_rgba, &cap.image, 16);
                if diff_sum.changed_ratio > 0.01 {
                    let diff_path = screenshots_dir.join(format!("step-{step_idx:03}-diff.png"));
                    let _ = diff_img.save(&diff_path);
                    all_findings.push(Finding {
                        id: format!("VIS-DIFF-{step_idx:03}"),
                        severity: Severity::Medium,
                        category: FindingCategory::VisualRegression,
                        title: format!(
                            "Visual Regression vs Baseline at Step #{step_idx} ({:.1}% pixels changed, SSIM {:.3})",
                            diff_sum.changed_ratio * 100.0,
                            diff_sum.ssim_approx
                        ),
                        description: format!(
                            "Compared against baseline {:?}: {} pixels changed.",
                            base_file, diff_sum.changed_pixels
                        ),
                        suggested_fix: "Review the generated diff heatmap image to verify whether layout changes were intentional.".to_string(),
                        bounds: diff_sum.changed_bounding_box,
                        repro_steps: vec![format!("Step #{step_idx}")],
                        correlated_logs: Vec::new(),
                        screenshot_path: Some(diff_path),
                    });
                }
            }
        }

        let health = proc_mon.sample_health();
        let ui_changed = prev_state_hash
            .as_ref()
            .map(|p| p != &state_hash)
            .unwrap_or(true);
        let logs_snapshot = proc_mon.snapshot_logs();

        let corr = correlator.correlate_action(
            step_idx,
            last_action.as_ref(),
            action_ts_ms,
            &logs_snapshot,
            ui_changed,
            Some(&health),
        );

        let mut step_findings = corr.generated_findings;

        if let (Some(prev_h), Some(prev_d), Some(act)) =
            (&prev_state_hash, prev_dhash, last_action.as_ref())
        {
            if let Some(dead_f) = crawler.evaluate_transition(
                step_idx,
                prev_h,
                &state_hash,
                dhash_distance(prev_d, dhash),
                act,
                corr.matched_logs.len(),
            ) {
                step_findings.push(dead_f);
            }
        }

        let recent_log_strings: Vec<String> =
            corr.matched_logs.iter().map(|l| l.text.clone()).collect();
        let critique_findings =
            vision.critique_screen(step_idx, &cap.image, &elements, &recent_log_strings);
        step_findings.extend(critique_findings);

        // Save annotated frame showing UIA controls, anomalies, and next target
        let next_action = match config.mode {
            ExecutionMode::AutonomousCrawl => crawler.select_next_action(&state_hash, &elements),
            ExecutionMode::GoalScenario => config
                .goals
                .get(step_idx)
                .and_then(|g| vision.plan_goal_step(g, &elements)),
        };

        let active_rect = next_action.as_ref().and_then(|act| match act {
            UiAction::Click { element_id, .. } | UiAction::Toggle { element_id, .. } => elements
                .iter()
                .find(|e| &e.id == element_id)
                .map(|e| e.bounds),
            UiAction::InputText { element_id, .. } => elements
                .iter()
                .find(|e| &e.id == element_id)
                .map(|e| e.bounds),
        });

        let anomaly_rects: Vec<BoundingRect> =
            step_findings.iter().filter_map(|f| f.bounds).collect();
        let annotated = annotate_frame(&cap.image, &elements, &anomaly_rects, active_rect.as_ref());
        let annotated_path = screenshots_dir.join(format!("step-{step_idx:03}-annotated.png"));
        let _ = annotated.save(&annotated_path);

        for f in &mut step_findings {
            if f.screenshot_path.is_none() {
                f.screenshot_path = Some(annotated_path.clone());
            }
        }

        all_findings.extend(step_findings.clone());
        step_records.push(QaStepRecord {
            step_index: step_idx,
            timestamp: Utc::now(),
            state_hash: state_hash.clone(),
            dhash,
            action: next_action.clone(),
            screenshot_path: Some(raw_shot_path),
            annotated_screenshot_path: Some(annotated_path),
            correlated_logs: corr.matched_logs,
            health: Some(health),
            findings: step_findings,
        });

        prev_state_hash = Some(state_hash);
        prev_dhash = Some(dhash);

        let Some(act) = next_action else {
            break;
        };

        // Execute the action in background without stealing focus
        let _ = dispatch_action_background(hwnd_raw, &act, None);
        if use_selftest_window {
            match &act {
                UiAction::Click { element_name, .. } if element_name.contains("Connect") => {
                    proc_mon.inject_synthetic_log(
                        LogStreamKind::Stdout,
                        "[HW] Serial port COM3 connected at 115200 bps: ACK",
                    );
                }
                UiAction::Click { element_name, .. } if element_name.contains("Calibrate") => {
                    proc_mon.inject_synthetic_log(
                        LogStreamKind::Stderr,
                        "[HW] WARN: Sensor calibration drift +1.8% detected; retry or NACK timeout on channel 2",
                    );
                }
                _ => {}
            }
        }
        last_action = Some(act);
        thread::sleep(Duration::from_millis(60));
    }

    if let Some(mut win) = selftest_guard {
        win.close();
    }
    proc_mon.terminate();

    // Deduplicate findings by title so repeated observations across steps stay concise
    let mut unique_titles = std::collections::HashSet::new();
    all_findings.retain(|f| unique_titles.insert(f.title.clone()));

    let report = QaSessionReport {
        session_name: config.session_name.clone(),
        target_description: target_desc,
        started_at,
        completed_at: Utc::now(),
        total_steps: step_records.len(),
        unique_states_visited: crawler.visited_state_count().max(1),
        findings: all_findings,
        steps: step_records,
    };

    let report_md_path = config.output_dir.join("report.md");
    let report_json_path = config.output_dir.join("report.json");
    std::fs::write(&report_md_path, report.to_agent_markdown())
        .map_err(|e| format!("Failed to write report.md: {e}"))?;
    std::fs::write(
        &report_json_path,
        report.to_json_pretty().unwrap_or_default(),
    )
    .map_err(|e| format!("Failed to write report.json: {e}"))?;

    Ok(SessionOutputBundle {
        report,
        report_md_path,
        report_json_path,
        screenshots_dir,
    })
}
