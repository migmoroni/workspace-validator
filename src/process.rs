//! Shell-free subprocess execution with bounded capture and cancellation.

use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

/// Captured facts from one subprocess attempt.
#[derive(Debug)]
pub struct ProcessOutput {
    /// Process exit code, absent when startup failed or no code was available.
    pub exit_code: Option<i32>,
    /// Complete observed lifetime in milliseconds.
    pub duration_ms: u64,
    /// Whether the configured deadline terminated the process.
    pub timed_out: bool,
    /// Whether cooperative cancellation terminated the process.
    pub interrupted: bool,
    /// UTF-8-lossy tail of standard output.
    pub stdout: String,
    /// UTF-8-lossy tail of standard error.
    pub stderr: String,
    /// Whether earlier standard-output bytes were discarded.
    pub stdout_truncated: bool,
    /// Whether earlier standard-error bytes were discarded.
    pub stderr_truncated: bool,
    /// Startup failure captured before a child process existed.
    pub start_error: Option<String>,
}

/// Runs a command directly, bounding output, duration, and cancellation.
///
/// The command is never passed through a shell. On Unix, it receives a
/// dedicated process group; on Windows, it is attached to a Job Object. Both
/// mechanisms let cancellation and timeout terminate descendants.
pub fn run(
    program: &str,
    args: &[String],
    cwd: &Path,
    timeout: Duration,
    limit: usize,
    cancelled: &Arc<AtomicBool>,
) -> ProcessOutput {
    let started = Instant::now();
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_group(&mut command);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return startup_failure(started, error),
    };
    let process_tree = match ProcessTree::attach(&child) {
        Ok(process_tree) => process_tree,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return startup_failure(started, error);
        }
    };
    // Both streams must drain concurrently: waiting on one full OS pipe while
    // ignoring the other can deadlock an otherwise healthy child process.
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let stdout_reader = thread::spawn(move || capture_tail(stdout, limit));
    let stderr_reader = thread::spawn(move || capture_tail(stderr, limit));
    let mut timed_out = false;
    let mut interrupted = false;
    // Polling keeps cancellation and timeout handling portable while the
    // reader threads independently consume potentially large outputs.
    let status = loop {
        if cancelled.load(Ordering::SeqCst) {
            interrupted = true;
            process_tree.terminate(&mut child);
            break child.wait().ok();
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            process_tree.terminate(&mut child);
            break child.wait().ok();
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(_) => break child.wait().ok(),
        }
    };
    let (stdout, stdout_truncated) = stdout_reader.join().unwrap_or_default();
    let (stderr, stderr_truncated) = stderr_reader.join().unwrap_or_default();
    ProcessOutput {
        exit_code: status.and_then(|status| status.code()),
        duration_ms: started.elapsed().as_millis() as u64,
        timed_out,
        interrupted,
        stdout,
        stderr,
        stdout_truncated,
        stderr_truncated,
        start_error: None,
    }
}

fn startup_failure(started: Instant, error: std::io::Error) -> ProcessOutput {
    ProcessOutput {
        exit_code: None,
        duration_ms: started.elapsed().as_millis() as u64,
        timed_out: false,
        interrupted: false,
        stdout: String::new(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        start_error: Some(error.to_string()),
    }
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
struct ProcessTree;

#[cfg(unix)]
impl ProcessTree {
    fn attach(_child: &std::process::Child) -> std::io::Result<Self> {
        Ok(Self)
    }

    fn terminate(&self, child: &mut std::process::Child) {
        unsafe extern "C" {
            fn kill(pid: i32, signal: i32) -> i32;
        }
        // The child starts its own process group, so the negated PID targets
        // only that group and closes pipes inherited by its descendants.
        unsafe {
            kill(-(child.id() as i32), 9);
        }
        let _ = child.kill();
    }
}

#[cfg(windows)]
struct ProcessTree {
    job: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl ProcessTree {
    fn attach(child: &std::process::Child) -> std::io::Result<Self> {
        use std::{mem::size_of, os::windows::io::AsRawHandle, ptr};
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        let job = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if job.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        let process_tree = Self { job };
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                process_tree.job,
                JobObjectExtendedLimitInformation,
                (&raw const limits).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if configured == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let assigned =
            unsafe { AssignProcessToJobObject(process_tree.job, child.as_raw_handle().cast()) };
        if assigned == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(process_tree)
    }

    fn terminate(&self, child: &mut std::process::Child) {
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;

        unsafe {
            TerminateJobObject(self.job, 1);
        }
        let _ = child.kill();
    }
}

#[cfg(windows)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;

        unsafe {
            CloseHandle(self.job);
        }
    }
}

#[cfg(not(any(unix, windows)))]
struct ProcessTree;

#[cfg(not(any(unix, windows)))]
impl ProcessTree {
    fn attach(_child: &std::process::Child) -> std::io::Result<Self> {
        Ok(Self)
    }

    fn terminate(&self, child: &mut std::process::Child) {
        let _ = child.kill();
    }
}

fn capture_tail(mut reader: impl Read, limit: usize) -> (String, bool) {
    let mut tail = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut truncated = false;
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => read,
        };
        tail.extend_from_slice(&buffer[..read]);
        if tail.len() > limit {
            let excess = tail.len() - limit;
            tail.drain(..excess);
            truncated = true;
        }
    }
    (String::from_utf8_lossy(&tail).into_owned(), truncated)
}

