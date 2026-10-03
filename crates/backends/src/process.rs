//! Synchronous child-process ownership for the offline worker.

use std::{
    io::{Read, Seek, SeekFrom},
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};

#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);

impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        // Always reap, including timeout, cancellation, and early-return paths.
        if let Some(pid) = rustix::process::Pid::from_raw(self.0.id() as i32) {
            let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn run(command: &mut Command, timeout: Duration, cancellation: &Cancellation) -> Result<()> {
    anyhow::ensure!(!cancellation.is_cancelled(), "operation cancelled");
    let mut stderr = tempfile::tempfile().context("create process log")?;
    let program = command.get_program().to_string_lossy().into_owned();
    command
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr.try_clone()?);
    let mut child = ChildGuard(
        command
            .spawn()
            .with_context(|| format!("start {program}"))?,
    );
    let started = Instant::now();
    loop {
        if let Some(status) = child.0.try_wait()? {
            if status.success() {
                return Ok(());
            }
            let length = stderr.metadata()?.len();
            stderr.seek(SeekFrom::Start(length.saturating_sub(4_096)))?;
            let mut tail = Vec::new();
            stderr.take(4_096).read_to_end(&mut tail)?;
            bail!(
                "{program} exited with {status}: {}",
                String::from_utf8_lossy(&tail)
            );
        }
        if cancellation.is_cancelled() {
            bail!("{program} cancelled");
        }
        if started.elapsed() >= timeout {
            bail!("{program} timed out");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_failure_and_timeout_are_not_success() {
        let token = Cancellation::default();
        assert!(run(&mut Command::new("false"), Duration::from_secs(1), &token).is_err());
        let started = Instant::now();
        assert!(
            run(
                Command::new("sleep").arg("10"),
                Duration::from_millis(30),
                &token
            )
            .is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(2));
        token.cancel();
        assert!(run(&mut Command::new("true"), Duration::from_secs(1), &token).is_err());
    }

    #[test]
    fn timeout_stops_descendants_as_well_as_the_downloader() {
        let directory = tempfile::tempdir().unwrap();
        let marker = directory.path().join("still-running");
        let mut command = Command::new("sh");
        command
            .args(["-c", "(sleep 0.3; touch \"$1\") & wait", "downloader"])
            .arg(&marker);
        assert!(
            run(
                &mut command,
                Duration::from_millis(100),
                &Cancellation::default()
            )
            .is_err()
        );
        thread::sleep(Duration::from_millis(500));
        assert!(
            !marker.exists(),
            "a child process continued after the job timed out"
        );
    }
}
