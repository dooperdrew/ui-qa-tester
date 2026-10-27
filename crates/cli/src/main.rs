#![forbid(unsafe_code)]
//! Headless CLI Runner & Session Orchestrator (`ui-qa-cli`, `M3.1`).

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command;
use ui_qa_core::config::{ExecutionMode, QaSessionConfig};
use ui_qa_core::i18n::I18nCatalog;
use ui_qa_core::model::Severity;
use ui_qa_core::report::GitHubIssueDraft;
use ui_qa_platform::actuator::list_top_level_windows;
use ui_qa_platform::orchestrator::{replay_action_trace, run_qa_session};

#[derive(Debug, Parser)]
#[command(
    name = "ui-qa-cli",
    version,
    about = "Autonomous Windows UI & Hardware-in-the-Loop QA Runner for Rust GUI Apps (iced, egui, Slint, Tauri)"
)]
struct Cli {
    /// UI/CLI language ("en" or "it")
    #[arg(long, default_value = "en")]
    locale: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run an autonomous crawl or goal-driven QA session
    Run {
        /// Path to qa-session.toml configuration file
        #[arg(long)]
        config: Option<PathBuf>,

        /// Override target executable path
        #[arg(long)]
        exe: Option<PathBuf>,

        /// Attach to a visible window matching title substring
        #[arg(long)]
        title: Option<String>,

        /// Attach to a specific HWND
        #[arg(long)]
        hwnd: Option<isize>,

        /// Output directory for report.md, report.json, and screenshots/
        #[arg(long)]
        output: Option<PathBuf>,

        /// Maximum autonomous steps
        #[arg(long)]
        max_steps: Option<usize>,

        /// Automatically sync GitHub Issues in the target repo for High/Critical findings with deduplication
        #[arg(long, default_value_t = false)]
        sync_github_issues: bool,

        /// Legacy alias for --sync-github-issues
        #[arg(long, default_value_t = false)]
        create_github_issues: bool,

        /// Exit with code 1 if any findings match or exceed this severity (e.g. "critical", "high", "medium")
        #[arg(long)]
        fail_on: Option<String>,

        /// Exit with code 1 if total findings exceed this number
        #[arg(long)]
        max_findings: Option<usize>,

        /// Process teardown strategy ("force_kill" or "graceful")
        #[arg(long)]
        teardown: Option<String>,
    },

    /// Replay an action-trace.json against a running window or process
    Replay {
        /// Path to qa-session.toml configuration file
        #[arg(long)]
        config: Option<PathBuf>,

        /// Path to action-trace.json recorded in a previous QA run
        #[arg(long)]
        trace: PathBuf,

        /// Attach to a visible window matching title substring
        #[arg(long)]
        title: Option<String>,

        /// Attach to a specific HWND
        #[arg(long)]
        hwnd: Option<isize>,

        /// Stop at a specific step index
        #[arg(long)]
        stop_at_step: Option<usize>,

        /// Output directory for replay screenshots and summary
        #[arg(long, default_value = "qa-output/replay")]
        output: PathBuf,
    },

    /// Sync QA session report findings to GitHub Issues with automatic deduplication
    SyncIssues {
        /// Path to report.json
        #[arg(long)]
        report: PathBuf,

        /// Target GitHub repository (e.g. 'owner/repo')
        #[arg(long)]
        repo: String,

        /// Minimum severity to sync (default: "high")
        #[arg(long, default_value = "high")]
        min_severity: String,
    },

    /// Watch target executable and run QA crawl on every new build
    Watch {
        /// Path to qa-session.toml
        #[arg(long)]
        config: PathBuf,

        /// Poll interval in milliseconds (default: 2000)
        #[arg(long, default_value_t = 2000)]
        interval_ms: u64,
    },

    /// Run a self-contained background QA self-test against the built-in Hardware Controller window
    SelfTest {
        /// Output directory for self-test report bundle
        #[arg(long, default_value = "qa-output/self-test")]
        output: PathBuf,
    },

    /// List all discoverable top-level Windows desktop windows
    Windows,

    /// Generate a starter qa-session.toml configuration file
    InitConfig {
        #[arg(long, default_value = "qa-session.toml")]
        path: PathBuf,
    },
}

fn parse_severity_str(s: &str) -> Option<Severity> {
    match s.trim().to_lowercase().as_str() {
        "critical" => Some(Severity::Critical),
        "high" => Some(Severity::High),
        "medium" => Some(Severity::Medium),
        "suggestion" => Some(Severity::Suggestion),
        _ => None,
    }
}

