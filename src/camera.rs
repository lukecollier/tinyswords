use bevy::prelude::*;

use crate::world::{TILE_SIZE, WORLD_SIZE};

#[derive(Component)]
pub struct MainCamera {
    pub move_by_viewport_borders: bool,
}

pub struct CameraPlugin<S: States> {
    state: S,
    or_state: S,
    loading_state: S,
}

impl<S: States> Plugin for CameraPlugin<S> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnTransition {
                exited: self.loading_state.clone(),
                entered: self.state.clone(),
            },
            setup_game_camera,
        )
        .add_systems(
            OnTransition {
                exited: self.loading_state.clone(),
                entered: self.or_state.clone(),
            },
            setup_game_camera,
        )
        .add_systems(
            Update,
            update_game_camera
                .run_if(in_state(self.state.clone()).or(in_state(self.or_state.clone()))),
        );
    }
}

impl<S: States> CameraPlugin<S> {
    pub fn run_on_state_or(state: S, or_state: S, loading_state: S) -> Self {
        Self {
            state,
            or_state,
            loading_state,
        }
    }
}

fn setup_game_camera(mut cmds: Commands) {
    cmds.spawn((
        Camera2d,
        Msaa::Off,
        MainCamera {
            move_by_viewport_borders: true,
        },
    ));
}

fn update_game_camera(
    time: Res<Time>,
    window_q: Query<&Window>,
    mut camera_q: Query<(&Camera, &mut Transform, &mut MainCamera)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // error if window does not exist
    let window = window_q.single();
    let (camera, mut camera_transform, camera_config) = camera_q.single_mut();
    // if the cursor is in the window we ready
    if let Some(cursor_pos) = window.cursor_position() {
        let camera_speed = 250.;
        let mut direction = Vec2::ZERO;
        if keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS) {
            direction += Vec2::Y;
        }
        if keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW) {
            direction -= Vec2::Y;
        }
        if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
            direction += Vec2::X;
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD) {
            direction -= Vec2::X;
        }
        if let Some(rect) = camera.logical_viewport_rect() {
            let mut inner = rect.clone();
            inner.min += Vec2::new(64., 64.);
            inner.max -= Vec2::new(64., 64.);
            if camera_config.move_by_viewport_borders {
                if !inner.contains(cursor_pos) && rect.contains(cursor_pos) {
                    if cursor_pos.y > inner.max.y {
                        direction += Vec2::Y;
                    }
                    if cursor_pos.y < inner.min.y {
                        direction -= Vec2::Y;
                    }
                    if cursor_pos.x < inner.min.x {
                        direction += Vec2::X;
                    }
                    if cursor_pos.x > inner.max.x {
                        direction -= Vec2::X;
                    }
                }
            }
            camera_transform.translation -=
                direction.extend(0.0) * time.delta_secs() * camera_speed;
            camera_transform.translation = camera_transform.translation.clamp(
                rect.half_size().extend(0.0),
                Vec3::new(
                    TILE_SIZE * WORLD_SIZE.x as f32,
                    TILE_SIZE * WORLD_SIZE.y as f32,
                    0.0,
                ) - rect.half_size().extend(0.0),
            );
        } else {
            camera_transform.translation -=
                direction.extend(0.0) * time.delta_secs() * camera_speed;
        }
    }
}
