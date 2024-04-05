use bevy::prelude::*;
/**
 * This is the plugin for the world, it's animations, and creating blocking
 */
use bevy::{
    math::U16Vec2,
    sprite::Anchor,
    text::{BreakLineOn, Text2dBounds},
};
use bevy_asset_loader::prelude::*;

use crate::{camera::GameCamera, GameState};

pub const WORLD_SIZE: U16Vec2 = U16Vec2::new(64, 64);
pub const TILE_SIZE: f32 = 64.0;
pub const TILE_VEC: Vec2 = Vec2::new(TILE_SIZE, TILE_SIZE);

// todo: Use bitmask crate https://docs.rs/bitmask/latest/bitmask/
const BITMASK_NONE: u8 = 0;
const BITMASK_TOP: u8 = 1;
const BITMASK_LEFT: u8 = 2;
const BITMASK_RIGHT: u8 = 4;
const BITMASK_BOT: u8 = 8;

const BITMASK_TOP_LEFT: u8 = 3;
const BITMASK_TOP_RIGHT: u8 = 5;
const BITMASK_HORIZONTAL: u8 = 6;
const BITMASK_TOP_LEFT_RIGHT: u8 = 7;
const BITMASK_VERTICAL: u8 = 9;
const BITMASK_BOT_LEFT: u8 = 10;
const BITMASK_BOT_TOP_LEFT: u8 = 11;
const BITMASK_BOT_RIGHT: u8 = 12;
const BITMASK_BOT_TOP_RIGHT: u8 = 13;
const BITMASK_BOT_LEFT_RIGHT: u8 = 14;
const BITMASK_CENTER: u8 = 15;

#[derive(Debug)]
enum Precedence {
    Water,
    Sand,
}

impl Precedence {
    fn to_z(&self) -> f32 {
        match self {
            Precedence::Water => 0.0,
            Precedence::Sand => 1.0,
        }
    }
}

#[derive(AssetCollection, Resource)]
struct WorldAssets {
    #[asset(path = "terrain/water/water.png")]
    water_texture: Handle<Image>,
    #[asset(path = "terrain/water/foam/foam.png")]
    foam: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 64., tile_size_y = 64., columns = 10, rows = 4))]
    ground_layout: Handle<TextureAtlasLayout>,
    #[asset(path = "terrain/ground/tilemap_land.png")]
    ground_texture: Handle<Image>,
}

impl WorldAssets {
    // sand index's
    const SAND: usize = 38;
    const SAND_LEFT: usize = 37;
    const SAND_HORIZONTAL: usize = 36;
    const SAND_RIGHT: usize = 35;
    const SAND_TOP_LEFT: usize = 5;
    const SAND_TOP_CENTRE: usize = 6;
    const SAND_TOP_RIGHT: usize = 7;
    const SAND_TOP: usize = 28;
    const SAND_CRUMBS: usize = 9;
    const SAND_CENTER_LEFT: usize = 15;
    const SAND_CENTER: usize = 16;
    const SAND_CENTER_RIGHT: usize = 17;
    const SAND_VERTICAL: usize = 18;
    const SAND_BOT_LEFT: usize = 25;
    const SAND_BOT_CENTRE: usize = 26;
    const SAND_BOT_RIGHT: usize = 27;
    const SAND_BOT: usize = 8;

