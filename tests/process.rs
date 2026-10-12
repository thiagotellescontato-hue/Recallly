use recallly_probe::storage::*;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, mpsc},
    time::{Duration, Instant as Deadline},
};

struct OldClock;
impl Clock for OldClock {
    fn now(&self) -> Instant {
        Instant::parse("2000-01-01T00:00:00Z").unwrap()
    }
}

struct Process {
    child: Child,
    lines: mpsc::Receiver<String>,
}
impl Process {
    fn start(path: &Path) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_recallly-probe"));
        for (key, _) in std::env::vars_os() {
            if ["KEY", "TOKEN", "SECRET", "PASSWORD", "CERT"]
                .iter()
                .any(|word| key.to_string_lossy().to_ascii_uppercase().contains(word))
            {
                command.env_remove(key);
            }
        }
        let mut child = command
            .env("RECALLLY_DB_PATH", path)
            .env("RECALLLY_CLAIM_TIMEOUT_SECONDS", "900")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if send.send(line.unwrap()).is_err() {
                    break;
                }
            }
        });
        Self { child, lines }
    }
    fn send(&mut self, value: Value) {
        writeln!(self.child.stdin.as_mut().unwrap(), "{value}").unwrap();
    }
    fn reply(&self) -> Value {
        serde_json::from_str(
            &self
                .lines
                .recv_timeout(Duration::from_secs(10))
                .expect("MCP response deadline"),
        )
        .unwrap()
    }
    fn initialize(&mut self) {
        self.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"Recallly process test","version":"0.2.0"}}}));
        assert_eq!(self.reply()["result"]["serverInfo"]["version"], "0.2.0");
        self.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    }
    fn status(&mut self) {
        self.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"recallly_status","arguments":{}}}));
        let reply = self.reply();
        let result = &reply["result"]["structuredContent"];
        assert_eq!(result.as_object().unwrap().len(), 6);
        for key in [
            "service",
            "status",
            "version",
            "current_time",
            "timezone",
            "machine_id",
        ] {
            assert!(result.get(key).is_some());
        }
        assert_eq!(result["service"], "Recallly");
        assert_eq!(result["status"], "online");
        assert_eq!(result["version"], "0.2.0");
        assert!(!reply.to_string().contains("PRIVATE_REMINDER_SENTINEL"));
        assert!(!reply.to_string().contains("sqlite3"));
    }
    fn finish(&mut self, eof: bool) -> (std::process::ExitStatus, String) {
        if eof {
            drop(self.child.stdin.take());
        }
        let deadline = Deadline::now() + Duration::from_secs(10);
        let exit = loop {
            if let Some(exit) = self.child.try_wait().unwrap() {
                break exit;
            }
            assert!(Deadline::now() < deadline, "process termination deadline");
            std::thread::sleep(Duration::from_millis(10));
        };
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        (exit, stderr)
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[test]
fn real_process_lock_eof_restart_crash_release_recovery_and_data_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("reminders.sqlite3");
    let mut seed = Store::open(&path, Arc::new(OldClock)).unwrap();
    let reminder = seed
        .create(
            "process-test",
            Content::new(
                "PRIVATE_REMINDER_SENTINEL",
                "PRIVATE_REMINDER_SENTINEL",
                "2000-01-01T00:00:00Z",
                "UTC",
            )
            .unwrap(),
        )
        .unwrap();
    let claim = seed.claim_next_due().unwrap().unwrap();
    drop(seed);
    let mut first = Process::start(&path);
    first.initialize();
    first.status();
    let mut second = Process::start(&path);
    let (exit, stderr) = second.finish(false);
    assert!(!exit.success());
    assert_eq!(
        stderr.trim(),
        "Another Recallly instance is using this database"
    );
    assert!(second.lines.try_iter().next().is_none());
    let (exit, stderr) = first.finish(true);
    assert!(exit.success());
    assert!(stderr.is_empty());
    let mut inspect = Store::open(&path, Arc::new(OldClock)).unwrap();
    let recovered = inspect.get(reminder.id).unwrap().unwrap();
    assert_eq!(recovered.status, Status::Pending);
    assert_eq!(recovered.revision, claim.revision + 1);
    assert_eq!(recovered.attempt_count, 1);
    assert_eq!(recovered.content, reminder.content);
    assert!(matches!(
        inspect.mark_sent(claim.id, claim.revision),
        Err(Error::RevisionConflict)
    ));
    drop(inspect);
    let mut restarted = Process::start(&path);
    restarted.initialize();
    restarted.status();
    restarted.child.kill().unwrap();
    let _ = restarted.finish(false);
    let mut after_crash = Process::start(&path);
    after_crash.initialize();
    after_crash.status();
    let (exit, stderr) = after_crash.finish(true);
    assert!(exit.success());
    assert!(stderr.is_empty());
    let inspect = Store::open(&path, Arc::new(OldClock)).unwrap();
    assert_eq!(
        inspect.get(reminder.id).unwrap().unwrap().content,
        reminder.content
    );
}

#[test]
fn corrupt_database_fails_before_mcp_announces_availability() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.sqlite3");
    std::fs::write(&path, b"not a database").unwrap();
    let mut process = Process::start(&path);
    let (exit, stderr) = process.finish(false);
    assert!(!exit.success());
    assert!(!stderr.is_empty());
    assert!(!stderr.contains(&path.to_string_lossy().to_string()));
    assert!(process.lines.try_iter().next().is_none());
}
