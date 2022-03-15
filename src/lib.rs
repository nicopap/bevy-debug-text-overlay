#![doc = include_str!("../Readme.md")]
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
