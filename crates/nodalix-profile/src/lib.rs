//! Shared user profile, avatar resolution and visual preferences for Nodalix OS apps.

pub mod paths;
pub mod profile;
pub mod users;
pub mod visual;

pub use profile::{
    load_profile, load_profile_for_home, save_profile, set_avatar_from_file, ShellPreferences,
    UserProfile,
};
pub use users::{
    avatar_candidates, current_user, initials_from_name, local_users, resolve_avatar, LocalUser,
};
pub use visual::VisualPreferences;
