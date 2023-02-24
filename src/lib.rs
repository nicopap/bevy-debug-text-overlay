#![doc = include_str!("../Readme.md")]

use bevy::prelude::Resource;

#[cfg(feature = "debug")]
mod block;
#[cfg(feature = "debug")]
mod overlay;
#[cfg(feature = "debug")]
pub use overlay::{CommandChannels, InvocationSiteKey, OverlayPlugin, COMMAND_CHANNELS};

#[cfg(not(feature = "debug"))]
mod mocks;
#[cfg(not(feature = "debug"))]
pub use mocks::OverlayPlugin;

/// Control position on screen of the debug overlay.
#[derive(Resource, Default)]
pub struct DebugOverlayLocation {
    pub margin_vertical: f32,
    pub margin_horizontal: f32,
}
