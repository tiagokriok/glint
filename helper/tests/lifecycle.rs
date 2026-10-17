//! Process lifetime: parent-death signal, process-group cleanup, and bounded
//! subprocess execution. Only harmless fixture processes are used.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use glint_helper::process::{Child, apply_pdeathsig_with, run_capture_with_timeout, spawn_group};

fn fixture_role() -> Option<String> {
    std::env::var("GLINT_LIFECYCLE_FIXTURE").ok()
}

/// Fixture entry point. Runs harmlessly (no-op) during a normal test run.
#[test]
fn lifecycle_fixture() {
    match fixture_role().as_deref() {
        Some("parent") => fixture_parent(),
        Some("child") => fixture_child(),
        _ => {}
    }
}

fn fixture_parent() -> ! {
    let exe = std::env::current_exe().expect("current exe");
    let mut child = Command::new(exe)
        .arg("--exact")
        .arg("lifecycle_fixture")
        .arg("--nocapture")
        .env("GLINT_LIFECYCLE_FIXTURE", "child")
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn child fixture");

    println!("child={}", child.id());
    let _ = std::io::stdout().flush();

    // Wait until the child has armed its parent-death signal, then vanish.
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(l) if l.trim() == "ready" => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
    }
    std::process::exit(0);
}

fn fixture_child() -> ! {
    let parent = std::os::unix::process::parent_id();
    glint_helper::process::prepare_helper(parent).expect("arm helper lifetime");
    println!("ready");
    let _ = std::io::stdout().flush();
    std::thread::sleep(Duration::from_secs(30));
    std::process::exit(9);
}

fn pid_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

fn wait_until_gone(pid: u32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !pid_alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    !pid_alive(pid)
}

#[test]
fn pdeathsig_race_exits_when_parent_already_changed() {
    let mut raced = false;
    apply_pdeathsig_with(1234, |_option, _| true, || 4321, || raced = true);
    assert!(raced, "a changed parent must trigger the race exit");
}

#[test]
fn pdeathsig_race_does_not_exit_when_parent_matches() {
    let mut raced = false;
    apply_pdeathsig_with(1234, |_option, _| true, || 1234, || raced = true);
    assert!(!raced);
}

#[test]
fn pdeathsig_failure_is_best_effort() {
    let mut raced = false;
    apply_pdeathsig_with(1234, |_option, _| false, || 4321, || raced = true);
    assert!(!raced, "a failed prctl must not abort the child");
}

#[test]
fn parent_death_leaves_no_fixture_child() {
    if fixture_role().is_some() {
        return;
    }
    let exe = std::env::current_exe().expect("current exe");
    let mut parent = Command::new(exe)
        .arg("--exact")
        .arg("lifecycle_fixture")
        .arg("--nocapture")
        .env("GLINT_LIFECYCLE_FIXTURE", "parent")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn parent fixture");

    let stdout = parent.stdout.take().expect("parent stdout");
    let mut child_pid = None;
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if let Some(rest) = line.trim().strip_prefix("child=") {
            child_pid = rest.parse::<u32>().ok();
            break;
        }
    }
    let child_pid = child_pid.expect("parent reported child pid");
    parent.wait().expect("parent fixture exit");
    assert!(
        wait_until_gone(child_pid, Duration::from_secs(3)),
        "child {child_pid} survived its parent"
    );
}

#[test]
fn process_group_termination_kills_and_reaps() {
    let mut child: Child = spawn_group(&["sleep", "60"]).expect("spawn sleeper");
    let pid = child.id();
    assert!(pid_alive(pid));

    child.terminate_and_reap(Duration::from_secs(2));
    assert!(wait_until_gone(pid, Duration::from_secs(2)));
}

#[test]
fn timed_out_command_is_killed_and_returns_none() {
    let started = Instant::now();
    let result = run_capture_with_timeout(&["sleep", "60"], Duration::from_millis(200));
    assert!(result.is_none(), "a timed-out command is unavailable");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "timeout must be bounded"
    );
}

#[test]
fn command_output_and_status_are_captured() {
    let out = run_capture_with_timeout(&["echo", "hello"], Duration::from_secs(2))
        .expect("echo should run");
    assert_eq!(out.status, 0);
    assert_eq!(out.stdout.trim(), "hello");

    // fuser returns 1 when no process holds the device; that is still a valid answer.
    let none =
        run_capture_with_timeout(&["false"], Duration::from_secs(2)).expect("false should run");
    assert_eq!(none.status, 1);
}

#[test]
fn oversized_command_output_is_unavailable() {
    let result = run_capture_with_timeout(
        &["sh", "-c", "head -c 70000 /dev/zero"],
        Duration::from_secs(2),
    );
    assert!(result.is_none());
}

#[test]
fn descendant_holding_stdout_cannot_extend_command_timeout() {
    let started = Instant::now();
    let result = run_capture_with_timeout(
        &["sh", "-c", "sleep 2 & exit 0"],
        Duration::from_millis(100),
    );
    assert!(result.is_none());
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn invalid_commands_and_non_utf8_output_are_unavailable() {
    assert!(run_capture_with_timeout(&[], Duration::from_secs(1)).is_none());
    assert!(spawn_group(&[]).is_err());
    assert!(
        run_capture_with_timeout(&["/nonexistent/glint-test"], Duration::from_secs(1)).is_none()
    );
    assert!(
        run_capture_with_timeout(&["sh", "-c", "printf '\\377'"], Duration::from_secs(1)).is_none()
    );
}

#[test]
fn termination_escalates_for_a_term_ignoring_group() {
    let mut child = spawn_group(&["sh", "-c", "trap '' TERM; exec sleep 60"]).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    child.terminate_and_reap(Duration::from_millis(20));
    assert!(wait_until_gone(child.id(), Duration::from_secs(1)));
    child.terminate_and_reap(Duration::from_millis(20));
}
