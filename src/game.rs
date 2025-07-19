use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

use crate::{
    camera::MainCamera,
    characters::{Character, CharacterActions},
    flowfield::{FlowFieldActor, FlowFieldDebugging},
    InGameState,
};

#[derive(AssetCollection, Resource)]
pub struct GameAssets {}

pub struct GamePlugin<S: States, L: States> {
    loading_state: L,
    state: S,
}

impl<
        S: States + bevy::state::state::FreelyMutableState,
        L: States + bevy::state::state::FreelyMutableState,
    > Plugin for GamePlugin<S, L>
{
    fn build(&self, app: &mut App) {
        app.configure_loading_state(
            LoadingStateConfig::new(self.loading_state.clone()).load_collection::<GameAssets>(),
        )
        .add_systems(
            Update,
            (
                update_return_to_editor,
                update_character_orders_flowfield,
                update_selection,
                update_character_state,
                debug_character_position_center,
            )
                .run_if(in_state(self.state.clone())),
        )
        .add_systems(OnEnter(self.state.clone()), setup_reset_camera_bounds);
    }
}

impl<S: States, L: States> GamePlugin<S, L> {
    pub fn run_on_state(state: S, loading_state: L) -> Self {
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
    mut next_state: ResMut<NextState<InGameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(InGameState::InEditor);
    }
}

fn setup_reset_camera_bounds(mut camera_q: Query<&mut Camera, With<MainCamera>>) {
    for mut camera in camera_q.iter_mut() {
        camera.viewport = None;
    }
}

//todo: use bevy picking
fn update_selection(
    mut cmds: Commands,
    window_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    characters_q: Query<(Entity, &GlobalTransform), With<Character>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        let Ok(window) = window_q.single() else {
            return;
        };
        let Ok((camera, camera_transform)) = camera_q.single() else {
            return;
        };
        let Some(cursor_pos) = window.cursor_position() else {
            return;
        };
        if let Ok(world_cursor_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            let mut closest: Option<Entity> = None;
            let mut closest_distance = f32::MAX;
            if !keyboard_input.pressed(KeyCode::ShiftLeft) {
                for (entity, _) in &characters_q {
                    if let Ok(mut deselect) = cmds.get_entity(entity) {
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
                if let Ok(mut selected) = cmds.get_entity(closest) {
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
            Vec2::new(16.0, 14.0),
            bevy::color::palettes::css::GREEN,
        );
    }
}

fn update_character_state(
    mut cmds: Commands,
    mut state_q: Query<(Entity, &FlowFieldActor, &mut CharacterActions, &Transform)>,
) {
    for (entity, actor, mut state, transform) in state_q.iter_mut() {
        match *state {
            CharacterActions::Standing => (),
            CharacterActions::Moving { ref mut direction } => {
                let at_destination = actor
                    .target
                    .abs_diff_eq(transform.translation.truncate(), 0.5);
                if at_destination {
                    *state = CharacterActions::Standing;
                    cmds.entity(entity).remove::<FlowFieldActor>();
                } else {
                    *direction = actor.steering;
                }
            }
            // if we're attacking we stop moving?
            CharacterActions::Attacking { direction, entity } => (),
        }
    }
}

fn update_character_orders_flowfield(
    mut cmds: Commands,
    window_q: Query<&Window>,
    camera_q: Query<(&Camera, &mut GlobalTransform), With<MainCamera>>,
    selected_q: Query<Entity, With<CharacterSelected>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
) {
    let Ok(window) = window_q.single() else {
        return;
    };
    for (camera, camera_transform) in camera_q.iter() {
        let Some(cursor_pos) = window.cursor_position() else {
            return;
        };
        let Ok(world_cursor_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
            return;
        };
        if mouse_button.just_pressed(MouseButton::Right) {
            for entity in selected_q {
                cmds.entity(entity).insert((
                    FlowFieldDebugging,
                    FlowFieldActor::new(world_cursor_pos),
                    CharacterActions::moving(),
                ));
            }
        }
    }
}
