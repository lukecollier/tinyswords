use bevy::prelude::*;

use crate::world::{TILE_SIZE, WORLD_SIZE};

#[derive(Component)]
pub struct GameCamera;

pub struct CameraPlugin<S: States> {
    state: S,
}

impl<S: States> Plugin for CameraPlugin<S> {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(self.state.clone()), setup_game_camera)
            .add_systems(
                Update,
                update_game_camera.run_if(in_state(self.state.clone())),
            );
    }
}

impl<S: States> CameraPlugin<S> {
    pub fn run_on_state(state: S) -> Self {
        Self { state }
    }
}

fn setup_game_camera(mut cmds: Commands) {
    cmds.spawn((Camera2dBundle::default(), GameCamera));
}

fn update_game_camera(
    time: Res<Time>,
    window_q: Query<&Window>,
    mut camera_q: Query<(&Camera, &mut Transform), With<GameCamera>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // error if window does not exist
    let window = window_q.single();
    let (_, mut camera_transform) = camera_q.single_mut();
    // if the cursor is in the window we ready
    if let Some(cursor_pos) = window.cursor_position() {
        let camera_speed = 250.;
        let height = window.height() as f32;
        let width = window.width() as f32;
        let height_percent = height / 100. * 7.5;
        let width_percent = width / 100. * 7.5;
        let mut direction = Vec2::ZERO;
        if cursor_pos.y > height - height_percent || keyboard_input.pressed(KeyCode::ArrowDown) {
            direction += Vec2::Y;
        }
        if cursor_pos.y < height_percent || keyboard_input.pressed(KeyCode::ArrowUp) {
            direction -= Vec2::Y;
        }
        if cursor_pos.x < width_percent || keyboard_input.pressed(KeyCode::ArrowLeft) {
            direction += Vec2::X;
        }
        if cursor_pos.x > width - width_percent || keyboard_input.pressed(KeyCode::ArrowRight) {
            direction -= Vec2::X;
        }
        camera_transform.translation -= direction.extend(0.0) * time.delta_seconds() * camera_speed;
        camera_transform.translation = camera_transform.translation.clamp(
            Vec3::new(width / 2., height / 2., 0.0),
            Vec3::new(
                TILE_SIZE * WORLD_SIZE.x as f32 - (width / 2.),
                TILE_SIZE * WORLD_SIZE.y as f32 - (height / 2.),
                0.0,
            ),
        );
    }
}
