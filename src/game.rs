use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

use crate::{
    camera::MainCamera,
    characters::{Character, Goal, Target},
    nav::Navigation,
    GameState,
};

#[derive(AssetCollection, Resource)]
pub struct GameAssets {}

pub struct GamePlugin<S: States> {
    loading_state: S,
    state: S,
}

impl<S: States> Plugin for GamePlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_loading_state(
            LoadingStateConfig::new(self.loading_state.clone()).load_collection::<GameAssets>(),
        )
        .add_systems(
            Update,
            (
                update_return_to_editor,
                update_character_orders,
                update_selection,
                debug_character_position_center,
            )
                .run_if(in_state(self.state.clone())),
        )
        .add_systems(OnEnter(self.state.clone()), setup_reset_camera_bounds);
    }
}

impl<S: States> GamePlugin<S> {
    pub fn run_on_state(state: S, loading_state: S) -> Self {
        Self {
            state,
            loading_state,
        }
    }
}

#[derive(Component)]
pub struct CharacterSelected;

fn update_return_to_editor(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::InEditor);
    }
}

fn setup_reset_camera_bounds(mut camera_q: Query<&mut Camera, With<MainCamera>>) {
    for mut camera in camera_q.iter_mut() {
        camera.viewport = None;
    }
}

fn update_selection(
    mut cmds: Commands,
    window_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    characters_q: Query<(Entity, &GlobalTransform), With<Character>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        let Ok(window) = window_q.get_single() else {
            return;
        };
        let Ok((camera, camera_transform)) = camera_q.get_single() else {
            return;
        };
        let Some(cursor_pos) = window.cursor_position() else {
            return;
        };
        if let Some(world_cursor_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            let mut closest: Option<Entity> = None;
            let mut closest_distance = f32::MAX;
            if !keyboard_input.pressed(KeyCode::ShiftLeft) {
                for (entity, _) in &characters_q {
                    if let Some(mut deselect) = cmds.get_entity(entity) {
                        deselect.remove::<CharacterSelected>();
                    }
                }
            }
            for (entity, character_pos) in &characters_q {
                // easy but bad, the way we'll do it is actually by first checking if
                // https://github.com/aevyrie/bevy_mod_picking/blob/main/backends/bevy_picking_sprite/src/lib.rs
                // we're in the rect of the sprite. then we'll get the texture data
                // from the sprite and convert that into a mask of 0's and 1's
                // from there we can check if the cursor is in the mask.
                // - we need to convert from logical to physical pixels first.
                let distance = character_pos
                    .translation()
                    .truncate()
                    .distance(world_cursor_pos)
                    .abs();
                if distance < 64.0 && closest_distance > distance {
                    closest = Some(entity);
                    closest_distance = distance;
                }
            }
            if let Some(closest) = closest {
                if let Some(mut selected) = cmds.get_entity(closest) {
                    selected.insert(CharacterSelected);
                }
            }
        }
    }
}

fn debug_character_position_center(
    mut character_q: Query<&Transform, With<CharacterSelected>>,
    mut gizmos: Gizmos,
) {
    for transform in character_q.iter_mut() {
        gizmos.ellipse_2d(
            transform.translation.truncate(),
            0.,
            Vec2::new(16.0, 14.0),
            Color::GREEN,
        );
    }
}

fn update_character_orders(
    window_q: Query<&Window>,
    camera_q: Query<(&Camera, &mut GlobalTransform), With<MainCamera>>,
    mut goal_q: Query<(&mut Goal, &Transform), With<CharacterSelected>>,
    navigation: Res<Navigation>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut gizmos: Gizmos,
) {
    let Ok(window) = window_q.get_single() else {
        return;
    };
    for (camera, camera_transform) in camera_q.iter() {
        let Some(cursor_pos) = window.cursor_position() else {
            return;
        };
        let Some(world_cursor_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos)
        else {
            return;
        };
        for (mut goal, transform) in goal_q.iter_mut() {
            if mouse_button.just_pressed(MouseButton::Right)
                && keyboard_input.pressed(KeyCode::ShiftLeft)
            {
                if let Some(Target::Position(pos)) = goal.path.back() {
                    let path: Vec<_> = navigation
                        .path_between_2d(*pos, world_cursor_pos)
                        .into_iter()
                        .map(|pos| Target::Position(pos))
                        .collect();
                    goal.extend(path);
                } else {
                    let path: Vec<_> = navigation
                        .path_between_2d(transform.translation.truncate(), world_cursor_pos)
                        .into_iter()
                        .map(|pos| Target::Position(pos))
                        .collect();
                    goal.extend(path);
                }
            } else if mouse_button.just_pressed(MouseButton::Right) {
                goal.clear();
                let path: Vec<_> = navigation
                    .path_between_2d(transform.translation.truncate(), world_cursor_pos)
                    .into_iter()
                    .map(|pos| Target::Position(pos))
                    .collect();
                goal.extend(path);
            }
        }
    }
    if keyboard_input.pressed(KeyCode::ShiftLeft) {
        for (goal, mover) in goal_q.iter() {
            if let Target::Position(target_pos) = goal.target {
                gizmos.line_2d(mover.translation.truncate(), target_pos, Color::WHITE);
                if let Some(Target::Position(first_pos)) = goal.path.front() {
                    gizmos.line_2d(target_pos, *first_pos, Color::WHITE);
                }
            }
            gizmos.linestrip_2d(
                goal.path.iter().filter_map(|target| match target {
                    Target::Position(pos) => Some(pos.clone()),
                    _ => None,
                }),
                Color::WHITE,
            );
        }
    }
}
