use bevy::{prelude::*, sprite::Anchor};
use bevy_asset_loader::prelude::*;
use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};

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
    pub fn pawn(&self) -> AnimatedSpriteBundle {
        let mut sprite_sheet = Sprite::from_atlas_image(
            self.pawn_texture.clone(),
            TextureAtlas {
                layout: self.pawn_layout.clone(),
                index: 0,
            },
        );
        sprite_sheet.flip_x = true;
        sprite_sheet.anchor = Anchor::Center;
        let mut animation = Animation::default();
        animation.clip_book.insert(String::from("default"), (0, 6));
        animation.clip_book.insert(String::from("walk"), (6, 11));
        animation.clip_book.insert(String::from("build"), (11, 16));
        AnimatedSpriteBundle {
            sprite_sheet,
            animation,
        }
    }

    pub fn raider(&self) -> AnimatedSpriteBundle {
        let mut sprite = Sprite::from_atlas_image(
            self.raider_texture.clone(),
            TextureAtlas {
                layout: self.raider_layout.clone(),
                index: 0,
            },
        );
        sprite.flip_x = true;
        sprite.anchor = Anchor::Center;
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
        AnimatedSpriteBundle {
            sprite_sheet: sprite,
            animation,
        }
    }
}

pub struct CharacterPlugin<S: States> {
    state: S,
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
                Update,
                (
                    // update_character_movement,
                    update_handle_actions,
                    update_animated_characters,
                    on_added_insert_visuals,
                )
                    .run_if(in_state(self.state.clone())),
            );
    }
}

impl<S: States> CharacterPlugin<S> {
    pub fn run_on_state(state: S, loading_state: S) -> Self {
        Self {
            state,
            loading_state,
        }
    }
}

#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct Stats {
    pub speed_in_pixels_per_second: f32,
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            speed_in_pixels_per_second: TILE_SIZE,
        }
    }
}

// simple state machine for our characters
#[derive(Component, Debug)]
pub enum CharacterActions {
    Standing,
    Moving { direction: Vec2 },
    // the feeling here is when the unit attacks we get it's attack range
    // and use that to decide when to switch to attacking
    // i.e if we're outside of attacking range we change the characters state to moving
    // and vice versa, so when we're in moving state and we enter attack range we switch to
    // attacking
    Attacking { direction: Vec2, entity: Entity },
}

impl CharacterActions {
    pub fn standing() -> Self {
        Self::Standing
    }

    pub fn moving() -> Self {
        Self::Moving {
            direction: Vec2::ZERO,
        }
    }
}

impl Default for CharacterActions {
    fn default() -> Self {
        CharacterActions::Standing
    }
}

#[derive(Component, Debug, Clone, Default)]
pub struct Moving {
    pub direction: Vec2,
    pub pixels_per_second: f32,
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
#[require(Transform, Stats, Pickable, CharacterActions)]
pub enum Character {
    Pawn,
    Raider,
}
impl Character {
    pub fn animated_sprite(&self, character_assets: &CharacterAssets) -> AnimatedSpriteBundle {
        match self {
            Character::Pawn => character_assets.pawn(),
            Character::Raider => character_assets.raider(),
        }
    }
}

fn on_added_insert_visuals(
    mut commands: Commands,
    query: Query<(Entity, &Character), (Added<Character>, Without<Sprite>, Without<Animation>)>,
    assets: Res<CharacterAssets>,
) {
    for (entity, character) in &query {
        let bundle = character.animated_sprite(&assets);
        commands.entity(entity).insert(bundle);
    }
}

#[derive(Bundle, Clone)]
pub struct AnimatedSpriteBundle {
    pub animation: Animation,
    pub sprite_sheet: Sprite,
}

fn setup_characters(cmds: Commands, assets: Res<CharacterAssets>) {
    // todo: I guess load from a map? Or something?
}

fn update_handle_actions(
    time: Res<Time>,
    mut state_q: Query<(
        &CharacterActions,
        &Stats,
        &mut Transform,
        &mut Animation,
        &mut Sprite,
    )>,
) {
    for (state, stats, mut transform, mut animation, mut sprite) in state_q.iter_mut() {
        match state {
            CharacterActions::Standing => animation.current_animation = "default".to_string(),
            CharacterActions::Moving { direction } => {
                animation.current_animation = "walk".to_string();
                let magnitude = time.delta().as_secs_f32() * stats.speed_in_pixels_per_second;
                let move_by = direction * magnitude;
                transform.translation += move_by.extend(0.);
                if direction.x < 0. {
                    sprite.flip_x = true;
                } else {
                    sprite.flip_x = false;
                }
            }
            CharacterActions::Attacking { direction, entity } => todo!(),
        }
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
