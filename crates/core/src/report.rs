//! Agent-Ready Markdown, JSON, and GitHub Issue Report Formatter (`M1.4`).
//! Generates structured QA reports designed to be fed directly into AI coding agents
//! (Antigravity, Claude Code, GitHub Copilot, Cursor) to fix discovered bugs and UX issues.

use crate::model::{Finding, QaStepRecord, Severity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaSessionReport {
    pub session_name: String,
    pub target_description: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub total_steps: usize,
    pub unique_states_visited: usize,
    #[serde(default)]
    pub interactive_elements_discovered: usize,
    #[serde(default)]
    pub interactive_elements_exercised: usize,
    #[serde(default)]
    pub coverage_percentage: f64,
    pub findings: Vec<Finding>,
    pub steps: Vec<QaStepRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssueDraft {
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
}

impl QaSessionReport {
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Formats an agent-ready Markdown report with executive summary, prioritized findings,
    /// correlated hardware logs, and a copy-pasteable AI Coding Agent Fix Prompt.
    pub fn to_agent_markdown(&self) -> String {
        let mut md = String::with_capacity(4096);
        let _ = writeln!(
            md,
            "# Autonomous UI & Hardware QA Report — `{}`\n",
            self.session_name
        );
        let _ = writeln!(md, "- **Target:** `{}`", self.target_description);
        let _ = writeln!(
            md,
            "- **Completed At:** `{}`",
            self.completed_at.to_rfc3339()
        );
        let _ = writeln!(md, "- **Steps Executed:** `{}`", self.total_steps);
        let _ = writeln!(
            md,
            "- **Unique UI States Discovered:** `{}`",
            self.unique_states_visited
        );
        if self.interactive_elements_discovered > 0 {
            let _ = writeln!(
                md,
                "- **Interactive Element Coverage:** `{}` / `{}` elements exercised (`{:.1}%`)",
                self.interactive_elements_exercised,
                self.interactive_elements_discovered,
                self.coverage_percentage
            );
        }

        let critical_cnt = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Critical)
            .count();
        let high_cnt = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::High)
            .count();
        let med_cnt = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Medium)
            .count();
        let sug_cnt = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Suggestion)
            .count();

        let _ = writeln!(
            md,
            "- **Findings Summary:** `{}` Critical | `{}` High | `{}` Medium | `{}` UX Suggestions\n",
            critical_cnt, high_cnt, med_cnt, sug_cnt
        );

        let _ = writeln!(
            md,
            "---\n\n## Prioritized Findings & UX Improvement Suggestions\n"
        );

        if self.findings.is_empty() {
            let _ = writeln!(
                md,
                "_No defects or UX anomalies detected during this session._\n"
            );
        } else {
            let mut sorted = self.findings.clone();
            sorted.sort_by_key(|a| std::cmp::Reverse(a.severity));

            for f in &sorted {
                let _ = writeln!(
                    md,
                    "### [{}] `{}` — {} ({:?})\n",
                    severity_badge(f.severity),
                    f.id,
                    f.title,
                    f.category
                );
                let _ = writeln!(md, "- **Description:** {}", f.description);
                if let Some(b) = f.bounds {
                    let _ = writeln!(
                        md,
                        "- **Bounding Box (Client px):** `x={}, y={}, width={}, height={}`",
                        b.x, b.y, b.width, b.height
                    );
                }
                if let Some(ref shot) = f.screenshot_path {
                    let _ = writeln!(md, "- **Annotated Screenshot:** `{}`", shot.display());
                }
                if !f.repro_steps.is_empty() {
                    let _ = writeln!(md, "- **Reproduction Path:**");
                    for s in &f.repro_steps {
                        let _ = writeln!(md, "  1. {s}");
                    }
                }
                if !f.correlated_logs.is_empty() {
                    let _ = writeln!(md, "- **Correlated Hardware / App Logs:**\n```text");
                    for l in &f.correlated_logs {
                        let _ = writeln!(md, "{l}");
                    }
                    let _ = writeln!(md, "```");
                }
                let _ = writeln!(md, "- **Recommended Fix:** {}\n", f.suggested_fix);
            }
        }

        let _ = writeln!(md, "---\n\n## Ready-to-Paste AI Coding Agent Prompt\n");
        let _ = writeln!(md, "```markdown");
        let _ = writeln!(
            md,
            "Please fix the following {} UI/UX and hardware-handling issues discovered by `ui-qa-tester` in `{}`:",
            self.findings.len(),
            self.target_description
        );
        for (idx, f) in self.findings.iter().enumerate() {
            let _ = writeln!(
                md,
                "{}. [{} / {:?}] {}: {} -> Fix: {}",
                idx + 1,
                severity_badge(f.severity),
                f.category,
                f.title,
                f.description,
                f.suggested_fix
            );
        }
        let _ = writeln!(md, "```");

        md
    }

    /// Converts findings at or above `min_severity` into deduplicated GitHub Issue drafts.
    pub fn to_github_issue_drafts(&self, min_severity: Severity) -> Vec<GitHubIssueDraft> {
        let mut seen_titles = std::collections::HashSet::new();
        let mut drafts = Vec::new();

        for f in &self.findings {
            if f.severity < min_severity || !seen_titles.insert(f.title.clone()) {
                continue;
            }

            let mut body = String::new();
            let _ = writeln!(body, "## Automated QA Finding (`{}`)\n", f.id);
            let _ = writeln!(body, "- **Severity:** `{:?}`", f.severity);
            let _ = writeln!(body, "- **Category:** `{:?}`", f.category);
            let _ = writeln!(
                body,
                "- **Target Application:** `{}`\n",
                self.target_description
            );
            let _ = writeln!(body, "### What Happened\n{}\n", f.description);

            if !f.repro_steps.is_empty() {
                let _ = writeln!(body, "### Reproduction Steps");
                for step in &f.repro_steps {
                    let _ = writeln!(body, "- {step}");
                }
                let _ = writeln!(body);
            }

            if !f.correlated_logs.is_empty() {
                let _ = writeln!(body, "### Correlated Hardware / Diagnostic Logs\n```text");
                for line in &f.correlated_logs {
                    let _ = writeln!(body, "{line}");
                }
                let _ = writeln!(body, "```\n");
            }

            let _ = writeln!(body, "### Suggested Fix\n{}", f.suggested_fix);

            let label_severity = match f.severity {
                Severity::Critical => "priority:critical",
                Severity::High => "priority:high",
                Severity::Medium => "priority:medium",
                Severity::Suggestion => "enhancement",
            };

            drafts.push(GitHubIssueDraft {
                title: format!("[QA] {}", f.title),
                body,
                labels: vec!["ui-qa-tester".to_string(), label_severity.to_string()],
            });
        }

        drafts
    }
}

