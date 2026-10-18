//! Device reconciliation and ordered publication shared by the helper and tests.

use std::collections::BTreeSet;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

use crate::attribution_worker::{
    AttributionJob, AttributionResult, AttributionSession, AttributionWorker,
};
use crate::device_access::DeviceReconciler;
use crate::runtime::StatusStream;
use crate::status::{State, Status};

pub trait AttributionQueue {
    fn submit(&self, job: AttributionJob);
    fn clear_pending(&self);
    fn take_result(&self) -> Option<AttributionResult>;
}

impl AttributionQueue for AttributionWorker {
    fn submit(&self, job: AttributionJob) {
        self.submit(job);
    }
    fn clear_pending(&self) {
        self.clear_pending();
    }
    fn take_result(&self) -> Option<AttributionResult> {
        self.take_result()
    }
}

pub struct DeviceRuntime<Q> {
    worker: Option<Q>,
    reconciler: DeviceReconciler,
    attribution: AttributionSession,
    stream: StatusStream,
}

impl<Q: AttributionQueue> DeviceRuntime<Q> {
    pub fn new(worker: Option<Q>) -> Self {
        Self {
            worker,
            reconciler: DeviceReconciler::new(),
            attribution: AttributionSession::new(),
            stream: StatusStream::new(),
        }
    }

    pub fn request(&mut self, now: Duration) {
        self.reconciler.request(now);
    }

    pub fn next_deadline(&self, now: Duration) -> Option<Duration> {
        self.reconciler.next_deadline(now)
    }

    pub fn pipewire_updates(
        &mut self,
        out: &mut impl Write,
        updates: impl IntoIterator<Item = Status>,
    ) -> io::Result<()> {
        emit(out, self.stream.pipewire_updates(updates))
    }

    /// Reconcile and invalidate first, flush detection before scheduling work,
    /// then consume only results still valid for the current observation.
    pub fn poll(
        &mut self,
        now: Duration,
        out: &mut impl Write,
        discover: &mut dyn FnMut() -> Option<Vec<PathBuf>>,
        lookup: &mut dyn FnMut(&[PathBuf]) -> Option<BTreeSet<u32>>,
        sync: &mut dyn FnMut(&[PathBuf]),
    ) -> io::Result<()> {
        let refreshed = self.reconciler.poll(now, discover, lookup);
        sync(self.reconciler.devices());
        let job = self.attribution.reconcile(&self.reconciler, now, refreshed);
        emit(out, self.stream.device_update(self.attribution.status()))?;
        if let Some(worker) = &self.worker {
            if let Some(job) = job {
                worker.submit(job);
            } else if self.attribution.status().state != State::Active {
                worker.clear_pending();
            }
            if let Some(result) = worker.take_result()
                && self.attribution.accept(result)
            {
                emit(out, self.stream.device_update(self.attribution.status()))?;
            }
        }
        Ok(())
    }
}

fn emit(out: &mut impl Write, lines: impl IntoIterator<Item = String>) -> io::Result<()> {
    for line in lines {
        writeln!(out, "{line}")?;
    }
    out.flush()
}