#[cfg(test)]
mod tests {
    use super::run;
    use std::{
        path::{Path, PathBuf},
        process::Command,
        sync::OnceLock,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread,
        time::{Duration, Instant},
    };
    use tempfile::TempDir;

    fn fixture() -> &'static Path {
        static FIXTURE: OnceLock<PathBuf> = OnceLock::new();
        FIXTURE
            .get_or_init(|| {
                let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/process-tool");
                let status =
                    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
                        .args(["build", "--quiet", "--locked", "--manifest-path"])
                        .arg(root.join("Cargo.toml"))
                        .status()
                        .expect("build process fixture");
                assert!(status.success(), "process fixture did not compile");
                root.join("target/debug").join(format!(
                    "workspace-validator-process-fixture{}",
                    std::env::consts::EXE_SUFFIX
                ))
            })
            .as_path()
    }

    #[test]
    fn preserves_tail_and_times_out() {
        let temp = TempDir::new().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let output = run(
            fixture().to_str().unwrap(),
            &["emit".into(), "1234567890".into(), "".into(), "0".into()],
            temp.path(),
            Duration::from_secs(2),
            4,
            &cancelled,
        );
        assert_eq!(output.stdout, "7890");
        assert!(output.stdout_truncated);
        let output = run(
            fixture().to_str().unwrap(),
            &["sleep".into(), "2000".into()],
            temp.path(),
            Duration::from_millis(10),
            4096,
            &cancelled,
        );
        assert!(output.timed_out);
    }

    #[test]
    fn captures_large_stdout_and_stderr_without_deadlock() {
        let temp = TempDir::new().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let output = run(
            fixture().to_str().unwrap(),
            &["stream".into(), "2000".into()],
            temp.path(),
            Duration::from_secs(5),
            4096,
            &cancelled,
        );

        assert_eq!(output.exit_code, Some(0));
        assert!(output.stdout_truncated);
        assert!(output.stderr_truncated);
        assert!(output.stdout.len() <= 4096);
        assert!(output.stderr.len() <= 4096);
        assert!(output.stdout.ends_with("stdout-1999\n"));
        assert!(output.stderr.ends_with("stderr-1999\n"));
    }

    #[test]
    fn cancellation_terminates_an_existing_descendant_process() {
        let temp = TempDir::new().unwrap();
        let marker = temp.path().join("descendant-survived");
        let started_marker = temp.path().join("descendant-started");
        let cancelled = Arc::new(AtomicBool::new(false));
        let run_cancelled = Arc::clone(&cancelled);
        let program = fixture().to_path_buf();
        let cwd = temp.path().to_path_buf();
        let arguments = vec![
            "spawn-descendant".into(),
            marker.display().to_string(),
            started_marker.display().to_string(),
            "100".into(),
            "1000".into(),
            "30000".into(),
        ];
        let running = thread::spawn(move || {
            run(
                program.to_str().unwrap(),
                &arguments,
                &cwd,
                Duration::from_secs(30),
                4096,
                &run_cancelled,
            )
        });
        let deadline = Instant::now() + Duration::from_secs(10);
        while !started_marker.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(started_marker.exists(), "descendant process did not start");
        cancelled.store(true, Ordering::SeqCst);
        let output = running.join().unwrap();

        assert!(output.interrupted);
        thread::sleep(Duration::from_millis(1200));
        assert!(!marker.exists(), "descendant process survived cancellation");
    }
}
