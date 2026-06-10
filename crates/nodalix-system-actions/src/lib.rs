//! Shared system session and power actions for Nodalix OS apps.

mod commands;
mod session;

pub use commands::{command_exists, resolve_nodalix_bin, run_command_status, spawn_detached};
pub use session::{
    availability, hibernate, lock_session, logout_session, poweroff, reboot, suspend,
    ActionAvailability, SystemAction,
};
