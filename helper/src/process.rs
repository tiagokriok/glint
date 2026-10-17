//! Child-process lifetime helpers: parent-death signal, process groups, and
//! bounded subprocess execution used for holder lookups.

use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Child as StdChild, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Arm the widget-owned helper before creating any worker threads.
pub fn prepare_helper(expected_parent: u32) -> io::Result<()> {
    // SIGKILL cannot be swallowed during a PipeWire reconnect or blocked lookup.
    // SAFETY: these calls only affect this process's lifetime/group.
    if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if std::os::unix::process::parent_id() != expected_parent {
        return Err(io::Error::other("widget exited during helper startup"));
    }
    if unsafe { libc::setpgid(0, 0) } != 0
        && unsafe { libc::getpgrp() } != std::process::id() as i32
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// `prctl(PR_SET_PDEATHSIG, ...)` option number (Linux).
pub const PR_SET_PDEATHSIG: i32 = libc::PR_SET_PDEATHSIG;

/// Request the kernel signal this process when `expected_parent` dies.
///
/// The parent may already have died between fork and this call, so the caller
/// checks its own parent id and exits on the race. Failure is best-effort.
pub fn apply_pdeathsig(expected_parent: u32) {
    apply_pdeathsig_with(
        expected_parent,
        |option, signal| {
            // SAFETY: prctl with PR_SET_PDEATHSIG and SIGTERM is async-signal safe.
            unsafe { libc::prctl(option, signal, 0, 0, 0) == 0 }
        },
        std::os::unix::process::parent_id,
        || {
            // The parent died during setup: do not linger.
            // SAFETY: _exit is async-signal safe and never returns.
            unsafe { libc::_exit(1) }
        },
    );
}

/// Testable core of [`apply_pdeathsig`].
pub fn apply_pdeathsig_with(
    expected_parent: u32,
    prctl: impl FnOnce(i32, i32) -> bool,
    getppid: impl FnOnce() -> u32,
    on_race: impl FnOnce(),
) {
    if !prctl(PR_SET_PDEATHSIG, libc::SIGTERM) {
        return;
    }
    if getppid() != expected_parent {
        on_race();
    }
}

/// A child placed in its own process group with a parent-death signal.
pub struct Child {
    inner: StdChild,
    pid: u32,
}

impl Child {
    pub fn id(&self) -> u32 {
        self.pid
    }

    /// Signal the child's process group and reap it within `timeout`.
    pub fn terminate_and_reap(&mut self, timeout: Duration) {
        if matches!(self.inner.try_wait(), Ok(Some(_))) {
            return;
        }
        signal_group(self.pid, libc::SIGTERM);
        if wait_timeout_child(&mut self.inner, timeout).is_none() {
            signal_group(self.pid, libc::SIGKILL);
            let _ = self.inner.wait();
        }
    }
}

/// Spawn a child in a new process group with `PR_SET_PDEATHSIG` set.
pub fn spawn_group(argv: &[&str]) -> io::Result<Child> {
    let (program, args) = split_argv(argv)?;
    let parent = std::process::id();
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    // SAFETY: the closure only calls async-signal-safe syscalls before exec.
    unsafe {
        command.pre_exec(move || {
            apply_pdeathsig(parent);
            Ok(())
        });
    }
    let inner = command.spawn()?;
    let pid = inner.id();
    Ok(Child { inner, pid })
}

/// Captured result of a bounded external command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: String,
}

/// Run a command with a hard timeout; `None` when it cannot run or times out.
///
/// The command is placed in its own process group with a parent-death signal,
/// so a timeout or helper crash cannot orphan it.
pub fn run_capture_with_timeout(argv: &[&str], timeout: Duration) -> Option<CommandOutput> {
    let (program, args) = split_argv(argv).ok()?;
    let parent = std::process::id();
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0);
    // SAFETY: the closure only calls async-signal-safe syscalls before exec.
    unsafe {
        command.pre_exec(move || {
            apply_pdeathsig(parent);
            Ok(())
        });
    }

    let mut child = command.spawn().ok()?;
    let pid = child.id();
    let mut stdout = child.stdout.take()?;
    // SAFETY: stdout owns this fd; change only its nonblocking flag.
    let flags = unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_GETFL) };
    let configured = flags >= 0
        && unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } >= 0;
    let result = configured
        .then(|| {
            let deadline = Instant::now() + timeout;
            let mut bytes = Vec::new();
            let mut eof = false;
            loop {
                if Instant::now() >= deadline {
                    return None;
                }
                let mut chunk = [0; 4096];
                match stdout.read(&mut chunk) {
                    Ok(0) => eof = true,
                    Ok(count) => {
                        bytes.extend_from_slice(&chunk[..count]);
                        if bytes.len() > 65536 {
                            return None;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                    Err(_) => return None,
                }
                if let Some(status) = child.try_wait().ok()?
                    && eof
                {
                    return Some(CommandOutput {
                        status: status.code().unwrap_or(-1),
                        stdout: String::from_utf8(bytes).ok()?,
                    });
                }
                thread::sleep(Duration::from_millis(1));
            }
        })
        .flatten();
    // Kill remaining group members even if the immediate child already exited.
    signal_group(pid, libc::SIGKILL);
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.wait();
    }
    result
}

fn split_argv<'a>(argv: &'a [&'a str]) -> io::Result<(&'a str, &'a [&'a str])> {
    argv.split_first()
        .map(|(program, args)| (*program, args))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty argv"))
}

fn signal_group(pid: u32, signal: i32) {
    // SAFETY: killpg/kill with a valid pid and signal number.
    if unsafe { libc::killpg(pid as i32, signal) } != 0 {
        unsafe {
            libc::kill(pid as i32, signal);
        }
    }
}

fn wait_timeout_child(child: &mut StdChild, timeout: Duration) -> Option<ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {}
            Err(_) => return None,
        }
        if Instant::now() >= deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(10));
    }
}
