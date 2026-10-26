#![forbid(unsafe_code)]
//! Headless CLI Runner & Session Orchestrator (`ui-qa-cli`, `M3.1`).

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command;
use ui_qa_core::config::{ExecutionMode, QaSessionConfig};
use ui_qa_core::i18n::I18nCatalog;
use ui_qa_core::model::Severity;
use ui_qa_platform::actuator::list_top_level_windows;
use ui_qa_platform::orchestrator::run_qa_session;

#[derive(Debug, Parser)]
#[command(
    name = "ui-qa-cli",
    version,
    about = "Autonomous Windows UI & Hardware-in-the-Loop QA Runner for Rust GUI Apps (iced, egui, Slint)"
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

        /// Automatically create GitHub Issues in the target repo for High/Critical findings
        #[arg(long, default_value_t = false)]
        create_github_issues: bool,
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
        Commands::Run {
            config,
            exe,
            title,
            hwnd,
            output,
            max_steps,
            create_github_issues,
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

            if create_github_issues {
                if let Some(ref repo) = cfg.target_github_repo {
                    let drafts = bundle.report.to_github_issue_drafts(Severity::High);
                    for draft in drafts {
                        let _ = Command::new("gh")
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
                    }
                }
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

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
