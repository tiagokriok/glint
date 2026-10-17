//! Glint camera observer helper library.
//!
//! The helper merges two independent observation paths for camera use:
//!
//! - `camera`: event-driven PipeWire graph classification (V4L2 capture streams).
//! - `device_access`: sustained `/dev/video*` holder evidence.
//!
//! Pure classification and reconciliation logic lives in the modules below so
//! it can be tested with fixtures and fake clocks, never a real camera.

pub mod camera;
pub mod device_access;
pub mod observer;
pub mod process;
pub mod runtime;
pub mod status;
