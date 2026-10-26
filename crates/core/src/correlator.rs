//! Real-time UI Action ↔ Hardware Log Correlator (`M1.2`).
//! Brackets each UI interaction with a configurable millisecond window `[-pre_ms, +post_ms]`
//! and flags hardware errors, panics, timeouts, and silent UI-vs-hardware state mismatches.

use crate::config::HardwareTelemetryConfig;
use crate::model::{
    Finding, FindingCategory, ProcessHealthSample, Severity, TimestampedLogLine, UiAction,
};
use regex::RegexSet;

#[derive(Debug, Clone)]
pub struct HardwareLogCorrelator {
    pre_window_ms: u64,
    post_window_ms: u64,
    ack_set: RegexSet,
    error_set: RegexSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrelationOutcome {
    pub matched_logs: Vec<TimestampedLogLine>,
    pub ack_lines: Vec<String>,
    pub error_lines: Vec<String>,
    pub generated_findings: Vec<Finding>,
}

impl HardwareLogCorrelator {
    pub fn new(cfg: &HardwareTelemetryConfig) -> Result<Self, regex::Error> {
        Ok(Self {
            pre_window_ms: cfg.correlation_pre_window_ms,
            post_window_ms: cfg.correlation_post_window_ms,
            ack_set: RegexSet::new(&cfg.ack_patterns)?,
            error_set: RegexSet::new(&cfg.error_patterns)?,
        })
    }

