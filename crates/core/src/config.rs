//! Session configuration schema (`qa-session.toml`) for `ui-qa-tester`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    AutonomousCrawl,
    GoalScenario,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusMode {
    /// Default: never calls `SetForegroundWindow` or `SendInput` (uses UIA patterns, `PostMessageW`, or SDK IPC).
    Background,
    /// Opt-in fallback for apps that ignore unfocused input.
    ForegroundFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum AiProviderConfig {
    Gemini {
        model: String,
        api_key_env: String,
    },
    OpenAiCompatible {
        endpoint_url: String,
        model: String,
        api_key_env: Option<String>,
    },
    HeuristicsOnly,
}

impl Default for AiProviderConfig {
    fn default() -> Self {
        Self::Gemini {
            model: "gemini-2.5-flash".to_string(),
            api_key_env: "GEMINI_API_KEY".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareTelemetryConfig {
    pub capture_stdout: bool,
    pub capture_stderr: bool,
    pub log_files: Vec<PathBuf>,
    pub correlation_pre_window_ms: u64,
    pub correlation_post_window_ms: u64,
    pub ack_patterns: Vec<String>,
    pub error_patterns: Vec<String>,
}

impl Default for HardwareTelemetryConfig {
    fn default() -> Self {
        Self {
            capture_stdout: true,
            capture_stderr: true,
            log_files: Vec::new(),
            correlation_pre_window_ms: 50,
            correlation_post_window_ms: 2000,
            ack_patterns: vec![r"(?i)\b(ack|connected|ready|ok|calibrated|synced)\b".to_string()],
            error_patterns: vec![
                r"(?i)\b(nack|error|panic|timeout|timed\s*out|disconnect|failed|fault|overrun)\b"
                    .to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QaGoalStep {
    pub description: String,
    pub expected_ui_text: Option<String>,
    pub expected_log_regex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QaSessionConfig {
    pub session_name: String,
    pub target_exe: Option<PathBuf>,
    pub target_args: Vec<String>,
    pub target_hwnd: Option<isize>,
    pub target_window_title: Option<String>,
    pub mode: ExecutionMode,
    pub focus_mode: FocusMode,
    pub max_steps: usize,
    pub destructive_control_regex: String,
    pub hardware: HardwareTelemetryConfig,
    pub ai: AiProviderConfig,
    pub goals: Vec<QaGoalStep>,
    pub baseline_dir: Option<PathBuf>,
    pub output_dir: PathBuf,
    pub target_github_repo: Option<String>,
    pub locale: String,
}

impl Default for QaSessionConfig {
    fn default() -> Self {
        Self {
            session_name: "default-qa-session".to_string(),
            target_exe: None,
            target_args: Vec::new(),
            target_hwnd: None,
            target_window_title: None,
            mode: ExecutionMode::AutonomousCrawl,
            focus_mode: FocusMode::Background,
            max_steps: 50,
            destructive_control_regex:
                r"(?i)\b(delete|format|factory\s*reset|erase|wipe|flash\s*firmware)\b".to_string(),
            hardware: HardwareTelemetryConfig::default(),
            ai: AiProviderConfig::default(),
            goals: Vec::new(),
            baseline_dir: None,
            output_dir: PathBuf::from("qa-output"),
            target_github_repo: None,
            locale: "en".to_string(),
        }
    }
}

impl QaSessionConfig {
    pub fn from_toml_str(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(input)
    }

    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_default_config_toml() {
        let cfg = QaSessionConfig::default();
        let serialized = cfg.to_toml_string().expect("serialize toml");
        let parsed = QaSessionConfig::from_toml_str(&serialized).expect("deserialize toml");
        assert_eq!(cfg, parsed);
        assert_eq!(parsed.focus_mode, FocusMode::Background);
    }
}
