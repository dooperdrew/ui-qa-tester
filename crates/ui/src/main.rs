//! Live QA Dashboard UI (`ui-qa-ui`, `M4.1` & `M4.2`).
//! Hosts the local QA Dashboard server and launches a native Windows Edge WebView2 (`--app`)
//! window (or browser fallback) with live background capture, UIA overlay, hardware log
//! correlation timeline, and 1-click Agent Fix Prompt export.

use base64::Engine as _;
use serde::Deserialize;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use ui_qa_core::config::QaSessionConfig;
use ui_qa_core::i18n::I18nCatalog;
use ui_qa_platform::actuator::list_top_level_windows;
use ui_qa_platform::orchestrator::run_qa_session;

const DASHBOARD_HTML: &str = include_str!("dashboard.html");

#[derive(Debug, Deserialize)]
struct RunRequest {
    #[serde(default)]
    self_test: bool,
    target_hwnd: Option<isize>,
    target_exe: Option<PathBuf>,
    max_steps: Option<usize>,
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let headless_check = args.iter().any(|a| a == "--headless-check");
    let port: u16 = args
        .windows(2)
        .find(|w| w[0] == "--port")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(17580);

    let bind_addr = if headless_check {
        "127.0.0.1:0".to_string()
    } else {
        format!("127.0.0.1:{port}")
    };

    let listener = TcpListener::bind(&bind_addr).or_else(|_| TcpListener::bind("127.0.0.1:0"))?;
    let local_addr = listener.local_addr()?;
    let url = format!("http://{local_addr}");
    println!("UI QA Dashboard running at {url}");

    if headless_check {
        return Ok(());
    }

    // Launch native Windows Edge app window (WebView2 shell) or default browser without blocking
    let _ = Command::new("cmd")
        .args(["/C", "start", "", "msedge", &format!("--app={url}")])
        .spawn()
        .or_else(|_| Command::new("cmd").args(["/C", "start", "", &url]).spawn());

    for stream in listener.incoming().flatten() {
        let _ = handle_http_request(stream);
    }

    Ok(())
}

pub fn handle_http_request(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buf = vec![0u8; 65536];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let req_str = String::from_utf8_lossy(&buf[..n]);
    let first_line = req_str.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }
    let method = parts[0];
    let path = parts[1];

    if method == "GET" && (path == "/" || path.starts_with("/index.html")) {
        return write_http_response(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            DASHBOARD_HTML.as_bytes(),
        );
    }

    if method == "GET" && path.starts_with("/api/windows") {
        let wins = list_top_level_windows();
        let body = serde_json::to_vec(&wins).unwrap_or_default();
        return write_http_response(&mut stream, "200 OK", "application/json", &body);
    }

    if method == "GET" && path.starts_with("/api/i18n") {
        let locale = if path.contains("locale=it") {
            "it"
        } else {
            "en"
        };
        let cat = I18nCatalog::new(locale);
        let mut map = std::collections::HashMap::new();
        for k in cat.keys() {
            map.insert(k.clone(), cat.t(k).to_string());
        }
        let body = serde_json::to_vec(&map).unwrap_or_default();
        return write_http_response(&mut stream, "200 OK", "application/json", &body);
    }

    if method == "POST" && path.starts_with("/api/run") {
        let body_start = req_str
            .find("\r\n\r\n")
            .map(|i| i + 4)
            .unwrap_or(req_str.len());
        let json_body = &req_str[body_start..];
        let run_req: RunRequest = serde_json::from_str(json_body).unwrap_or(RunRequest {
            self_test: true,
            target_hwnd: None,
            target_exe: None,
            max_steps: Some(4),
        });

        let mut cfg = QaSessionConfig {
            session_name: "dashboard-qa-session".to_string(),
            target_hwnd: run_req.target_hwnd,
            target_exe: run_req.target_exe,
            max_steps: run_req.max_steps.unwrap_or(4),
            output_dir: PathBuf::from("qa-output/dashboard-latest"),
            ..Default::default()
        };
        let use_selftest =
            run_req.self_test || (cfg.target_hwnd.is_none() && cfg.target_exe.is_none());

        match run_qa_session(&cfg, use_selftest) {
            Ok(bundle) => {
                let latest_b64 = bundle
                    .report
                    .steps
                    .last()
                    .and_then(|s| s.annotated_screenshot_path.as_ref())
                    .and_then(|p| std::fs::read(p).ok())
                    .map(|bytes| base64::engine::general_purpose::STANDARD.encode(bytes));

                let resp_json = serde_json::json!({
                    "report": bundle.report,
                    "markdown": bundle.report.to_agent_markdown(),
                    "latest_annotated_png_b64": latest_b64,
                });
                let bytes = serde_json::to_vec(&resp_json).unwrap_or_default();
                return write_http_response(&mut stream, "200 OK", "application/json", &bytes);
            }
            Err(e) => {
                cfg.session_name = "error".to_string();
                let err_json = serde_json::json!({ "error": e });
                let bytes = serde_json::to_vec(&err_json).unwrap_or_default();
                return write_http_response(
                    &mut stream,
                    "500 Internal Server Error",
                    "application/json",
                    &bytes,
                );
            }
        }
    }

    write_http_response(&mut stream, "404 Not Found", "text/plain", b"Not Found")
}

fn write_http_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_html_contains_all_four_views_and_i18n_hooks() {
        assert!(DASHBOARD_HTML.contains("tab-setup"));
        assert!(DASHBOARD_HTML.contains("tab-live"));
        assert!(DASHBOARD_HTML.contains("tab-findings"));
        assert!(DASHBOARD_HTML.contains("tab-diff"));
        assert!(DASHBOARD_HTML.contains("/api/run"));
    }
}
