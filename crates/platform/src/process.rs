//! Target process launcher, real-time `stdout`/`stderr` & diagnostic file log tailer,
//! and `IsHungAppWindow` / memory health watchdog (`M2.1`).

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use ui_qa_core::model::{LogStreamKind, ProcessHealthSample, TimestampedLogLine};
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::IsHungAppWindow;

pub struct MonitoredTargetProcess {
    child: Option<Child>,
    pid: u32,
    hwnd_raw: Option<isize>,
    epoch: Instant,
    logs: Arc<Mutex<Vec<TimestampedLogLine>>>,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
}

impl MonitoredTargetProcess {
    /// Spawns a target `.exe` with piped `stdout`/`stderr` and tails any configured diagnostic log files.
    pub fn spawn_exe(
        exe_path: &std::path::Path,
        args: &[String],
        log_files: &[PathBuf],
    ) -> std::io::Result<Self> {
        let epoch = Instant::now();
        let mut child = Command::new(exe_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let pid = child.id();
        let logs = Arc::new(Mutex::new(Vec::new()));
        let stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));

        if let Some(stdout) = child.stdout.take() {
            let logs_c = Arc::clone(&logs);
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    let ts = epoch.elapsed().as_millis() as u64;
                    if let Ok(mut guard) = logs_c.lock() {
                        guard.push(TimestampedLogLine {
                            timestamp_ms: ts,
                            stream: LogStreamKind::Stdout,
                            text: line,
                        });
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let logs_c = Arc::clone(&logs);
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    let ts = epoch.elapsed().as_millis() as u64;
                    if let Ok(mut guard) = logs_c.lock() {
                        guard.push(TimestampedLogLine {
                            timestamp_ms: ts,
                            stream: LogStreamKind::Stderr,
                            text: line,
                        });
                    }
                }
            });
        }

        let mut proc_mon = Self {
            child: Some(child),
            pid,
            hwnd_raw: None,
            epoch,
            logs,
            stop_flag,
        };
        proc_mon.start_file_tailers(log_files);
        Ok(proc_mon)
    }

    /// Attaches to an already-running window `HWND` and optional PID, tailing any configured log files.
    pub fn attach_hwnd(hwnd_raw: isize, pid: u32, log_files: &[PathBuf]) -> Self {
        let mut mon = Self {
            child: None,
            pid,
            hwnd_raw: Some(hwnd_raw),
            epoch: Instant::now(),
            logs: Arc::new(Mutex::new(Vec::new())),
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        mon.start_file_tailers(log_files);
        mon
    }

    fn start_file_tailers(&mut self, log_files: &[PathBuf]) {
        for raw_path in log_files {
            let path = raw_path.clone();
            let logs_c = Arc::clone(&self.logs);
            let stop_c = Arc::clone(&self.stop_flag);
            let epoch = self.epoch;
            thread::spawn(move || {
                let mut offset = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                while !stop_c.load(std::sync::atomic::Ordering::SeqCst) {
                    if let Ok(mut file) = File::open(&path) {
                        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
                        if len < offset {
                            offset = 0; // Log file rotated/truncated
                        }
                        if len > offset && file.seek(SeekFrom::Start(offset)).is_ok() {
                            let mut reader = BufReader::new(file);
                            let mut line = String::new();
                            while let Ok(bytes) = reader.read_line(&mut line) {
                                if bytes == 0 {
                                    break;
                                }
                                offset += bytes as u64;
                                let ts = epoch.elapsed().as_millis() as u64;
                                if let Ok(mut guard) = logs_c.lock() {
                                    guard.push(TimestampedLogLine {
                                        timestamp_ms: ts,
                                        stream: LogStreamKind::DiagnosticFile {
                                            path: path.clone(),
                                        },
                                        text: line.trim_end().to_string(),
                                    });
                                }
                                line.clear();
                            }
                        }
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            });
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    pub fn inject_synthetic_log(&self, stream: LogStreamKind, text: impl Into<String>) {
        let ts = self.elapsed_ms();
        if let Ok(mut guard) = self.logs.lock() {
            guard.push(TimestampedLogLine {
                timestamp_ms: ts,
                stream,
                text: text.into(),
            });
        }
    }

    pub fn snapshot_logs(&self) -> Vec<TimestampedLogLine> {
        self.logs.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn set_hwnd(&mut self, hwnd_raw: isize) {
        self.hwnd_raw = Some(hwnd_raw);
    }

    /// Samples current working-set RAM, `IsHungAppWindow` responsiveness, and process exit status.
    pub fn sample_health(&mut self) -> ProcessHealthSample {
        let timestamp_ms = self.elapsed_ms();
        let exited_code = self
            .child
            .as_mut()
            .and_then(|c| c.try_wait().ok().flatten())
            .and_then(|status| status.code());

        let is_hung = self
            .hwnd_raw
            .map(|raw| {
                // SAFETY: IsHungAppWindow queries Win32 window message responsiveness without side effects.
                unsafe { IsHungAppWindow(HWND(raw as *mut _)).as_bool() }
            })
            .unwrap_or(false);

        let working_set_bytes = query_process_working_set(self.pid);

        ProcessHealthSample {
            timestamp_ms,
            pid: self.pid,
            working_set_bytes,
            cpu_percent: 0.0,
            is_hung,
            exited_code,
        }
    }

    pub fn terminate(&mut self) {
        self.stop_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for MonitoredTargetProcess {
    fn drop(&mut self) {
        self.terminate();
    }
}

/// Queries the working-set memory (in bytes) of `pid` (or the current process if `pid == std::process::id()`).
pub fn query_process_working_set(pid: u32) -> u64 {
    // SAFETY: Opening process with read-only query rights, reading PROCESS_MEMORY_COUNTERS, and closing handle.
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
        else {
            return 0;
        };
        let mut counters = PROCESS_MEMORY_COUNTERS {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ..Default::default()
        };
        let ok = GetProcessMemoryInfo(
            handle,
            &mut counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
        .is_ok();
        let _ = CloseHandle(handle);
        if ok {
            counters.WorkingSetSize as u64
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_current_process_memory_and_tails_logs() {
        let current_pid = std::process::id();
        let ws = query_process_working_set(current_pid);
        assert!(ws > 1024 * 100, "Expected non-zero working set, got {ws}");

        let mut mon = MonitoredTargetProcess::attach_hwnd(0, current_pid, &[]);
        mon.inject_synthetic_log(LogStreamKind::Stdout, "Hardware ready: ACK");
        let logs = mon.snapshot_logs();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].text.contains("ACK"));

        let health = mon.sample_health();
        assert_eq!(health.pid, current_pid);
        assert!(!health.is_hung);
    }
}