fn sync_drafts_to_github(repo: &str, drafts: &[GitHubIssueDraft]) -> anyhow::Result<usize> {
    let output = Command::new("gh")
        .args([
            "issue", "list", "--repo", repo, "--state", "all", "--limit", "100", "--json", "title",
        ])
        .output();

    let existing_titles: std::collections::HashSet<String> = if let Ok(out) = output {
        #[derive(serde::Deserialize)]
        struct IssueSummary {
            title: String,
        }
        let list: Vec<IssueSummary> = serde_json::from_slice(&out.stdout).unwrap_or_default();
        list.into_iter()
            .map(|i| i.title.trim().to_lowercase())
            .collect()
    } else {
        std::collections::HashSet::new()
    };

    let mut created = 0;
    for draft in drafts {
        if existing_titles.contains(&draft.title.trim().to_lowercase()) {
            println!("  [SKIP] Existing issue found: '{}'", draft.title);
            continue;
        }
        let status = Command::new("gh")
            .args([
                "issue",
                "create",
                "--repo",
                repo,
                "--title",
                &draft.title,
                "--body",
                &draft.body,
            ])
            .status();
        if let Ok(s) = status {
            if s.success() {
                println!("  [CREATED] Issue filed: '{}'", draft.title);
                created += 1;
            }
        }
    }
    Ok(created)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let i18n = I18nCatalog::new(&cli.locale);

    match cli.command {
        Commands::Windows => {
            let wins = list_top_level_windows();
            println!("{}", serde_json::to_string_pretty(&wins)?);
        }
        Commands::InitConfig { path } => {
            let cfg = QaSessionConfig::default();
            let toml_str = cfg.to_toml_string()?;
            std::fs::write(&path, toml_str)?;
            println!("Wrote starter configuration to {}", path.display());
        }
        Commands::SelfTest { output } => {
            let mut cfg = QaSessionConfig {
                session_name: "self-test-session".to_string(),
                output_dir: output,
                max_steps: 4,
                locale: cli.locale.clone(),
                ..Default::default()
            };
            cfg.mode = ExecutionMode::AutonomousCrawl;

            let bundle =
                run_qa_session(&cfg, true).map_err(|e| anyhow::anyhow!("Self-test failed: {e}"))?;
            println!("{}", i18n.t("cli.selftest.pass"));
            println!("  Markdown Report : {}", bundle.report_md_path.display());
            println!("  JSON Report     : {}", bundle.report_json_path.display());
            println!("  Screenshots Dir : {}", bundle.screenshots_dir.display());
            println!("  Findings Count  : {}", bundle.report.findings.len());
        }
        Commands::Replay {
            config,
            trace,
            title,
            hwnd,
            stop_at_step,
            output,
        } => {
            let mut cfg = if let Some(cfg_path) = config {
                let raw = std::fs::read_to_string(&cfg_path)?;
                QaSessionConfig::from_toml_str(&raw)?
            } else {
                QaSessionConfig::default()
            };
            if let Some(t) = title {
                cfg.target_window_title = Some(t);
            }
            if let Some(h) = hwnd {
                cfg.target_hwnd = Some(h);
            }
            cfg.output_dir = output;

            let summary_path = replay_action_trace(&cfg, &trace, stop_at_step)
                .map_err(|e| anyhow::anyhow!("Replay failed: {e}"))?;
            println!("Trace replay complete. Summary: {}", summary_path.display());
        }
        Commands::SyncIssues {
            report,
            repo,
            min_severity,
        } => {
            let raw = std::fs::read_to_string(&report)?;
            let rep: ui_qa_core::report::QaSessionReport = serde_json::from_str(&raw)?;
            let min_sev = parse_severity_str(&min_severity).unwrap_or(Severity::High);
            let drafts = rep.to_github_issue_drafts(min_sev);
            println!(
                "Syncing {} finding(s) with severity >= {:?} to GitHub repo '{}'...",
                drafts.len(),
                min_sev,
                repo
            );
            let created = sync_drafts_to_github(&repo, &drafts)?;
            println!("Synced: {} new issue(s) created.", created);
        }
        Commands::Watch {
            config,
            interval_ms,
        } => {
            let raw = std::fs::read_to_string(&config)?;
            let cfg = QaSessionConfig::from_toml_str(&raw)?;
            let Some(target_exe) = cfg.target_exe.clone() else {
                anyhow::bail!("Watch requires target_exe to be configured in qa-session.toml");
            };

            println!(
                "Watching {} for changes (poll interval: {}ms)...",
                target_exe.display(),
                interval_ms
            );
            let mut last_modified = std::fs::metadata(&target_exe)
                .and_then(|m| m.modified())
                .ok();
            let mut prev_findings_count: Option<usize> = None;

            loop {
                std::thread::sleep(std::time::Duration::from_millis(interval_ms));
                let current_modified = std::fs::metadata(&target_exe)
                    .and_then(|m| m.modified())
                    .ok();
                if current_modified.is_some() && current_modified != last_modified {
                    last_modified = current_modified;
                    println!(
                        "\n[CHANGE DETECTED] Target executable rebuilt. Spawning automated QA run..."
                    );
                    match run_qa_session(&cfg, false) {
                        Ok(bundle) => {
                            let curr_count = bundle.report.findings.len();
                            println!(
                                "Run finished: {} steps, {} findings.",
                                bundle.report.total_steps, curr_count
                            );
                            if let Some(prev) = prev_findings_count {
                                let delta_path = cfg.output_dir.join("delta.md");
                                let diff_msg = if curr_count < prev {
                                    format!(
                                        "Fixed {} finding(s)! (previously {}, now {})",
                                        prev - curr_count,
                                        prev,
                                        curr_count
                                    )
                                } else if curr_count > prev {
                                    format!(
                                        "Introduced {} new finding(s)! (previously {}, now {})",
                                        curr_count - prev,
                                        prev,
                                        curr_count
                                    )
                                } else {
                                    format!("Findings unchanged (count: {}).", curr_count)
                                };
                                println!("  Build Delta: {}", diff_msg);
                                let _ = std::fs::write(
                                    &delta_path,
                                    format!(
                                        "# QA Build Delta\n\n- **Target:** `{}`\n- **Status:** {}\n",
                                        target_exe.display(),
                                        diff_msg
                                    ),
                                );
                            }
                            prev_findings_count = Some(curr_count);
                        }
                        Err(e) => {
                            eprintln!("QA session failed: {e}");
                        }
                    }
                }
            }
        }
        Commands::Run {
            config,
            exe,
            title,
            hwnd,
            output,
            max_steps,
            sync_github_issues,
            create_github_issues,
            fail_on,
            max_findings,
            teardown,
        } => {
            let mut cfg = if let Some(cfg_path) = config {
                let raw = std::fs::read_to_string(&cfg_path)?;
                QaSessionConfig::from_toml_str(&raw)?
            } else {
                QaSessionConfig::default()
            };

            if let Some(e) = exe {
                cfg.target_exe = Some(e);
            }
            if let Some(t) = title {
                cfg.target_window_title = Some(t);
            }
            if let Some(h) = hwnd {
                cfg.target_hwnd = Some(h);
            }
            if let Some(o) = output {
                cfg.output_dir = o;
            }
            if let Some(m) = max_steps {
                cfg.max_steps = m;
            }
            if let Some(td) = teardown {
                cfg.teardown_strategy = td;
            }

            let use_selftest = cfg.target_exe.is_none()
                && cfg.target_window_title.is_none()
                && cfg.target_hwnd.is_none();

            let bundle = run_qa_session(&cfg, use_selftest)
                .map_err(|e| anyhow::anyhow!("Run failed: {e}"))?;

            println!(
                "Session '{}' complete ({} steps, {} findings).",
                bundle.report.session_name,
                bundle.report.total_steps,
                bundle.report.findings.len()
            );
            println!("  Markdown Report : {}", bundle.report_md_path.display());
            println!("  JSON Report     : {}", bundle.report_json_path.display());
            println!(
                "  Element Coverage: {:.1}% ({}/{} interactive elements)",
                bundle.report.coverage_percentage,
                bundle.report.interactive_elements_exercised,
                bundle.report.interactive_elements_discovered
            );

            let should_sync_issues = sync_github_issues || create_github_issues;
            if should_sync_issues {
                if let Some(ref repo) = cfg.target_github_repo {
                    let drafts = bundle.report.to_github_issue_drafts(Severity::High);
                    let _ = sync_drafts_to_github(repo, &drafts);
                }
            }

            // CI/CD Severity and Findings Gate
            let fail_sev = fail_on
                .as_deref()
                .and_then(parse_severity_str)
                .or(cfg.fail_on_severity);
            let mut failed_gate = false;

            if let Some(threshold) = fail_sev {
                let failing: Vec<_> = bundle
                    .report
                    .findings
                    .iter()
                    .filter(|f| f.severity >= threshold)
                    .collect();
                if !failing.is_empty() {
                    eprintln!(
                        "\n[CI/CD GATE FAILED] {} finding(s) met or exceeded severity threshold '{:?}':",
                        failing.len(),
                        threshold
                    );
                    for f in &failing {
                        eprintln!("  - [{:?}] {}", f.severity, f.title);
                    }
                    failed_gate = true;
                }
            }

            let max_thresh = max_findings.or(cfg.max_findings_threshold);
            if let Some(max_f) = max_thresh {
                if bundle.report.findings.len() > max_f {
                    eprintln!(
                        "\n[CI/CD GATE FAILED] Total findings ({}) exceeded maximum threshold ({})",
                        bundle.report.findings.len(),
                        max_f
                    );
                    failed_gate = true;
                }
            }

            if failed_gate {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_end_to_end_self_test_generates_report_bundle() {
        let temp_dir =
            std::env::temp_dir().join(format!("ui-qa-cli-test-{}", uuid::Uuid::new_v4()));
        let cfg = QaSessionConfig {
            session_name: "unit-selftest".to_string(),
            output_dir: temp_dir.clone(),
            max_steps: 3,
            ..Default::default()
        };

        let bundle = run_qa_session(&cfg, true).expect("run self-test session");
        assert!(bundle.report_md_path.exists());
        assert!(bundle.report_json_path.exists());
        assert!(bundle.report.total_steps >= 2);
        assert!(!bundle.report.findings.is_empty());
        assert!(bundle.report.coverage_percentage >= 0.0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