fn severity_badge(s: Severity) -> &'static str {
    match s {
        Severity::Critical => "CRITICAL",
        Severity::High => "HIGH",
        Severity::Medium => "MEDIUM",
        Severity::Suggestion => "SUGGESTION",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BoundingRect, FindingCategory};

    #[test]
    fn generates_agent_markdown_and_github_issue_drafts() {
        let finding = Finding {
            id: "HW-LOG-001".to_string(),
            severity: Severity::High,
            category: FindingCategory::HardwareLogMismatch,
            title: "Silent Hardware Timeout on Connect".to_string(),
            description: "Clicked Connect but COM3 timed out in stderr.".to_string(),
            suggested_fix: "Show an error banner when COM3 open fails.".to_string(),
            bounds: Some(BoundingRect::new(40, 60, 180, 44)),
            repro_steps: vec!["Step #1: Click 'Connect'".to_string()],
            correlated_logs: vec!["ERROR: COM3 timeout".to_string()],
            screenshot_path: None,
        };

        let report = QaSessionReport {
            session_name: "sensor-suite".to_string(),
            target_description: "sensor-gui.exe".to_string(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            total_steps: 5,
            unique_states_visited: 3,
            interactive_elements_discovered: 10,
            interactive_elements_exercised: 8,
            coverage_percentage: 80.0,
            findings: vec![finding],
            steps: Vec::new(),
        };

        let md = report.to_agent_markdown();
        assert!(md.contains("Silent Hardware Timeout on Connect"));
        assert!(md.contains("Ready-to-Paste AI Coding Agent Prompt"));

        let drafts = report.to_github_issue_drafts(Severity::Medium);
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].title, "[QA] Silent Hardware Timeout on Connect");
    }
}