    fn water(&self, xy: Vec2) -> SpriteBundle {
        let texture = self.water_texture.clone();
        SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture,
            transform: Transform::from_translation(xy.extend(Precedence::Water.to_z())),
            ..default()
        }
    }

    fn sand_index_from_bitmask(&self, bitmask: u8) -> usize {
        match bitmask {
            BITMASK_LEFT => Self::SAND_LEFT,
            BITMASK_RIGHT => Self::SAND_RIGHT,
            BITMASK_HORIZONTAL => Self::SAND_HORIZONTAL,
            BITMASK_VERTICAL => Self::SAND_VERTICAL,
            BITMASK_CENTER => Self::SAND_CENTER,
            BITMASK_BOT => Self::SAND_BOT,
            BITMASK_TOP => Self::SAND_TOP,
            // todo: Fix a naming convention, are we refering to the open connections? Makes sense
            BITMASK_BOT_TOP_RIGHT => Self::SAND_CENTER_LEFT,
            BITMASK_BOT_TOP_LEFT => Self::SAND_CENTER_RIGHT,
            BITMASK_BOT_LEFT_RIGHT => Self::SAND_TOP_CENTRE,
            BITMASK_TOP_LEFT_RIGHT => Self::SAND_BOT_CENTRE,
            BITMASK_BOT_RIGHT => Self::SAND_TOP_LEFT,
            BITMASK_BOT_LEFT => Self::SAND_TOP_RIGHT,
            BITMASK_TOP_RIGHT => Self::SAND_BOT_LEFT,
            BITMASK_TOP_LEFT => Self::SAND_BOT_RIGHT,
            _ => Self::SAND,
        }
    }

    fn bitmask_from_sand_index(&self, idx: usize) -> u8 {
        match idx {
            Self::SAND_LEFT => BITMASK_LEFT,
            Self::SAND_RIGHT => BITMASK_RIGHT,
            Self::SAND_HORIZONTAL => BITMASK_HORIZONTAL,
            Self::SAND_VERTICAL => BITMASK_VERTICAL,
            Self::SAND_CENTER => BITMASK_CENTER,
            Self::SAND_TOP => BITMASK_TOP,
            Self::SAND_BOT => BITMASK_BOT,
            Self::SAND_CENTER_LEFT => BITMASK_BOT_TOP_RIGHT,
            Self::SAND_CENTER_RIGHT => BITMASK_BOT_TOP_LEFT,
            Self::SAND_TOP_CENTRE => BITMASK_BOT_LEFT_RIGHT,
            Self::SAND_BOT_CENTRE => BITMASK_TOP_LEFT_RIGHT,
            Self::SAND_TOP_LEFT => BITMASK_BOT_RIGHT,
            Self::SAND_TOP_RIGHT => BITMASK_BOT_LEFT,
            Self::SAND_BOT_LEFT => BITMASK_TOP_RIGHT,
            Self::SAND_BOT_RIGHT => BITMASK_TOP_LEFT,
            _ => BITMASK_NONE,
        }
    }

    fn tile_from(&self, idx: u8, xy: Vec2, level: Precedence) -> SpriteSheetBundle {
        let layout = self.ground_layout.clone();
        let texture = self.ground_texture.clone();
        SpriteSheetBundle {
            sprite: Sprite {
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture,
            transform: Transform::from_translation(xy.extend(level.to_z())),
            atlas: TextureAtlas {
                layout,
                index: idx as usize,
            },
            ..default()
        }
    }
}

pub struct WorldPlugin<S: States> {
    state: S,
}

impl<S: States> Plugin for WorldPlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_loading_state(
            LoadingStateConfig::new(GameState::AssetLoading).load_collection::<WorldAssets>(),
        )
        .add_systems(OnEnter(self.state.clone()), setup_tile_system)
        .add_systems(
            Update,
            update_tile_system.run_if(in_state(self.state.clone())),
        );
        // note: When adding update states prefix with `.run_if(in_state(self.state.clone())))`
    }
}

impl<S: States> WorldPlugin<S> {
    pub fn run_on_state(state: S) -> Self {
        Self { state }
    }
}

#[derive(Component, Debug)]
struct Tile {
    pos: U16Vec2,
    precedence: Precedence,
}

impl Tile {
    fn empty(x: u16, y: u16) -> Self {
        Self {
            pos: U16Vec2::new(x, y),
            precedence: Precedence::Water,
        }
    }
    fn pixel_coordinates(&self, tile_size: f32) -> Vec2 {
        Vec2::new(self.pos.x as f32 * tile_size, self.pos.y as f32 * tile_size)
    }
}

fn check_neighbours_bitmask<'a, I>(tile_pos: U16Vec2, tiles_iter: I) -> u8
where
    I: Iterator<Item = &'a Tile>,
{
    let mut bitmask = 0;
    for tile in tiles_iter {
        // up
        if tile.pos == (tile_pos + U16Vec2::Y) {
            bitmask += 2_u32.pow(0);
        }
        // left
        if tile.pos == (tile_pos - U16Vec2::X) {
            bitmask += 2_u32.pow(1);
        }
        // right
        if tile.pos == (tile_pos + U16Vec2::X) {
            bitmask += 2_u32.pow(2);
        }
        // down
        if tile.pos == (tile_pos - U16Vec2::Y) {
            bitmask += 2_u32.pow(3);
        }
    }
    bitmask as u8
}

