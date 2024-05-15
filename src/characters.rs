use bevy::{prelude::*, sprite::Anchor, utils::HashMap};
use bevy_asset_loader::prelude::*;
use std::{collections::VecDeque, time::Duration};

use crate::world::TILE_SIZE;

pub const ANIMATION_SPEED: Duration = Duration::from_millis(100);

#[derive(AssetCollection, Resource)]
pub struct CharacterAssets {
    #[asset(path = "factions/knights/troops/pawn/blue/pawn.png")]
    pub pawn_texture: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 192., tile_size_y = 192., columns = 6, rows = 6))]
    pub pawn_layout: Handle<TextureAtlasLayout>,

    #[asset(path = "factions/goblins/troops/raider/red/raider_red.png")]
    pub raider_texture: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 192., tile_size_y = 192., columns = 7, rows = 6))]
    pub raider_layout: Handle<TextureAtlasLayout>,

    #[asset(path = "deco/knights_sign.png")]
    pub target_sign: Handle<Image>,
}

impl CharacterAssets {
    pub fn pawn(&self, xy: Vec2) -> CharacterBundle {
        let sprite_sheet = SpriteSheetBundle {
            sprite: Sprite {
                flip_x: true,
                // todo: This can be custom
                anchor: Anchor::Custom(Vec2::new(0.0, -0.15)),
                ..default()
            },
            texture: self.pawn_texture.clone(),
            transform: Transform::from_translation(xy.extend(128.)),
            atlas: TextureAtlas {
                layout: self.pawn_layout.clone(),
                index: 0,
            },
            ..default()
        };
        let mut animation = Animation::default();
        animation.clip_book.insert(String::from("default"), (0, 6));
        animation.clip_book.insert(String::from("walk"), (6, 11));
        animation.clip_book.insert(String::from("build"), (11, 16));
        CharacterBundle {
            id: Character { id: 0 },
            stats: Stats {
                speed_in_pixels_per_second: TILE_SIZE,
            },
            target: Goal {
                target: Target::None,
                path: VecDeque::new(),
            },
            sprite_sheet,
            animation,
        }
    }

    pub fn raider(&self, xy: Vec2) -> CharacterBundle {
        let sprite_sheet = SpriteSheetBundle {
            sprite: Sprite {
                flip_x: true,
                anchor: Anchor::Custom(Vec2::new(0.0, -0.15)),
                ..default()
            },
            texture: self.raider_texture.clone(),
            transform: Transform::from_translation(xy.extend(128.)),
            atlas: TextureAtlas {
                layout: self.raider_layout.clone(),
                index: 0,
            },
            ..default()
        };
        let mut animation = Animation::default();
        animation.clip_book.insert(String::from("default"), (1, 7));
        animation.clip_book.insert(String::from("walk"), (7, 13));
        animation.clip_book.insert(String::from("attack"), (13, 18));
        animation
            .clip_book
            .insert(String::from("attack_down"), (18, 23));
        animation
            .clip_book
            .insert(String::from("attack_up"), (23, 28));
        CharacterBundle {
            id: Character { id: 0 },
            stats: Stats {
                speed_in_pixels_per_second: TILE_SIZE,
            },
            target: Goal {
                target: Target::None,
                path: VecDeque::new(),
            },
            sprite_sheet,
            animation,
        }
    }
}

pub struct CharacterPlugin<S: States> {
    state: S,
    or_state: S,
    loading_state: S,
}

impl<S: States> Plugin for CharacterPlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_loading_state(
            LoadingStateConfig::new(self.loading_state.clone())
                .load_collection::<CharacterAssets>(),
        )
        .register_type::<Character>()
        .register_type::<Animation>()
        .register_type::<Stats>()
        .register_type::<Goal>()
        .add_systems(
            OnTransition {
                from: self.loading_state.clone(),
                to: self.state.clone(),
            },
            setup_characters,
        )
        .add_systems(
            OnTransition {
                from: self.loading_state.clone(),
                to: self.or_state.clone(),
            },
            setup_characters,
        )
        .add_systems(
            Update,
            (update_character_movement, update_animated_characters)
                .run_if(in_state(self.state.clone()).or_else(in_state(self.or_state.clone()))),
        );
    }
}

impl<S: States> CharacterPlugin<S> {
    pub fn run_on_state_or(state: S, or_state: S, loading_state: S) -> Self {
        Self {
            state,
            or_state,
            loading_state,
        }
    }
}

#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct Stats {
    pub speed_in_pixels_per_second: f32,
}

#[derive(Debug, PartialEq, Clone, Reflect)]
pub enum Target {
    Entity(Entity),
    Position(Vec2),
    None,
}

impl Default for Target {
    fn default() -> Self {
        Target::None
    }
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Goal {
    #[reflect(skip_serializing)]
    pub target: Target,
    #[reflect(skip_serializing)]
    pub path: VecDeque<Target>,
}

impl Default for Goal {
    fn default() -> Self {
        Self {
            target: Target::None,
            path: VecDeque::new(),
        }
    }
}

impl Goal {
    pub fn add_target(&mut self, target: Target) {
        self.path.push_back(target);
    }

    pub fn extend(&mut self, path: Vec<Target>) {
        for target in path {
            self.path.push_back(target);
        }
    }

    pub fn clear(&mut self) {
        self.path.clear();
    }
}

#[derive(Component, Clone, Reflect)]
pub struct Animation {
    #[reflect(skip_serializing)]
    timer: Timer,
    #[reflect(skip_serializing)]
    frame: usize,
    #[reflect(skip_serializing)]
    current_animation: String,
    clip_book: HashMap<String, (u8, u8)>,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            timer: Timer::new(ANIMATION_SPEED, TimerMode::Repeating),
            frame: 0,
            current_animation: String::from("default"),
            clip_book: HashMap::new(),
        }
    }
}

#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct Character {
    pub id: u8,
}

#[derive(Bundle, Clone)]
pub struct CharacterBundle {
    pub id: Character,
    pub stats: Stats,
    pub target: Goal,
    pub sprite_sheet: SpriteSheetBundle,
    pub animation: Animation,
}

impl CharacterBundle {}

fn setup_characters(cmds: Commands, assets: Res<CharacterAssets>) {
    // todo: I guess load from a map? Or something?
}

fn update_character_movement(
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

fn update_animated_characters(
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
