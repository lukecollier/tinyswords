use bevy::prelude::*;

use crate::world::{TILE_SIZE, WORLD_SIZE};

#[derive(Component)]
pub struct MainCamera {
    pub move_by_viewport_borders: bool,
}

pub struct CameraPlugin<S: States> {
    state: S,
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
            Update,
            update_game_camera.run_if(in_state(self.state.clone())),
        );
    }
}

impl<S: States> CameraPlugin<S> {
    pub fn run_on_state(state: S, loading_state: S) -> Self {
        Self {
            state,
            loading_state,
        }
    }
}

fn setup_game_camera(mut cmds: Commands) {
    cmds.spawn((
        Transform::from_translation(Vec3::new(0., 0., 0.)),
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
    camera_q: Single<(&Camera, &mut MainCamera)>,
    camera_transform_q: Single<&mut Transform, With<MainCamera>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let (camera, camera_config) = camera_q.into_inner();
    // error if window does not exist
    let window = window_q.single();
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
            if direction != Vec2::ZERO {
                let mut camera_transform = camera_transform_q.into_inner();
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
            }
        } else {
            if direction != Vec2::ZERO {
                let mut camera_transform = camera_transform_q.into_inner();
                camera_transform.translation -=
                    direction.extend(0.0) * time.delta_secs() * camera_speed;
            }
        }
    }
}
