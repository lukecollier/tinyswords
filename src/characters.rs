use bevy::{prelude::*, sprite::Anchor, utils::HashMap};
use bevy_asset_loader::prelude::*;
use std::{collections::VecDeque, time::Duration};

use crate::world::TILE_SIZE;

pub const ANIMATION_SPEED: Duration = Duration::from_millis(100);

#[derive(AssetCollection, Resource)]
pub struct CharacterAssets {
    #[asset(path = "factions/knights/troops/pawn/blue/pawn.png")]
    pub pawn_texture: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 192, tile_size_y = 192, columns = 6, rows = 6))]
    pub pawn_layout: Handle<TextureAtlasLayout>,

    #[asset(path = "factions/goblins/troops/raider/red/raider_red.png")]
    pub raider_texture: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 192, tile_size_y = 192, columns = 7, rows = 6))]
    pub raider_layout: Handle<TextureAtlasLayout>,

    #[asset(path = "deco/knights_sign.png")]
    pub target_sign: Handle<Image>,
}

impl CharacterAssets {
    pub fn pawn(&self, xy: Vec2) -> CharacterBundle {
        let mut sprite_sheet = Sprite::from_atlas_image(
            self.pawn_texture.clone(),
            TextureAtlas {
                layout: self.pawn_layout.clone(),
                index: 0,
            },
        );
        sprite_sheet.flip_x = true;
        sprite_sheet.anchor = Anchor::Custom(Vec2::new(0.0, -0.15));
        let mut animation = Animation::default();
        animation.clip_book.insert(String::from("default"), (0, 6));
        animation.clip_book.insert(String::from("walk"), (6, 11));
        animation.clip_book.insert(String::from("build"), (11, 16));
        CharacterBundle {
            id: Character::Pawn,
            stats: Stats {
                speed_in_pixels_per_second: TILE_SIZE,
            },
            target: Goal::default(),
            transform: Transform::from_translation(xy.extend(128.)),
            sprite_sheet,
            animation,
        }
    }

    pub fn raider(&self, xy: Vec2) -> CharacterBundle {
        let mut sprite = Sprite::from_atlas_image(
            self.raider_texture.clone(),
            TextureAtlas {
                layout: self.raider_layout.clone(),
                index: 0,
            },
        );
        sprite.flip_x = true;
        sprite.anchor = Anchor::Custom(Vec2::new(0.0, -0.15));
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
            id: Character::Raider,
            stats: Stats {
                speed_in_pixels_per_second: TILE_SIZE,
            },
            transform: Transform::from_translation(xy.extend(128.)),
            target: Goal::default(),
            sprite_sheet: sprite,
            animation,
        }
    }
}

pub struct CharacterPlugin<S: States> {
    state: S,
    or_state: S,
    loading_state: S,
}

impl<S: States + bevy::state::state::FreelyMutableState> Plugin for CharacterPlugin<S> {
    fn build(&self, app: &mut App) {
        app.register_type::<Character>()
            .configure_loading_state(
                LoadingStateConfig::new(self.loading_state.clone())
                    .load_collection::<CharacterAssets>(),
            )
            .add_systems(
                OnTransition {
                    exited: self.loading_state.clone(),
                    entered: self.state.clone(),
                },
                setup_characters,
            )
            .add_systems(
                OnTransition {
                    exited: self.loading_state.clone(),
                    entered: self.or_state.clone(),
                },
                setup_characters,
            )
            .add_systems(
                Update,
                (
                    update_character_movement,
                    update_animated_characters,
                    on_added_insert_visuals,
                )
                    .run_if(in_state(self.state.clone()).or(in_state(self.or_state.clone()))),
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

#[derive(Debug, PartialEq, Clone)]
pub enum Target {
    Entity(Entity),
    // todo: Should use Vec3
    Position(Vec2),
    None,
}

impl Default for Target {
    fn default() -> Self {
        Target::None
    }
}

#[derive(Component, Debug, Clone)]
pub struct Goal {
    pub target: Target,
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

#[derive(Component, Clone)]
pub struct Animation {
    timer: Timer,
    frame: usize,
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

#[derive(Component, Eq, PartialEq, Clone, Copy, Reflect, Debug)]
#[reflect(Component)]
pub enum Character {
    Pawn,
    Raider,
}
impl Character {
    pub fn bundle(&self, character_assets: &CharacterAssets, xy: Vec2) -> CharacterBundle {
        match self {
            Character::Pawn => character_assets.pawn(xy),
            Character::Raider => character_assets.raider(xy),
        }
    }
}

fn on_added_insert_visuals(
    mut commands: Commands,
    query: Query<
        (Entity, &Character, &Transform),
        (Added<Character>, Without<Sprite>, Without<Animation>),
    >,
    assets: Res<CharacterAssets>,
) {
    for (entity, character, transform) in &query {
        let bundle = character.bundle(&assets, transform.translation.truncate());
        commands
            .entity(entity)
            .insert((bundle.sprite_sheet, bundle.animation));
    }
}

// todo: Deprecate and move to require macro
#[derive(Bundle, Clone)]
pub struct CharacterBundle {
    pub id: Character,
    pub stats: Stats,
    pub target: Goal,
    pub sprite_sheet: Sprite,
    pub transform: Transform,
    pub animation: Animation,
}

#[derive(Bundle, Clone)]
pub struct AnimatedSprite {
    pub animation: Animation,
    pub sprite_sheet: Sprite,
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
                // todo: Change the z depending on the height of the character
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
    mut animated_q: Query<(&mut Sprite, &mut Animation)>,
    time: Res<Time>,
) {
    for (mut sprite, mut animated) in &mut animated_q {
        if let Some(ref mut texture_atlas) = sprite.texture_atlas {
            if animated.frame > usize::MAX {
                animated.frame = 0;
            }
            animated.timer.tick(time.delta());
            if animated.timer.finished() {
                animated.frame += 1;
            }
            if let Some((lower, upper)) =
                animated.clip_book.get(&animated.current_animation).clone()
            {
                texture_atlas.index =
                    *lower as usize + (animated.frame % (*upper - *lower) as usize);
            }
        }
    }
}
