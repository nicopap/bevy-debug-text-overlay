//! Overlay display for debugging
//!
//! # Architecture Overview
//!
//! The implementation is as follow:
//! * We have a static variable [`static@COMMAND_CHANNELS`] of type [`CommandChannels`]
//!   that contains channels for syncing [`Command`]s.
//! * [`screen_print!`] secretly expands to a call of to that global variable,
//!   it simply pushes messages to the sender channel using
//!   [`CommandChannels::refresh_text`] method. This is why, `COMMAND_CHANNELS` is
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

// TODO: better API?
/// Display text on top left corner of the screen.
///
/// The same `screen_print!` invocation can only have a single text displayed
/// on screen at the same time, unless specified otherwise.
///
/// # Limitations
///
/// * Entity count: Entities used for displaying text are never despawned,
///   so if at one point you have very many messages displayed at the same time,
///   it might slow down afterward your game. Note that aready spawned entities
///   are reused, so you need not fear leaks.
/// * Max call per frame: at most 4096 messages can be printed per frame,
///   exceeding that amount will panic.
///
/// # Usage
///
/// Call `screen_print!` like you would call any `format!`-style macros from
/// the standard lib.
///
/// You can also customize color and timeout, by adding prefix optional arguments
/// (only supported in this order):
///
/// 1. `push`: Do not overwrite previous text value. This allows
///    printing multiple messages from the same macro call, you can use this
///    in loops, or for messages that makes sense to duplicate on screen.
///    Be advised! Using a `push` message once per frame will spam the log.
/// 2. `sec: <timeout>`: specify in seconds for how long the text shows up
///    (default is 7 seconds)
/// 3. `col: <color>`: specify the color of the text. Default is
///    `fallback_color` provided in `OverlayPlugin`, which itself defaults
///    to yellow.
///
/// ```rust,no_run
/// use bevy_debug_text_overlay::{screen_print, OverlayPlugin};
/// use bevy::prelude::Color;
///
/// let x = (13, 3.4, vec![1,2,3,4,5,6,7,8]);
/// screen_print!("multiline: {x:#?}");
/// screen_print!(push, "This shows multiple times");
/// screen_print!(sec: 6.0, "first and second fields: {}, {}", x.0, x.1);
/// screen_print!(col: Color::BLUE, "single line: {x:?}");
/// screen_print!(sec: 10.0, col: Color::BLUE, "last field: {:?}", x.2);
/// ```
#[macro_export]
macro_rules! screen_print {
    (push, col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {
        screen_print!(@impl push, sec: 7.0, col: Some($color), $text $(, $fmt_args)*);
    };
    (col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {
        screen_print!(@impl sec: 7.0, col: Some($color), $text $(, $fmt_args)*);
    };
    (push, sec: $timeout:expr, col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {
        screen_print!(@impl push, sec: $timeout, col: Some($color), $text $(, $fmt_args)*);
    };
    (sec: $timeout:expr, col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {
        screen_print!(@impl sec: $timeout, col: Some($color), $text $(, $fmt_args)*);
    };
    (push, sec: $timeout:expr, $text:expr $(, $fmt_args:expr)*) => {
        screen_print!(@impl push, sec: $timeout, col: None, $text $(, $fmt_args)*);
    };
    (sec: $timeout:expr, $text:expr $(, $fmt_args:expr)*) => {
        screen_print!(@impl sec: $timeout, col: None, $text $(, $fmt_args)*);
    };
    (push, $text:expr $(, $fmt_args:expr)*) => {
        screen_print!(@impl push, sec: 7.0, col: None, $text $(, $fmt_args)*);
    };
    ($text:expr $(, $fmt_args:expr)*) => {
        screen_print!(@impl sec: 7.0, col: None, $text $(, $fmt_args)*);
    };
    (@impl sec: $timeout:expr, col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {{
        use $crate::{InvocationSiteKey, COMMAND_CHANNELS};
        let key = InvocationSiteKey { file: file!(), line: line!(), column: column!() };
        COMMAND_CHANNELS.refresh_text(key, || format!($text $(, $fmt_args)*), $timeout as f64, $color);
    }};
    (@impl push, sec: $timeout:expr, col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {{
        use $crate::{InvocationSiteKey, COMMAND_CHANNELS};
        let key = InvocationSiteKey { file: file!(), line: line!(), column: column!() };
        COMMAND_CHANNELS.push_text(key, || format!($text $(, $fmt_args)*), $timeout as f64, $color);
    }};
}

/// Specific call site of [`screen_print!`].
///
/// Used to identify where a message is coming from and replacing it on screen
/// when updated.
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
    /// Update in place or add new message already printed at given site.
    Refresh {
        key: InvocationSiteKey,
        color: Option<Color>,
        text: String,
        timeout: f64,
    },
    /// Always add the message to the screen.
    Push {
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
    pub fn refresh_text(
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
    pub fn push_text(
        &self,
        key: InvocationSiteKey,
        text: impl FnOnce() -> String,
        timeout: f64,
        color: Option<Color>,
    ) {
        let text = format!("{key} {}\n", text());
        let cmd = Command::Push { text, color, timeout };
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
    font_size: f32,
    color: Color,
}
impl<'a> From<&'a OverlayPlugin> for Options {
    fn from(plugin: &'a OverlayPlugin) -> Self {
        Self {
            color: plugin.fallback_color,
            font_size: plugin.font_size,
        }
    }
}

#[derive(Copy, Clone)]
struct PushEntry {
    entity: Entity,
    expired: f64,
}
#[derive(Default)]
struct PushList(Vec<PushEntry>);
impl PushList {
    fn new_or_allocate(
        &mut self,
        spawn_new: impl FnOnce() -> Entity,
        current: f64,
        timeout: f64,
    ) -> Option<Entity> {
        let free_existing = self.0.iter_mut().find(|entry| entry.expired < current);
        let ret = free_existing.as_ref().map(|entry| entry.entity);
        let new_entry = || PushEntry { entity: spawn_new(), expired: current + timeout };
        match free_existing {
            Some(to_update) => to_update.expired = current + timeout,
            None => self.0.push(new_entry()),
        }
        ret
    }
}
fn update_messages_as_per_commands(
    mut messages: Query<(&mut Text, &mut Message)>,
    mut key_entities: Local<HashMap<InvocationSiteKey, Entity>>,
    mut push_entities: Local<PushList>,
    mut cmds: Commands,
    time: Res<Time>,
    options: Res<Options>,
) {
    let channels = &COMMAND_CHANNELS;
    let text_style = |color| TextStyle {
        color,
        font_size: options.font_size,
        ..Default::default()
    };
    let current_time = time.elapsed_seconds_f64();
    let mut spawn_new = |text, color, timeout| {
        let style = Style { position_type: PositionType::Absolute, ..default() };
        cmds.spawn((
            TextBundle::from_section(text, text_style(color)).with_style(style),
            Message::new(timeout + current_time),
        ))
        .insert(Visibility::Hidden)
        .id()
    };
    let mut update_message = |entity, new_text, new_color, timeout| {
        // FIXME: this can skip requests if the scheduling acts up and we
        // get two consecutive message from the same `screen_print!`
        if let Ok((mut ui_text, mut message)) = messages.get_mut(entity) {
            message.expiration = timeout + current_time;
            if ui_text.sections[0].style.color != new_color {
                ui_text.sections[0].style.color = new_color;
            }
            if ui_text.sections[0].value != new_text {
                ui_text.sections[0].value = new_text;
            }
        }
    };
    let iterator = channels.receiver.lock().unwrap();
    for message in iterator.try_iter() {
        match message {
            Command::Refresh { key, color, text, timeout } => {
                let color = color.unwrap_or(options.color);
                if let Some(&entity) = key_entities.get(&key) {
                    update_message(entity, text, color, timeout);
                } else {
                    let entity = spawn_new(text, color, timeout);
                    key_entities.insert(key, entity);
                }
            }
            Command::Push { color, text, timeout } => {
                let color = color.unwrap_or(options.color);
                let spawn = || spawn_new(text.clone(), color, timeout);
                if let Some(entity) = push_entities.new_or_allocate(spawn, current_time, timeout) {
                    update_message(entity, text, color, timeout);
                }
            }
        }
    }
}

fn layout_messages(
    mut messages: Query<(Entity, &mut Style, &mut Visibility, &Node, &Message)>,
    mut line_sizes: Local<Blocks<Entity, f32>>,
    // position: Res<crate::DebugOverlayLocation>,
    time: Res<Time>,
) {
    use Visibility::{Hidden, Visible};
    for (entity, mut style, mut vis, node, message) in messages.iter_mut() {
        let size = node.size();
        let is_expired = message.expiration < time.elapsed_seconds_f64();
        let is_visible = *vis == Visible;
        if is_visible == is_expired {
            *vis = if is_visible { Hidden } else { Visible };
            if !is_expired {
                let offset = line_sizes.insert_size(entity, size.y);
                style.top = Val::Px(offset);
                style.left = Val::Px(0.0);
            } else {
                line_sizes.remove(entity);
            }
        }
    }
}

/// The text overlay plugin, you must add this plugin for the [`screen_print!`] macro
/// to work.
///
/// You can manage some of the text properties by setting the fields of the
/// plugin.
pub struct OverlayPlugin {
    /// The color to use when none are specified in [`screen_print!`], by
    /// default it is yellow.
    pub fallback_color: Color,
    /// The size of the message to display on screen, by default it is 13.0
    pub font_size: f32,
}
impl Default for OverlayPlugin {
    fn default() -> Self {
        Self { fallback_color: Color::YELLOW, font_size: 13.0 }
    }
}

impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource::<Options>(self.into()).add_systems(
            Update,
            (update_messages_as_per_commands, layout_messages).chain(),
        );
    }
}
