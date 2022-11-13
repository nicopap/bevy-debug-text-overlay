//! Overlay display for debugging
//!
//! # Architecture Overview
//!
//! The implementation is as follow:
//! * We have a static variable [`static@COMMAND_CHANNELS`] of type [`CommandChannels`]
//!   that contains channels for syncing [`Command`]s.
//! * [`screen_print!`] secretly expands to a call of to that global variable,
//!   it simply pushes messages to the sender channel using
//!   [`CommandChannels::add_text`] method. This is why, `COMMAND_CHANNELS` is
//!   public. The end user code needs to be able to access it. But it is kept
//!   hidden thanks to the `#[doc(hidden)]` attribute.
//! * The [`update_messages_as_per_commands`] system reads from the `receiver`
//!   channel of `COMMAND_CHANNELS` and updates or adds new debug message entities.
//!   For each [`Command::Refresh`], a line is updated or added, a refresh can change
//!   the text or the color, and will always update the [`Message::expiration`].
//! * The [`layout_messages`] system takes care of the layout (making sure to
//!   **NOT** move visible text, filling empty spaces, and hidding expirated
//!   messages). It uses the dumb 1D allocation algorithm specified in
//!   [`crate::block`].
//!
//! ## Notes
//!
//! The channels use the `try_{send,iter}` methods to avoid blocking
//! operations. Since we are already running parallel thanks to the bevy
//! scheduler, there is no need for control-flow timing operations.
//!
//! Each individual invocation of [`screen_print!`] gets a unique
//! [`InvocationSiteKey`], and a corresponding `Entity`.
use std::fmt;
use std::sync::{
    mpsc::{self, Receiver, SyncSender},
    Mutex,
};

use bevy::{prelude::*, utils::HashMap};
use lazy_static::lazy_static;

use crate::block::Blocks;

const MAX_LINES: usize = 4096;
lazy_static! {
    #[doc(hidden)]
    pub static ref COMMAND_CHANNELS: CommandChannels = {
        let (sender, receiver) = mpsc::sync_channel(MAX_LINES);
        CommandChannels {
            sender,
            receiver: Mutex::new(receiver),
        }
    };
}

/// Display text on top left corner of the screen.
///
/// # Usage
///
/// Call `screen_print!` like you would call any `format!`-style macros from
/// the standard lib.
///
/// You can also customize color and timeout, by adding prefix arguments:
/// * `sec: <timeout>`: specify in seconds for how long the text shows up
///   (default is 7 seconds)
/// * `col: <color>`: specify the color of the text. Default is
///   `fallback_color` provided in `OverlayPlugin`, which itself defaults
///   to yellow.
///
/// If both prefixes are used, you must specify `sec` before `col`.
/// ```rust,no_run
/// use bevy_debug_text_overlay::{screen_print, OverlayPlugin};
/// use bevy::prelude::Color;
///
/// let x = (13, 3.4, vec![1,2,3,4,5,6,7,8]);
/// screen_print!("multiline: {x:#?}");
/// screen_print!(sec: 6.0, "first and second fields: {}, {}", x.0, x.1);
/// screen_print!(col: Color::BLUE, "single line: {x:?}");
/// screen_print!(sec: 10.0, col: Color::BLUE, "last field: {:?}", x.2);
/// ```
// TODO: better API?
#[macro_export]
macro_rules! screen_print {
    (col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {
        screen_print!(@impl sec: 7.0, col: Some($color), $text $(, $fmt_args)*);
    };
    (sec: $timeout:expr, col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {
        screen_print!(@impl sec: $timeout, col: Some($color), $text $(, $fmt_args)*);
    };
    (sec: $timeout:expr, $text:expr $(, $fmt_args:expr)*) => {
        screen_print!(@impl sec: $timeout, col: None, $text $(, $fmt_args)*);
    };
    ($text:expr $(, $fmt_args:expr)*) => {
        screen_print!(@impl sec: 7.0, col: None, $text $(, $fmt_args)*);
    };
    (@impl sec: $timeout:expr, col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {{
        use $crate::{InvocationSiteKey, COMMAND_CHANNELS};
        let key = InvocationSiteKey { file: file!(), line: line!(), column: column!() };
        COMMAND_CHANNELS.add_text(key, || format!($text $(, $fmt_args)*), $timeout as f64, $color);
    }};
}

#[derive(Hash, PartialEq, Eq)]
#[doc(hidden)]
pub struct InvocationSiteKey {
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}
impl fmt::Display for InvocationSiteKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}:{}:{}]", self.file, self.line, self.column)
    }
}

enum Command {
    Refresh {
        key: InvocationSiteKey,
        color: Option<Color>,
        text: String,
        timeout: f64,
    },
}

/// Queue text to display on the screen
#[doc(hidden)]
pub struct CommandChannels {
    sender: SyncSender<Command>,
    receiver: Mutex<Receiver<Command>>,
}
impl CommandChannels {
    // POSSIBLE LEAD: consider providing an API so that at_interval (from demo.rs) can
    // be used without too much hassle
    pub fn add_text(
        &self,
        key: InvocationSiteKey,
        text: impl FnOnce() -> String,
        timeout: f64,
        color: Option<Color>,
    ) {
        let text = format!("{key} {}\n", text());
        let cmd = Command::Refresh { text, key, color, timeout };
        self.sender
            .try_send(cmd)
            .expect("Number of debug messages exceeds limit!");
    }
}

