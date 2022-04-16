# Bevy Debug Text Overlay

[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)
[![Latest version](https://img.shields.io/crates/v/bevy_debug_text_overlay.svg)](https://crates.io/crates/bevy_debug_text_overlay)
[![Apache 2.0](https://img.shields.io/badge/license-Apache-blue.svg)](./LICENSE)
[![Documentation](https://docs.rs/bevy-debug-text-overlay/badge.svg)](https://docs.rs/bevy-debug-text-overlay/)

A proof of concept for adding a very convenient text overlay
macro to [the bevy game engine](https://bevyengine.org/).

This is derived from [the code I used during the first bevy game jam](https://github.com/team-plover/warlocks-gambit/blob/1ea5464717a45ea1ee96c1ab696c2c10d5cb79e8/src/debug_overlay.rs).
There are major improvements: most notably the text doesn't jump around all
the time, and each message can have its own color.

`screen_print!` **is very convenient**, if you are an incorrigible
println-debugger, you will love this crate when working with bevy!

## Usage

```toml
[dependencies]
bevy-debug-text-overlay = "2.0"
```

This bevy plugin is fairly trivial to use. You must:
1. Add the `OverlayPlugin` to your app
2. Add a `UiCameraBundle` entity
3. Use the `screen_print!` macro wherever you want, just use it like you would
   use `println!`, no need to pass special arguments.

This will display on the top left of the screen the text for a short time.

Please see the [`screen_print!`](https://docs.rs/bevy-debug-text-overlay/latest/bevy_debug_text_overlay/macro.screen_print.html) documentation for detailed usage instructions.

### Code example

```rust,no_run
use bevy::prelude::*;
use bevy_debug_text_overlay::{screen_print, OverlayPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // !!!!IMPORTANT!!!! Add the OverlayPlugin here
        .add_plugin(OverlayPlugin { font_size: 32.0, ..Default::default() })
        .add_startup_system(setup)
        .add_system(screen_print_text)
        .run();
}
fn setup(mut commands: Commands) {
    // !!!!IMPORTANT!!!! you must add a UiCameraBundle if you didn't already
    commands.spawn_bundle(UiCameraBundle::default());
}
// Notice how we didn't have to add any special system parameters
fn screen_print_text(time: Res<Time>) {
    let current_time = time.seconds_since_startup();
    let at_interval = |t: f64| current_time % t < time.delta_seconds_f64();
    let x = (13, 3.4, vec![1,2,3,4,5,6,7,8]);
    if at_interval(0.1) {
        let last_fps = 1.0 / time.delta_seconds();
        screen_print!(col: Color::CYAN, "fps: {last_fps:.0}");
        screen_print!("current time: {current_time:.2}")
    }
    if at_interval(2.0) {
        let col = Color::FUCHSIA;
        screen_print!(sec: 0.5, col: col, "every two seconds: {}, {:?}", x.0, x.2)
    }
    if at_interval(5.0) {
        screen_print!(sec: 3.0, "every five seconds: {x:#?}");
    }
}
```

This should look like as follow:

https://user-images.githubusercontent.com/26321040/158537677-e9339fd0-3bed-4a83-a4cc-bc1340e5d78b.mp4

### Cargo features

#### `builtin-font`

The plugin provides its own ascii font by default, but if you want to disable
it, you can disable the `builtin-font` cargo feature.

#### `debug`

It is possible to replace `screen_print!` by an empty macro by disabling the
`debug` cargo feature. This also disables all of `bevy-debug-text-overlay`
dependencies, since there is no code to run.

No further action is required to completely disable the plugin. Mock
implementations are provided for release mod.

To use that feature, you can setup your `Cargo.toml` as follow:

```toml
# Add a debug feature to your own Cargo.toml, make it default
[features]
debug = ["bevy-debug-text-overlay/debug"]
default = ["debug"]

# Manually specify features for bevy-debug-text-overlay (omitting "debug")
bevy-debug-text-overlay = { version = "2.0", default-features = false, features = ["builtin-font"] }
```

Now when making your release build, you should use
```sh
cargo build --release --no-default-features
```

I'm aware that it can be cumbersome for some, please fill an issue if this
really doesn't mix well with your own workflow.

## Notes on performance

It seems that built without compiler optimization, displaying text on screen in
bevy is a CPU hog, not sure why but it is (shrug). I designed the plugin with
performance in mind, but the culprit is bevy not me.

You might be interested in [enabling optimizations for dependencies in your debug
builds](https://bevy-cheatbook.github.io/pitfalls/performance.html).

## Known limitations

I'm welcoming contributions if you have any fixes:
* There is a chance that the `Blocks` algo may accumulate small differences in
  gaps in the text lines and become bloated.
* There is a very custom, very dodgy resource allocation module. If someone can
  link me to a good 1D res alloc crate, I'd be happy to use it instead of
  `block`.
* This is not part of bevy itself, so you gotta add it as a dependency to your
  app :(
* You can't set it up so that it's displayed from the bottom up or to the
  right of the screen.

## Changelog

* `2.0.0`: **Breaking**: bump bevy version to `0.7` (you should be able to
  upgrade from `1.0.0` without changing your code)

### Version matrix

| bevy | latest supporting version      |
|------|--------|
| 0.7  | 2.0.0 |
| 0.6  | 1.0.0 |

## API stability warning

This is a tinny crate so it's literally impossible to cause major breaking
changes. But I'm not convinced the current macro API is optimal, and it might
change in the future.

## License

This library is licensed under Apache 2.0.

### Font

The font in `screen_debug_text.ttf` is derived from Adobe SourceSans, licensed
under the SIL OFL. see file at `licenses/SIL Open Font License.txt`.
