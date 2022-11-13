use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy_debug_text_overlay::{screen_print, OverlayPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // !!!!IMPORTANT!!!! Add the OverlayPlugin here
        .add_plugin(OverlayPlugin { font_size: 32.0, ..Default::default() })
        .add_startup_system(setup)
        .add_system(screen_print_text)
        .add_system(show_fps)
        .add_system(show_cursor_position)
        .run();
}

#[derive(Debug)]
struct ForShow {
    field_1: f64,
    field_2: &'static str,
    field_3: Vec<usize>,
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn screen_print_text(time: Res<Time>) {
    let delta = time.delta_seconds_f64();
    let current_time = time.elapsed_seconds_f64();
    let at_interval = |t: f64| current_time % t < delta;
    let x = (13, 3.4);
    let show = ForShow {
        field_1: current_time - 30.0,
        field_2: "Hello world",
        field_3: vec![1, 2, 3, 4],
    };
    let mut mut_show = &mut ForShow {
        field_1: current_time + 30.0,
        field_2: "Hello world",
        field_3: vec![5, 2, 9, 1],
    };
    if at_interval(2.0) {
        screen_print!(sec: 0.5, "every 2:{x:?}");
    }
    if at_interval(5.0) {
        screen_print!(sec: 2.0, "every 5:{}", &x.0)
    }
    if at_interval(10.0) {
        screen_print!("every 10secs: {:.1}\n{mut_show:#?}", show.field_1)
    }
    mut_show.field_1 = 34.34234;
    if at_interval(13.0) {
        let col = Color::RED;
        screen_print!(col: col, "every 13: {}, {:?}", show.field_2, show.field_3)
    }
    if at_interval(5.0) {
        let col = Color::PINK;
        screen_print!(sec: 3.0, col: col, "every 30: {mut_show:?}");
    }
    if at_interval(0.1) {
        screen_print!("current time: {current_time:.2}")
    }
}

fn show_fps(time: Res<Time>, mut deltas: Local<Vec<f32>>, mut ring_ptr: Local<usize>) {
    let delta = time.delta_seconds_f64();
    let current_time = time.elapsed_seconds_f64();
    let at_interval = |t: f64| current_time % t < delta;
    if *ring_ptr >= 4096 {
        *ring_ptr = 0;
    }
    if deltas.len() <= *ring_ptr {
        deltas.push(time.delta_seconds());
    } else {
        deltas.insert(*ring_ptr, time.delta_seconds());
    }
    *ring_ptr += 1;
    if at_interval(2.0) {
        let fps = deltas.len() as f32 / deltas.iter().sum::<f32>();
        let last_fps = 1.0 / time.delta_seconds();
        screen_print!(col: Color::GREEN, "fps: {fps:.0}");
        screen_print!(col: Color::CYAN, "last: {last_fps:.0}");
    }
}

fn show_cursor_position(
    windows: Res<Windows>,
    time: Res<Time>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    let delta = time.delta_seconds_f64();
    let current_time = time.elapsed_seconds_f64();
    let at_interval = |t: f64| current_time % t < delta;
    if at_interval(0.5) {
        let (camera, camera_transform) = camera.single();
        if let RenderTarget::Window(window) = camera.target {
            let window = windows.get(window).unwrap();
            if let Some(screen_pos) = window.cursor_position() {
                let window_size = Vec2::new(window.width(), window.height());
                let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
                let ndc_to_world =
                    camera_transform.compute_matrix() * camera.projection_matrix().inverse();
                let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
                let world_pos: Vec2 = world_pos.truncate();

                screen_print!("World coords: {:.3}/{:.3}", world_pos.x, world_pos.y);
                screen_print!("Window coords: {:.3}/{:.3}", screen_pos.x, screen_pos.y);
            }
        }
    }
}