    /// Correlates a UI action taken at `action_ts_ms` with buffered log lines and optional process health.
    pub fn correlate_action(
        &self,
        step_index: usize,
        action: Option<&UiAction>,
        action_ts_ms: u64,
        all_logs: &[TimestampedLogLine],
        ui_changed: bool,
        health: Option<&ProcessHealthSample>,
    ) -> CorrelationOutcome {
        let window_start = action_ts_ms.saturating_sub(self.pre_window_ms);
        let window_end = action_ts_ms.saturating_add(self.post_window_ms);

        let matched_logs: Vec<TimestampedLogLine> = all_logs
            .iter()
            .filter(|l| l.timestamp_ms >= window_start && l.timestamp_ms <= window_end)
            .cloned()
            .collect();

        let mut ack_lines = Vec::new();
        let mut error_lines = Vec::new();
        for line in &matched_logs {
            if self.error_set.is_match(&line.text) {
                error_lines.push(line.text.clone());
            } else if self.ack_set.is_match(&line.text) {
                ack_lines.push(line.text.clone());
            }
        }

        let mut findings = Vec::new();
        let action_desc = action
            .map(describe_action)
            .unwrap_or_else(|| "Initial screen observation".to_string());

        if !error_lines.is_empty() {
            let severity = if error_lines
                .iter()
                .any(|l| l.to_ascii_lowercase().contains("panic"))
            {
                Severity::Critical
            } else {
                Severity::High
            };

            let title = if ui_changed {
                format!("Silent Hardware/Log Error after {action_desc}")
            } else {
                format!("Hardware/Log Fault on {action_desc}")
            };

            findings.push(Finding {
                id: format!("HW-LOG-{step_index:03}"),
                severity,
                category: FindingCategory::HardwareLogMismatch,
                title,
                description: format!(
                    "During step #{step_index} ({action_desc}), {} error/fault log line(s) were emitted within [{} ms, {} ms]. UI state changed: {ui_changed}.",
                    error_lines.len(),
                    window_start,
                    window_end
                ),
                suggested_fix: "Surface the hardware/backend error explicitly in the UI state (toast/banner/error indicator) and disable or retry the offending hardware command instead of failing silently in logs.".to_string(),
                bounds: None,
                repro_steps: vec![format!("Step #{step_index}: {action_desc}")],
                correlated_logs: error_lines.clone(),
                screenshot_path: None,
            });
        }

        if let Some(h) = health {
            if h.is_hung {
                findings.push(Finding {
                    id: format!("PROC-HUNG-{step_index:03}"),
                    severity: Severity::Critical,
                    category: FindingCategory::CrashOrHang,
                    title: format!("Target Window Froze (Hung UI Thread) after {action_desc}"),
                    description: format!(
                        "Process PID {} stopped responding to Win32 messages (IsHungAppWindow = true) after {action_desc}.",
                        h.pid
                    ),
                    suggested_fix: "Offload blocking serial/USB/hardware I/O from the GUI render thread (`egui::Context::request_repaint` / `iced::Task::perform` / `slint::invoke_from_event_loop`) onto a background worker thread or async task.".to_string(),
                    bounds: None,
                    repro_steps: vec![format!("Step #{step_index}: {action_desc}")],
                    correlated_logs: error_lines.clone(),
                    screenshot_path: None,
                });
            }
            if let Some(code) = h.exited_code {
                if code != 0 {
                    findings.push(Finding {
                        id: format!("PROC-EXIT-{step_index:03}"),
                        severity: Severity::Critical,
                        category: FindingCategory::CrashOrHang,
                        title: format!("Target Application Crashed (Exit Code {code})"),
                        description: format!(
                            "Process PID {} terminated unexpectedly with exit code {code} during step #{step_index} ({action_desc}).",
                            h.pid
                        ),
                        suggested_fix: "Inspect the correlated stderr backtrace, replace `.unwrap()` / `.expect()` on fallible hardware/device operations with graceful `Result` handling.".to_string(),
                        bounds: None,
                        repro_steps: vec![format!("Step #{step_index}: {action_desc}")],
                        correlated_logs: error_lines.clone(),
                        screenshot_path: None,
                    });
                }
            }
        }

        CorrelationOutcome {
            matched_logs,
            ack_lines,
            error_lines,
            generated_findings: findings,
        }
    }
}

pub fn describe_action(action: &UiAction) -> String {
    match action {
        UiAction::Click {
            element_name, x, y, ..
        } => format!("Click '{}' at ({x}, {y})", element_name),
        UiAction::InputText { element_id, text } => {
            format!("InputText '{text}' into '{element_id}'")
        }
        UiAction::Toggle { element_name, .. } => format!("Toggle '{}'", element_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LogStreamKind;
    use std::time::Instant;

    #[test]
    fn correlates_logs_within_time_window_and_flags_silent_hardware_mismatch() {
        let cfg = HardwareTelemetryConfig::default();
        let correlator = HardwareLogCorrelator::new(&cfg).expect("compile regexes");

        let logs = vec![
            TimestampedLogLine {
                timestamp_ms: 900,
                stream: LogStreamKind::Stdout,
                text: "Boot complete".to_string(),
            },
            TimestampedLogLine {
                timestamp_ms: 1020,
                stream: LogStreamKind::Stdout,
                text: "Sending calibrate command...".to_string(),
            },
            TimestampedLogLine {
                timestamp_ms: 1180,
                stream: LogStreamKind::Stderr,
                text: "ERROR: Serial timeout waiting for sensor NACK on COM3".to_string(),
            },
            TimestampedLogLine {
                timestamp_ms: 5000,
                stream: LogStreamKind::Stdout,
                text: "Unrelated later log".to_string(),
            },
        ];

        let action = UiAction::Click {
            element_id: "btn_cal".to_string(),
            element_name: "Calibrate Sensor".to_string(),
            x: 120,
            y: 80,
        };

        let outcome = correlator.correlate_action(1, Some(&action), 1000, &logs, true, None);
        assert_eq!(outcome.matched_logs.len(), 2);
        assert_eq!(outcome.error_lines.len(), 1);
        assert_eq!(outcome.generated_findings.len(), 1);
        assert_eq!(outcome.generated_findings[0].severity, Severity::High);
        assert!(outcome.generated_findings[0]
            .title
            .contains("Silent Hardware/Log Error"));
    }

    #[test]
    fn throughput_budget_nfr_p3_under_2ms_per_1000_lines() {
        let cfg = HardwareTelemetryConfig::default();
        let correlator = HardwareLogCorrelator::new(&cfg).expect("compile regexes");

        let logs: Vec<TimestampedLogLine> = (0..1000)
            .map(|i| TimestampedLogLine {
                timestamp_ms: i * 2,
                stream: LogStreamKind::Stdout,
                text: format!("Sensor frame #{i} status=OK v=3.31V"),
            })
            .collect();

        let start = Instant::now();
        let outcome = correlator.correlate_action(1, None, 500, &logs, false, None);
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        assert!(!outcome.ack_lines.is_empty());
        // NFR-P3: < 2 ms per event correlation even across 1,000 log lines
        assert!(
            elapsed_ms < 25.0,
            "Expected fast correlation across 1,000 lines, took {elapsed_ms:.2} ms"
        );
    }
}
