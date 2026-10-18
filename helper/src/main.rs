//! `glint-helper`: bounded JSON-lines camera status observer.
//!
//! stdout carries one `{"version":1,"camera":{...}}` object per line; stderr is
//! diagnostic. Two independent observation paths are merged:
//!
//! - the event-driven PipeWire graph (main thread), and
//! - sustained `/dev/video*` holder evidence (worker loop).
//!
//! The helper never opens a camera device and reads no frames.

use std::io;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use glint_helper::attribution_worker::{AttributionWorker, enrich_job};
use glint_helper::device_access::{self, DeviceWatch};
use glint_helper::device_runtime::DeviceRuntime;
use glint_helper::observer;
use glint_helper::status::Status;

const VIDEO_DIR: &str = "/dev";
/// Upper bound between loop wakeups so native device events are drained promptly.
const MAX_WAIT: Duration = Duration::from_millis(50);

fn main() -> ExitCode {
    let parent = std::os::unix::process::parent_id();
    if let Err(error) = glint_helper::process::prepare_helper(parent) {
        eprintln!("glint-helper: lifetime setup failed: {error}");
        return ExitCode::FAILURE;
    }
    let stop = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::sync_channel::<Status>(256);

    let reconciler_stop = stop.clone();
    let reconciler = thread::spawn(move || run_reconciler(receiver, reconciler_stop));

    observer::run_pipewire(sender, stop.clone());

    stop.store(true, Ordering::Relaxed);
    let _ = reconciler.join();
    ExitCode::SUCCESS
}

fn run_reconciler(receiver: Receiver<Status>, stop: Arc<AtomicBool>) {
    let start = Instant::now();
    let mut watch = DeviceWatch::new();
    // Failure to start enrichment never changes detection availability.
    let worker =
        AttributionWorker::new(|job| enrich_job(Path::new("/proc"), Path::new("/sys"), &job)).ok();
    let mut runtime = DeviceRuntime::new(worker);

    let stdout = io::stdout();
    let mut out = stdout.lock();

    while !stop.load(Ordering::Relaxed) {
        let now = start.elapsed();

        if runtime
            .pipewire_updates(&mut out, receiver.try_iter().take(256))
            .is_err()
        {
            break;
        }

        for kind in watch.drain() {
            if kind.should_reconcile() {
                runtime.request(now);
            }
        }

        let mut discover = || device_access::try_discover_devices(Path::new(VIDEO_DIR));
        let mut lookup = |devices: &[std::path::PathBuf]| device_access::lookup_holders(devices);
        if runtime
            .poll(now, &mut out, &mut discover, &mut lookup, &mut |devices| {
                watch.sync(devices)
            })
            .is_err()
        {
            break;
        }

        let timeout = wait_timeout(now, runtime.next_deadline(now));
        match receiver.recv_timeout(timeout) {
            Ok(status) => {
                if runtime.pipewire_updates(&mut out, [status]).is_err() {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                // The PipeWire source stopped; keep the independent device path
                // running rather than spinning on a closed channel.
                if runtime
                    .pipewire_updates(&mut out, [observer::unavailable_status()])
                    .is_err()
                {
                    break;
                }
                thread::sleep(timeout);
            }
        }
    }
    stop.store(true, Ordering::Relaxed);
}

fn wait_timeout(now: Duration, deadline: Option<Duration>) -> Duration {
    let cap = now + MAX_WAIT;
    let target = deadline.map_or(cap, |deadline| deadline.min(cap));
    target.saturating_sub(now)
}