#[derive(Component)]
struct Message {
    expiration: f64,
}
impl Message {
    fn new(expiration: f64) -> Self {
        Self { expiration }
    }
}

#[derive(Resource)]
struct Options {
    font: Option<&'static str>,
    font_size: f32,
    color: Color,
}
impl<'a> From<&'a OverlayPlugin> for Options {
    fn from(plugin: &'a OverlayPlugin) -> Self {
        Self {
            font: plugin.font,
            color: plugin.fallback_color,
            font_size: plugin.font_size,
        }
    }
}

#[derive(Resource)]
struct OverlayFont(Handle<Font>);
impl FromWorld for OverlayFont {
    fn from_world(world: &mut World) -> Self {
        let options = world.get_resource::<Options>().unwrap();
        let assets = world.get_resource::<AssetServer>().unwrap();
        let font = match options.font {
            Some(font) => assets.load(font),
            #[cfg(not(feature = "builtin-font"))]
            None => panic!(
                "No default font supplied, please either set the `builtin-font` \
                 flag or provide your own font file by setting the `font` field of \
                 `OverlayPlugin` to `Some(thing)`"
            ),
            #[cfg(feature = "builtin-font")]
            None => world.get_resource_mut::<Assets<Font>>().unwrap().add(
                Font::try_from_bytes(include_bytes!("screen_debug_text.ttf").to_vec())
                    .expect("The hardcoded builtin font is valid, this should never fail."),
            ),
        };
        Self(font)
    }
}

fn update_messages_as_per_commands(
    mut messages: Query<(&mut Text, &mut Message)>,
    mut key_entities: Local<HashMap<InvocationSiteKey, Entity>>,
    mut cmds: Commands,
    time: Res<Time>,
    options: Res<Options>,
    font: Res<OverlayFont>,
) {
    let channels = &COMMAND_CHANNELS;
    let text_style = |color| TextStyle {
        color,
        font: font.0.clone(),
        font_size: options.font_size,
    };
    let current_time = time.elapsed_seconds_f64();
    let iterator = channels.receiver.lock().unwrap();
    for Command::Refresh { key, color, text, timeout } in iterator.try_iter() {
        let color = color.unwrap_or(options.color);
        if let Some(&entity) = key_entities.get(&key) {
            // FIXME: this can skip requests if the scheduling acts up and we
            // get two consecutive message from the same `screen_print!`
            if let Ok((mut ui_text, mut message)) = messages.get_mut(entity) {
                message.expiration = timeout + current_time;
                if ui_text.sections[0].style.color != color {
                    ui_text.sections[0].style.color = color;
                }
                if ui_text.sections[0].value != text {
                    ui_text.sections[0].value = text;
                }
            }
        } else {
            let entity = cmds
                .spawn(
                    TextBundle::from_section(text, text_style(color)).with_style(Style {
                        position_type: PositionType::Absolute,
                        ..Default::default()
                    }),
                )
                .insert((
                    Visibility { is_visible: false },
                    Message::new(timeout + current_time),
                ))
                .id();
            key_entities.insert(key, entity);
        }
    }
}

fn layout_messages(
    mut messages: Query<(Entity, &mut Style, &mut Visibility, &Node, &Message)>,
    mut line_sizes: Local<Blocks<Entity, f32>>,
    time: Res<Time>,
) {
    for (entity, mut style, mut vis, node, text) in messages.iter_mut() {
        let size = node.size();
        let is_expired = text.expiration < time.elapsed_seconds_f64();
        if vis.is_visible == is_expired {
            vis.is_visible = !is_expired;
            if !is_expired {
                style.position.left = Val::Px(0.0);
                let offset = line_sizes.insert_size(entity, size.y);
                style.position.top = Val::Px(offset);
            } else {
                line_sizes.remove(entity);
            }
        }
    }
}

/// The text overlay plugin, you must add this plugin for the [`screen_print!`] macro
/// to work. You must also spawn a `UiCameraBundle`.
///
/// You can manage some of the text properties by setting the fields of the
/// plugin.
pub struct OverlayPlugin {
    /// The font to use, by default it is a variant of adobe's SourcePro
    /// only containing ascii characters.
    ///
    /// If the `builtin-font` flag is disabled, you must set this to `Some(thing)`,
    /// otherwise the plugin will panic at initialization.
    ///
    /// You can provide your own font by setting this to `Some("path/to/font.ttf")`.
    pub font: Option<&'static str>,
    /// The color to use when none are specified in [`screen_print!`], by
    /// default it is yellow.
    pub fallback_color: Color,
    /// The size of the message to display on screen, by default it is 13.0
    pub font_size: f32,
}
impl Default for OverlayPlugin {
    fn default() -> Self {
        Self {
            font: None,
            fallback_color: Color::YELLOW,
            font_size: 13.0,
        }
    }
}
impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource::<Options>(self.into())
            .init_resource::<OverlayFont>()
            .add_system(layout_messages.after("update_line"))
            .add_system(update_messages_as_per_commands.label("update_line"));
    }
}
