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

// autotiling theory:
// we look around at neighbours, and calculate their bit mask from a 8x8 kernel for the "same" tile
// Then we use a LUT to get the appropriate tile
// Tiles need to have a precedence order. So if I use a sand tile, it should be below a grass tile.
// This approach is super fast which is good!
// For example a grass tile of bitmask sum 1 is the square tile, ez!
// with the current tile set each tile is 64x64 pixels in size
// 1 0 1
// 0 x 0
// 1 0 1 <- single tile
//
// 0 1 0
// 1 x 1
// 0 1 0 <- center tile
//
// 0 0 0
// 1 x 1
// 0 1 0 <- top tile
//
// 0 0 0
// 0 x 1
// 0 1 0 <- top left tile
// The tiles are also on a invisible background, so the precedence of tiles under them means some
// tiles we need two stacked tiles
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

    fn sand_isolate_idx(&self) -> usize {
        38
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
    x: u16,
    y: u16,
    precedence: Precedence,
}

impl Tile {
    fn empty(x: u16, y: u16) -> Self {
        Self {
            x,
            y,
            precedence: Precedence::Water,
        }
    }
    fn pixel_coordinates(&self, tile_size: f32) -> Vec2 {
        Vec2::new(self.x as f32 * tile_size, self.y as f32 * tile_size)
    }
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
                    // let tile_rect = Rect::new(0.0, 0.0, TILE_SIZE, TILE_SIZE);
                    let tile_pos = (world_cursor_pos / TILE_VEC).floor();
                    gizmos.rect_2d(
                        tile_pos * TILE_VEC + TILE_VEC / 2.0,
                        0.0,
                        TILE_VEC,
                        Color::GREEN,
                    );
                    if mouse_button.just_pressed(MouseButton::Left) {
                        let mut found = false;
                        for (mut texture_atlas, tile) in &mut tiles_q {
                            if tile.x == tile_pos.x as u16 && tile.y == tile_pos.y as u16 {
                                found = true;
                                let new_sprite = assets.sand_isolate_idx();
                                texture_atlas.index = new_sprite;
                                dbg!("replaced land");
                                break;
                            }
                        }
                        if found == false {
                            let new_sprite = assets.sand_isolate_idx();
                            let sprite = assets.tile_from(
                                new_sprite as u8,
                                tile_pos * TILE_VEC,
                                Precedence::Sand,
                            );
                            let tile = Tile::empty(tile_pos.x as u16, tile_pos.y as u16);
                            dbg!("added sand");
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
