//! Mocks for [`OverlayPlugin`] and [`screen_print`]
//!
//! `screen_print` "uses" the provided variables to avoid warnings when
//! disabling debug mode.
#[derive(Default)]
pub struct OverlayPlugin {
    pub font: Option<&'static str>,
    pub fallback_color: bevy::prelude::Color,
    pub font_size: f32,
}
impl bevy::prelude::Plugin for OverlayPlugin {
    fn build(&self, _app: &mut bevy::prelude::App) {}
}

#[macro_export]
macro_rules! screen_print {
    (push, col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {{
        let _ = ($color, format!($text $(, $fmt_args)*));
    }};
    (col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {{
        let _ = ($color, format!($text $(, $fmt_args)*));
    }};
    (push, sec: $timeout:expr, col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {{
        let _ = ($color, $timeout, format!($text $(, $fmt_args)*));
    }};
    (sec: $timeout:expr, col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {{
        let _ = ($color, $timeout, format!($text $(, $fmt_args)*));
    }};
    (push, sec: $timeout:expr, $text:expr $(, $fmt_args:expr)*) => {{
        let _ = ($timeout, format!($text $(, $fmt_args)*));
    }};
    (sec: $timeout:expr, $text:expr $(, $fmt_args:expr)*) => {{
        let _ = ($timeout, format!($text $(, $fmt_args)*));
    }};
    (push, $text:expr $(, $fmt_args:expr)*) => {{
        let _ = format!($text $(, $fmt_args)*);
    }};
    ($text:expr $(, $fmt_args:expr)*) => {{
        let _ = format!($text $(, $fmt_args)*);
    }};
    (@impl sec: $timeout:expr, col: $color:expr, $text:expr $(, $fmt_args:expr)*) => {{
        let _ = ($color, $timeout, format!($text $(, $fmt_args)*));
    }};
}
