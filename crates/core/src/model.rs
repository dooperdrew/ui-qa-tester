//! Domain models for UI elements, state graph nodes, hardware logs, and QA findings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BoundingRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl BoundingRect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn center(&self) -> (i32, i32) {
        (
            self.x + (self.width as i32) / 2,
            self.y + (self.height as i32) / 2,
        )
    }

    pub fn area(&self) -> u64 {
        (self.width as u64) * (self.height as u64)
    }

    pub fn intersection_area(&self, other: &Self) -> u64 {
        let x_left = self.x.max(other.x);
        let y_top = self.y.max(other.y);
        let x_right = (self.x + self.width as i32).min(other.x + other.width as i32);
        let y_bottom = (self.y + self.height as i32).min(other.y + other.height as i32);

        if x_right <= x_left || y_bottom <= y_top {
            0
        } else {
            ((x_right - x_left) as u64) * ((y_bottom - y_top) as u64)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlSource {
    UiaAccessKit,
    VisionGrounded,
    SdkHook,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiElement {
    pub id: String,
    pub name: String,
    pub role: String,
    pub control_type_id: i32,
    pub value: Option<String>,
    pub is_enabled: bool,
    pub supports_invoke: bool,
    pub bounds: BoundingRect,
    pub source: ControlSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiAction {
    Click {
        element_id: String,
        element_name: String,
        x: i32,
        y: i32,
    },
    InputText {
        element_id: String,
        text: String,
    },
    Toggle {
        element_id: String,
        element_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogStreamKind {
    Stdout,
    Stderr,
    DiagnosticFile { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimestampedLogLine {
    pub timestamp_ms: u64,
    pub stream: LogStreamKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessHealthSample {
    pub timestamp_ms: u64,
    pub pid: u32,
    pub working_set_bytes: u64,
    pub cpu_percent: f32,
    pub is_hung: bool,
    pub exited_code: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Suggestion,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCategory {
    HardwareLogMismatch,
    VisualLayoutBug,
    UxFriction,
    AccessibilityGap,
    CrashOrHang,
    VisualRegression,
    ImprovementSuggestion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub category: FindingCategory,
    pub title: String,
    pub description: String,
    pub suggested_fix: String,
    pub bounds: Option<BoundingRect>,
    pub repro_steps: Vec<String>,
    pub correlated_logs: Vec<String>,
    pub screenshot_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QaStepRecord {
    pub step_index: usize,
    pub timestamp: DateTime<Utc>,
    pub state_hash: String,
    pub dhash: u64,
    pub action: Option<UiAction>,
    pub screenshot_path: Option<PathBuf>,
    pub annotated_screenshot_path: Option<PathBuf>,
    pub correlated_logs: Vec<TimestampedLogLine>,
    pub health: Option<ProcessHealthSample>,
    pub findings: Vec<Finding>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounding_rect_center_and_intersection() {
        let a = BoundingRect::new(10, 20, 100, 50);
        assert_eq!(a.center(), (60, 45));
        assert_eq!(a.area(), 5000);

        let b = BoundingRect::new(60, 20, 100, 50);
        assert_eq!(a.intersection_area(&b), 50 * 50);

        let disjoint = BoundingRect::new(200, 200, 30, 30);
        assert_eq!(a.intersection_area(&disjoint), 0);
    }
}
