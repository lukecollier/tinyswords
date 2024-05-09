use bevy::{prelude::*, sprite::Anchor, utils::HashMap};
use bevy_asset_loader::prelude::*;
use std::{collections::VecDeque, time::Duration};

use crate::{camera::MainCamera, world::TILE_SIZE, GameState};

pub const ANIMATION_SPEED: Duration = Duration::from_millis(100);

#[derive(AssetCollection, Resource)]
pub struct UnitAssets {
    #[asset(path = "factions/knights/troops/pawn/blue/pawn.png")]
    pub pawn_texture: Handle<Image>,

    #[asset(texture_atlas_layout(tile_size_x = 192., tile_size_y = 192., columns = 5, rows = 5))]
    pub pawn_layout: Handle<TextureAtlasLayout>,

    #[asset(path = "deco/knights_sign.png")]
    pub target_sign: Handle<Image>,
}

pub struct UnitPlugin<S: States> {
    state: S,
}

impl<S: States> Plugin for UnitPlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_loading_state(
            LoadingStateConfig::new(GameState::AssetLoading).load_collection::<UnitAssets>(),
        )
        .add_systems(OnEnter(self.state.clone()), (setup_units))
        .add_systems(
            Update,
            (
                update_unit_movement,
                update_animated_units,
                debug_unit_movement,
            )
                .run_if(in_state(self.state.clone())),
        );
    }
}

impl<S: States> UnitPlugin<S> {
    pub fn run_on_state(state: S) -> Self {
        Self { state }
    }
}

#[derive(Component)]
struct Stats {
    speed_in_pixels_per_second: f32,
}

#[derive(Debug, PartialEq)]
enum Target {
    Entity(Entity),
    Position(Vec2),
    None,
}

#[derive(Component, Debug)]
struct Goal {
    target: Target,
    path: VecDeque<Target>,
}

impl Goal {
    fn add_target(&mut self, target: Target) {
        self.path.push_back(target);
    }
}

#[derive(Component)]
struct Animation {
    timer: Timer,
    frame: usize,
    current_animation: String,
    clip_book: HashMap<String, (u8, u8)>,
}

impl Default for Animation {
    fn default() -> Self {
        let mut default_clipbook = HashMap::with_capacity(3);
        Self {
            timer: Timer::new(ANIMATION_SPEED, TimerMode::Repeating),
            frame: 0,
            current_animation: String::from("default"),
            clip_book: default_clipbook,
        }
    }
}

impl Animation {
    fn pawn() -> Self {
        let mut default = Animation::default();
        default.clip_book.insert(String::from("default"), (0, 5));
        default.clip_book.insert(String::from("walk"), (5, 10));
        default.clip_book.insert(String::from("build"), (10, 15));
        default
    }
}

#[derive(Bundle)]
struct UnitBundle {
    stats: Stats,
    target: Goal,
    sprite_sheet: SpriteSheetBundle,
    animation: Animation,
}

fn setup_units(mut cmds: Commands, assets: Res<UnitAssets>) {
    let pawn = SpriteSheetBundle {
        sprite: Sprite {
            flip_x: true,
            anchor: Anchor::Center,
            ..default()
        },
        texture: assets.pawn_texture.clone(),
        transform: Transform::from_xyz(64., 64., 128.),
        atlas: TextureAtlas {
            layout: assets.pawn_layout.clone(),
            index: 0,
        },
        ..default()
    };
    cmds.spawn(UnitBundle {
        stats: Stats {
            speed_in_pixels_per_second: TILE_SIZE,
        },
        target: Goal {
            target: Target::None,
            path: VecDeque::new(),
        },
        sprite_sheet: pawn,
        animation: Animation::pawn(),
    });
}

fn update_unit_movement(
    mut goal_q: Query<(
        &Stats,
        &mut Transform,
        &mut Goal,
        &mut Sprite,
        &mut Animation,
    )>,
    time: Res<Time>,
) {
    for (stats, mut transform, mut goal, mut sprite, mut animation) in goal_q.iter_mut() {
        match goal.target {
            Target::Entity(_) => {}
            Target::Position(position) => {
                animation.current_animation = String::from("walk");
                let magnitude = time.delta().as_secs_f32() * stats.speed_in_pixels_per_second;
                let direction = position.extend(transform.translation.z) - transform.translation;
                *transform = Transform::from_translation(
                    transform.translation + direction.normalize() * magnitude,
                );
                // Make the sprite face the direction it's moving
                if position.x < transform.translation.x {
                    sprite.flip_x = true;
                } else {
                    sprite.flip_x = false;
                }

                if position.distance(transform.translation.truncate()) < magnitude {
                    goal.target = Target::None;
                }
            }
            Target::None => {
                if let Some(next_target) = goal.path.pop_front() {
                    goal.target = next_target;
                } else {
                    animation.current_animation = String::from("default");
                }
            }
        };
    }
}

fn debug_unit_movement(
    window_q: Query<&Window>,
    camera_q: Query<(&Camera, &mut GlobalTransform), With<MainCamera>>,
    mut goal_q: Query<(&mut Goal, &Transform)>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut gizmos: Gizmos,
) {
    if let Ok(window) = window_q.get_single() {
        for (camera, camera_transform) in camera_q.iter() {
            if let Some(cursor_pos) = window.cursor_position() {
                if let Some(world_cursor_pos) =
                    camera.viewport_to_world_2d(camera_transform, cursor_pos)
                {
                    for (mut goal, _) in goal_q.iter_mut() {
                        if mouse_button.just_pressed(MouseButton::Right)
                            && keyboard_input.pressed(KeyCode::ShiftLeft)
                        {
                            goal.add_target(Target::Position(world_cursor_pos));
                        } else if mouse_button.just_pressed(MouseButton::Right) {
                            goal.target = Target::Position(world_cursor_pos);
                        }
                    }
                }
            }
        }
    }
    if keyboard_input.pressed(KeyCode::ShiftLeft) {
        for (goal, mover) in goal_q.iter() {
            if let Target::Position(target_pos) = goal.target {
                gizmos.line_2d(mover.translation.truncate(), target_pos, Color::ORANGE_RED);
                if let Some(Target::Position(first_pos)) = goal.path.front() {
                    gizmos.line_2d(target_pos, *first_pos, Color::SEA_GREEN);
                }
            }
            gizmos.linestrip_2d(
                goal.path.iter().filter_map(|target| match target {
                    Target::Position(pos) => Some(pos.clone()),
                    _ => None,
                }),
                Color::SEA_GREEN,
            );
        }
    }
}

fn update_animated_units(
    mut animated_q: Query<(&mut TextureAtlas, &mut Animation)>,
    time: Res<Time>,
) {
    for (mut texture_atlas, mut animated) in &mut animated_q {
        if animated.frame > usize::MAX {
            animated.frame = 0;
        }
        animated.timer.tick(time.delta());
        if animated.timer.finished() {
            animated.frame += 1;
        }
        if let Some((lower, upper)) = animated.clip_book.get(&animated.current_animation).clone() {
            texture_atlas.index = *lower as usize + (animated.frame % (*upper - *lower) as usize);
        }
    }
}