fn update_tile_system(
    mut cmds: Commands,
    window_q: Query<&Window>,
    camera_q: Query<(&Camera, &mut GlobalTransform), With<GameCamera>>,
    mut tiles_q: Query<(&mut TextureAtlas, &Tile)>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    assets: Res<WorldAssets>,
    mut gizmos: Gizmos,
) {
    if let Ok(window) = window_q.get_single() {
        for (camera, camera_transform) in camera_q.iter() {
            if let Some(cursor_pos) = window.cursor_position() {
                if let Some(world_cursor_pos) =
                    camera.viewport_to_world_2d(camera_transform, cursor_pos)
                {
                    let tile_pos = (world_cursor_pos / TILE_VEC).floor().as_u16vec2();
                    gizmos.rect_2d(
                        tile_pos.as_vec2() * TILE_VEC + TILE_VEC / 2.0,
                        0.0,
                        TILE_VEC,
                        Color::GREEN,
                    );

                    if mouse_button.just_pressed(MouseButton::Left)
                        && tiles_q
                            .iter()
                            .find(|(_, tile)| tile.pos == tile_pos)
                            .is_none()
                    {
                        let mut found = false;
                        let bit_mask_total = check_neighbours_bitmask(
                            tile_pos,
                            tiles_q.iter().map(|(_, tile)| tile),
                        );
                        let new_idx = assets.sand_index_from_bitmask(bit_mask_total);
                        for (mut texture_atlas, tile) in &mut tiles_q {
                            // right
                            if tile.pos == tile_pos + U16Vec2::X {
                                let bitmask_right = assets
                                    .bitmask_from_sand_index(texture_atlas.index)
                                    + BITMASK_LEFT;
                                texture_atlas.index = assets.sand_index_from_bitmask(bitmask_right);
                            }
                            // left
                            if tile.pos == tile_pos - U16Vec2::X {
                                let bitmask_left = assets
                                    .bitmask_from_sand_index(texture_atlas.index)
                                    + BITMASK_RIGHT;
                                texture_atlas.index = assets.sand_index_from_bitmask(bitmask_left);
                            }
                            // down
                            if tile.pos == tile_pos - U16Vec2::Y {
                                let bitmask_down = assets
                                    .bitmask_from_sand_index(texture_atlas.index)
                                    + BITMASK_TOP;
                                texture_atlas.index = assets.sand_index_from_bitmask(bitmask_down);
                            }
                            // up
                            if tile.pos == tile_pos + U16Vec2::Y {
                                let bitmask_up = assets
                                    .bitmask_from_sand_index(texture_atlas.index)
                                    + BITMASK_BOT;
                                texture_atlas.index = assets.sand_index_from_bitmask(bitmask_up);
                            }
                            if tile.pos == tile_pos {
                                found = true;
                                texture_atlas.index = new_idx;
                            }
                        }
                        if found == false {
                            let sprite = assets.tile_from(
                                new_idx as u8,
                                tile_pos.as_vec2() * TILE_VEC,
                                Precedence::Sand,
                            );
                            let tile = Tile::empty(tile_pos.x as u16, tile_pos.y as u16);
                            cmds.spawn((tile, sprite));
                        }
                    }
                }
            }
        }
    }
}

fn setup_tile_system(mut cmds: Commands, assets: Res<WorldAssets>) {
    for x in 0..WORLD_SIZE.x {
        for y in 0..WORLD_SIZE.y {
            let tile = Tile::empty(x, y);
            let coords = tile.pixel_coordinates(TILE_SIZE);
            let sprite = assets.water(coords);
            cmds.spawn((tile, sprite));
        }
    }
    for idx in 0..(4 * 10) {
        let tile = Tile::empty(idx + 1, 1);
        let coords = tile.pixel_coordinates(TILE_SIZE);
        let sprite = assets.tile_from(idx as u8, coords, Precedence::Sand);
        let text_style = TextStyle {
            font_size: 32.0,
            ..default()
        };
        let text_justification = JustifyText::Center;
        // 2d camera
        let text_bundle = Text2dBundle {
            text: Text {
                sections: vec![TextSection::new(idx.to_string(), text_style.clone())],
                justify: text_justification,
                linebreak_behavior: BreakLineOn::WordBoundary,
            },
            text_2d_bounds: Text2dBounds {
                // Wrap text in the rectangle
                size: Vec2::new(64., 64.),
            },
            // ensure the text is drawn on top of the box
            transform: Transform::from_translation(coords.extend(99.0) + 32.),
            ..default()
        };
        cmds.spawn((tile, sprite));
        cmds.spawn(text_bundle);
    }
}
