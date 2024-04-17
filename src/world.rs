use std::time::Duration;

use bevy::prelude::*;
/**
 * This is the plugin for the world, it's animations, and creating blocking
 */
use bevy::{math::U16Vec2, sprite::Anchor};
use bevy_asset_loader::prelude::*;

use crate::ui::UiAssets;
use crate::{camera::GameCamera, GameState};

pub const WORLD_SIZE: U16Vec2 = U16Vec2::new(32, 32);
pub const TILE_SIZE: f32 = 64.0;
pub const TILE_VEC: Vec2 = Vec2::new(TILE_SIZE, TILE_SIZE);

pub const ANIMATION_SPEED: Duration = Duration::from_millis(100);

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

/**
 * Global Animations are all synchronised to switch at the same time
 */
#[derive(Resource)]
struct GlobalAnimation {
    timer: Timer,
    frame: usize,
}

/**
 * Global Animations are all synchronised to switch at the same time
 */
#[derive(Resource)]
struct EditorOptions {
    selected: TileKind,
}

#[derive(Component)]
struct EditorTileTypeButton {
    option: TileKind,
    selected: bool,
}

// todo this would be a bundle i guess
#[derive(Component)]
struct Selection;

impl Default for EditorOptions {
    fn default() -> Self {
        Self {
            selected: TileKind::Grass,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Component, Clone)]
enum TileKind {
    Water,
    Grass,
    Sand,
    Cliff,
    Platau,
    Crumbs,
}

impl Default for GlobalAnimation {
    fn default() -> Self {
        Self {
            timer: Timer::new(ANIMATION_SPEED, TimerMode::Repeating),
            frame: 0,
        }
    }
}

#[derive(Debug)]
enum Precedence {
    Water,
    Coast,
    Sand,
    Grass,
    Cliff,
    Decals,
}

impl Precedence {
    fn to_z(&self) -> f32 {
        match self {
            Precedence::Water => -1.0,
            Precedence::Coast => 0.0,
            Precedence::Sand => 1.0,
            Precedence::Grass => 2.0,
            Precedence::Cliff => 3.0,
            Precedence::Decals => 4.0,
        }
    }
}

#[derive(AssetCollection, Resource)]
pub struct WorldAssets {
    #[asset(path = "terrain/water/water.png")]
    pub water_texture: Handle<Image>,

    #[asset(path = "terrain/water/foam/foam.png")]
    pub foam_texture: Handle<Image>,

    #[asset(texture_atlas_layout(tile_size_x = 192., tile_size_y = 192., columns = 8, rows = 1))]
    pub foam_layout: Handle<TextureAtlasLayout>,

    #[asset(path = "terrain/ground/tilemap_sand.png")]
    pub sand_texture: Handle<Image>,

    #[asset(texture_atlas_layout(tile_size_x = 64., tile_size_y = 64., columns = 5, rows = 4))]
    pub sand_layout: Handle<TextureAtlasLayout>,

    #[asset(path = "terrain/ground/tilemap_grass.png")]
    pub grass_texture: Handle<Image>,

    #[asset(texture_atlas_layout(tile_size_x = 64., tile_size_y = 64., columns = 5, rows = 4))]
    pub grass_layout: Handle<TextureAtlasLayout>,

    #[asset(path = "terrain/ground/tilemap_cliff.png")]
    pub cliff_texture: Handle<Image>,

    #[asset(texture_atlas_layout(tile_size_x = 64., tile_size_y = 64., columns = 4, rows = 7))]
    pub cliff_layout: Handle<TextureAtlasLayout>,
}

impl WorldAssets {
    // sand index's
    const ISOLATE: usize = 18;
    const CAP_RIGHT: usize = 17;
    const HORIZONTAL: usize = 16;
    const CAP_LEFT: usize = 15;
    const TOP_LEFT: usize = 0;
    const TOP_CENTER: usize = 1;
    const TOP_RIGHT: usize = 2;
    const CAP_BOT: usize = 13;
    const CRUMBS: usize = 4;
    const LEFT: usize = 5;
    const NONE: usize = 6;
    const RIGHT: usize = 7;
    const VERTICAL: usize = 8;
    const BOT_LEFT: usize = 10;
    const BOT: usize = 11;
    const BOT_RIGHT: usize = 12;
    const CAP_TOP: usize = 3;

    fn water(&self, xy: Vec2) -> SpriteBundle {
        let texture = self.water_texture.clone();
        SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::BottomLeft,
                ..default()
            },
            transform: Transform::from_translation(xy.extend(-1.)),
            texture,
            ..default()
        }
    }

    // todo: This should probably be a data structure that can be queried
    // with all the mappings for the different tiles
    fn platau_index_from_bitmask(&self, bitmask: u8) -> usize {
        match bitmask {
            BITMASK_CENTER => 5,
            BITMASK_TOP => 11,
            BITMASK_VERTICAL => 7,
            BITMASK_BOT => 3,
            BITMASK_RIGHT => 16,
            BITMASK_HORIZONTAL => 17,
            BITMASK_LEFT => 18,

            BITMASK_TOP_LEFT => 10,
            BITMASK_TOP_RIGHT => 8,
            BITMASK_BOT_RIGHT => 0,
            BITMASK_BOT_LEFT => 2,

            BITMASK_BOT_TOP_LEFT => 6,
            BITMASK_BOT_TOP_RIGHT => 4,

            BITMASK_BOT_LEFT_RIGHT => 1,
            BITMASK_TOP_LEFT_RIGHT => 9,
            BITMASK_NONE => 19,
            _ => 19,
        }
    }

    fn bitmask_from_platau_index(&self, idx: usize) -> u8 {
        match idx {
            5 => BITMASK_CENTER,
            11 => BITMASK_TOP,
            7 => BITMASK_VERTICAL,
            3 => BITMASK_BOT,
            16 => BITMASK_RIGHT,
            17 => BITMASK_HORIZONTAL,
            18 => BITMASK_LEFT,

            10 => BITMASK_TOP_LEFT,
            8 => BITMASK_TOP_RIGHT,
            0 => BITMASK_BOT_RIGHT,
            2 => BITMASK_BOT_LEFT,

            6 => BITMASK_BOT_TOP_LEFT,
            4 => BITMASK_BOT_TOP_RIGHT,

            1 => BITMASK_BOT_LEFT_RIGHT,
            9 => BITMASK_TOP_LEFT_RIGHT,

            19 => BITMASK_NONE,
            _ => BITMASK_NONE,
        }
    }

    // 0,1,2,3

    // cliff's are different, we basically need to know if it's a singular or double
    fn cliff_index_from_bitmask(&self, bitmask: u8) -> usize {
        // todo: Randomize the mid section so cliff walls don't look repititve
        let mid_section = vec![12, 13, 14, 20, 21, 22];
        // could randomize the idependent section as well
        let independent = vec![15, 23];
        match bitmask {
            1 => 14,
            3 => 13,
            2 => 12,
            0 => 23,
            // todo: if it's covered don't render
            // BITMASK_TOP => 3,      // none
            // BITMASK_VERTICAL => 7, // none
            // BITMASK_CENTER => 7, // none
            _ => 23,
        }
    }

    fn bitmask_from_cliff_index(&self, idx: usize) -> u8 {
        match idx {
            14 => 1,
            13 => 3,
            12 => 2,
            23 => 0,
            _ => 0,
        }
    }

    fn index_from_bitmask(&self, bitmask: u8) -> usize {
        match bitmask {
            BITMASK_LEFT => Self::CAP_RIGHT,
            BITMASK_RIGHT => Self::CAP_LEFT,
            BITMASK_HORIZONTAL => Self::HORIZONTAL,
            BITMASK_VERTICAL => Self::VERTICAL,
            BITMASK_CENTER => Self::NONE,
            BITMASK_BOT => Self::CAP_TOP,
            BITMASK_TOP => Self::CAP_BOT,
            // todo: Fix a naming convention, are we refering to the open connections? Makes sense
            BITMASK_BOT_TOP_RIGHT => Self::LEFT,
            BITMASK_BOT_TOP_LEFT => Self::RIGHT,
            BITMASK_BOT_LEFT_RIGHT => Self::TOP_CENTER,
            BITMASK_TOP_LEFT_RIGHT => Self::BOT,
            BITMASK_BOT_RIGHT => Self::TOP_LEFT,
            BITMASK_BOT_LEFT => Self::TOP_RIGHT,
            BITMASK_TOP_RIGHT => Self::BOT_LEFT,
            BITMASK_TOP_LEFT => Self::BOT_RIGHT,
            _ => Self::ISOLATE,
        }
    }

    fn bitmask_from_index(&self, idx: usize) -> u8 {
        match idx {
            Self::CAP_RIGHT => BITMASK_LEFT,
            Self::CAP_LEFT => BITMASK_RIGHT,
            Self::HORIZONTAL => BITMASK_HORIZONTAL,
            Self::VERTICAL => BITMASK_VERTICAL,
            Self::NONE => BITMASK_CENTER,
            Self::CAP_BOT => BITMASK_TOP,
            Self::CAP_TOP => BITMASK_BOT,
            Self::LEFT => BITMASK_BOT_TOP_RIGHT,
            Self::RIGHT => BITMASK_BOT_TOP_LEFT,
            Self::TOP_CENTER => BITMASK_BOT_LEFT_RIGHT,
            Self::BOT => BITMASK_TOP_LEFT_RIGHT,
            Self::TOP_LEFT => BITMASK_BOT_RIGHT,
            Self::TOP_RIGHT => BITMASK_BOT_LEFT,
            Self::BOT_LEFT => BITMASK_TOP_RIGHT,
            Self::BOT_RIGHT => BITMASK_TOP_LEFT,
            _ => BITMASK_NONE,
        }
    }

    fn foam(&self, xy: Vec2, z: f32) -> SpriteSheetBundle {
        let layout = self.foam_layout.clone();
        let texture = self.foam_texture.clone();
        SpriteSheetBundle {
            sprite: Sprite {
                anchor: Anchor::Center,
                ..default()
            },
            texture,
            transform: Transform::from_translation(xy.extend(z)),
            atlas: TextureAtlas { layout, index: 0 },
            ..default()
        }
    }

    fn grass(&self, idx: u8, xy: Vec2, z: f32) -> SpriteSheetBundle {
        if idx > 5 * 4 {
            panic!("out of bounds");
        }
        let layout = self.grass_layout.clone();
        let texture = self.grass_texture.clone();
        SpriteSheetBundle {
            sprite: Sprite {
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture,
            transform: Transform::from_translation(xy.extend(z)),
            atlas: TextureAtlas {
                layout,
                index: idx as usize,
            },
            ..default()
        }
    }

    fn cliff(&self, idx: u8, xy: Vec2, z: f32) -> SpriteSheetBundle {
        if idx > 4 * 7 {
            panic!("out of bounds");
        }
        let layout = self.cliff_layout.clone();
        let texture = self.cliff_texture.clone();
        SpriteSheetBundle {
            sprite: Sprite {
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture,
            transform: Transform::from_translation(xy.extend(z)),
            atlas: TextureAtlas {
                layout,
                index: idx as usize,
            },
            ..default()
        }
    }

    fn sand(&self, idx: u8, xy: Vec2, z: f32) -> SpriteSheetBundle {
        if idx > 5 * 4 {
            panic!("out of bounds");
        }
        let layout = self.sand_layout.clone();
        let texture = self.sand_texture.clone();
        SpriteSheetBundle {
            sprite: Sprite {
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture,
            transform: Transform::from_translation(xy.extend(z)),
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
        .add_systems(
            OnEnter(self.state.clone()),
            (setup_tile_system, setup_editor_ui),
        )
        .init_resource::<GlobalAnimation>()
        .init_resource::<EditorOptions>()
        .add_systems(
            Update,
            (update_tile_system, update_animated_tiles, update_editor_ui)
                .run_if(in_state(self.state.clone())),
        );
        // note: When adding update states prefix with `.run_if(in_state(self.state.clone())))`
    }
}

impl<S: States> WorldPlugin<S> {
    pub fn run_on_state(state: S) -> Self {
        Self { state }
    }
}

#[derive(Component, Debug, Clone)]
struct Tile {
    pos: U16Vec2,
    elevation: u8,
}

impl Tile {
    // question: How does elevation create cliff's? If a tile is elevated more then it's neighbours
    // we should create a cliff 1 below it. If the elevation is 2 it should have two cliff tiles
    // below it, etc for the total elevation
    fn with_elevation(mut self, elevation: u8) -> Self {
        self.elevation = elevation;
        self
    }

    fn add_elevation(&mut self, elevation: u8) {
        if elevation == u8::MAX {
            return;
        }
        self.elevation += elevation;
    }

    fn minus_elevation(&mut self, elevation: u8) {
        if elevation == 0 {
            return;
        }
        self.elevation -= elevation;
    }
    fn new(x: u16, y: u16) -> Self {
        Self {
            pos: U16Vec2::new(x, y),
            elevation: 0,
        }
    }
    fn from_coordinates(&self, tile_size: f32) -> Vec2 {
        Vec2::new(self.pos.x as f32 * tile_size, self.pos.y as f32 * tile_size)
    }
}

#[derive(Bundle, Clone)]
struct TileBundle {
    tile: Tile,
    kind: TileKind,
    // todo
    // sprite: SpriteBundle,
}

impl TileBundle {
    fn with_elevation(mut self, elevation: u8) -> Self {
        self.tile.elevation = elevation;
        self
    }
    fn sand(x: u16, y: u16) -> Self {
        TileBundle {
            tile: Tile::new(x, y),
            kind: TileKind::Sand,
        }
    }
    fn cliff(x: u16, y: u16) -> Self {
        TileBundle {
            tile: Tile::new(x, y),
            kind: TileKind::Cliff,
        }
    }
}

#[derive(Component, Debug)]
struct GloballyAnimated {
    max_frames: u8,
}

impl GloballyAnimated {
    fn new(max_frames: u8) -> Self {
        Self { max_frames }
    }
}
fn check_neighbours_bitmask_x<'a, I>(tile_pos: U16Vec2, tiles_iter: I) -> u8
where
    I: Iterator<Item = &'a Tile>,
{
    let mut bitmask = 0;
    for tile in tiles_iter.filter(|tile| tile.pos.y == tile_pos.y) {
        let current_pos = tile.pos.as_ivec2();
        let tile_pos_ivec = tile_pos.as_ivec2();
        // left
        if current_pos == (tile_pos_ivec - IVec2::X) {
            bitmask += 2_u32.pow(0);
        }
        // right
        if current_pos == (tile_pos_ivec + IVec2::X) {
            bitmask += 2_u32.pow(1);
        }
    }
    bitmask as u8
}

fn check_neighbours_bitmask<'a, I>(tile_pos: U16Vec2, tiles_iter: I) -> u8
where
    I: Iterator<Item = &'a Tile>,
{
    let mut bitmask = 0;
    for tile in tiles_iter {
        let current_pos = tile.pos.as_ivec2();
        let tile_pos_ivec = tile_pos.as_ivec2();
        // up
        if current_pos == (tile_pos_ivec + IVec2::Y) {
            bitmask += 2_u32.pow(0);
        }
        // left
        if current_pos == (tile_pos_ivec - IVec2::X) {
            bitmask += 2_u32.pow(1);
        }
        // right
        if current_pos == (tile_pos_ivec + IVec2::X) {
            bitmask += 2_u32.pow(2);
        }
        // down
        if current_pos == (tile_pos_ivec - IVec2::Y) {
            bitmask += 2_u32.pow(3);
        }
    }
    bitmask as u8
}

fn update_animated_tiles(
    mut animated_q: Query<(&mut TextureAtlas, &mut GloballyAnimated)>,
    time: Res<Time>,
    mut global_animation: ResMut<GlobalAnimation>,
) {
    if global_animation.frame > usize::MAX {
        global_animation.frame = 0;
    }
    global_animation.timer.tick(time.delta());
    if global_animation.timer.finished() {
        global_animation.frame += 1;
        for (mut texture_atlas, animated) in &mut animated_q {
            texture_atlas.index = global_animation.frame % animated.max_frames as usize;
        }
    }
}

fn update_tile_system(
    mut cmds: Commands,
    window_q: Query<&Window>,
    camera_q: Query<(&Camera, &mut GlobalTransform), With<GameCamera>>,
    mut tile_q: Query<(Entity, &Children, &mut Tile, &TileKind)>,
    mut texture_atlas_q: Query<(&mut TextureAtlas, &TileKind)>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    assets: Res<WorldAssets>,
    editor_opts: ResMut<EditorOptions>,
    mut gizmos: Gizmos,
) {
    if let Ok(window) = window_q.get_single() {
        for (camera, camera_transform) in camera_q.iter() {
            if let Some(cursor_pos) = window.cursor_position() {
                if let Some(world_cursor_pos) =
                    camera.viewport_to_world_2d(camera_transform, cursor_pos)
                {
                    let tile_pos = (world_cursor_pos / TILE_VEC).floor().as_u16vec2();
                    let tile_pos_ivec = tile_pos.as_ivec2();
                    gizmos.rect_2d(
                        tile_pos.as_vec2() * TILE_VEC + TILE_VEC / 2.0,
                        0.0,
                        TILE_VEC,
                        Color::GREEN,
                    );
                    for (_, _, tile, tile_kind) in &tile_q {
                        if tile.pos == tile_pos && tile_kind == &TileKind::Cliff {
                            gizmos.rect_2d(
                                (tile.pos).as_vec2() * TILE_VEC
                                    + (TILE_VEC / 2.0)
                                    + (Vec2::Y * TILE_VEC / 2.0),
                                0.0,
                                TILE_VEC * Vec2::new(1.0, 2.0),
                                Color::GREEN,
                            );
                        } else {
                        }
                    }
                    // instead of this nonsense, we grab the neighbours and do
                    if mouse_button.pressed(MouseButton::Left)
                        && editor_opts.selected == TileKind::Sand
                        && tile_q
                            .iter()
                            .filter(|(_, _, _, kind)| **kind == TileKind::Sand)
                            .find(|(_, _, tile, _)| tile.pos == tile_pos)
                            .is_none()
                    {
                        // note: This should be modularised for some of the features
                        let mut found = false;
                        let bit_mask_total = check_neighbours_bitmask(
                            tile_pos,
                            tile_q
                                .iter()
                                .filter(|(_, _, _, kind)| **kind == TileKind::Sand)
                                .map(|(_, _, tile, _)| tile),
                        );
                        let new_idx = assets.index_from_bitmask(bit_mask_total);
                        for (entity, _, tile, _) in &mut tile_q
                            .iter_mut()
                            .filter(|(_, _, _, kind)| **kind == TileKind::Sand)
                        {
                            let (mut texture_atlas, _) = texture_atlas_q.get_mut(entity).unwrap();
                            let current_pos = tile.pos.as_ivec2();
                            // right
                            if current_pos == tile_pos_ivec + IVec2::X {
                                let bitmask_right =
                                    assets.bitmask_from_index(texture_atlas.index) + BITMASK_LEFT;
                                texture_atlas.index = assets.index_from_bitmask(bitmask_right);
                            }
                            // left
                            if current_pos == tile_pos_ivec - IVec2::X {
                                let bitmask_left =
                                    assets.bitmask_from_index(texture_atlas.index) + BITMASK_RIGHT;
                                texture_atlas.index = assets.index_from_bitmask(bitmask_left);
                            }
                            // down
                            if current_pos == tile_pos_ivec - IVec2::Y {
                                let bitmask_down =
                                    assets.bitmask_from_index(texture_atlas.index) + BITMASK_TOP;
                                texture_atlas.index = assets.index_from_bitmask(bitmask_down);
                            }
                            // up
                            if current_pos == tile_pos_ivec + IVec2::Y {
                                let bitmask_up =
                                    assets.bitmask_from_index(texture_atlas.index) + BITMASK_BOT;
                                texture_atlas.index = assets.index_from_bitmask(bitmask_up);
                            }
                            if tile.pos == tile_pos {
                                found = true;
                                texture_atlas.index = new_idx;
                            }
                        }
                        if found == false {
                            let tile = TileBundle::sand(tile_pos.x as u16, tile_pos.y as u16);
                            let sprite =
                                assets.sand(new_idx as u8, tile_pos.as_vec2() * TILE_VEC, 0.0);
                            cmds.spawn((tile, sprite)).with_children(|parent| {
                                parent.spawn((
                                    assets.foam(TILE_VEC * 0.5, -1.0),
                                    GloballyAnimated::new(7),
                                ));
                            });
                        }
                    }

                    if editor_opts.selected == TileKind::Cliff
                        && mouse_button.pressed(MouseButton::Left)
                        && tile_q
                            .iter()
                            .filter(|(_, _, tile, _)| tile.elevation == 1)
                            .find(|(_, _, tile, _)| tile.pos == tile_pos)
                            .is_none()
                    {
                        let mut found = false;
                        for (entity, children, tile, _) in &mut tile_q
                            .iter_mut()
                            .filter(|(_, _, tile, _)| tile.elevation == 1)
                        {
                            let current_pos = tile.pos.as_ivec2();
                            if current_pos == tile_pos_ivec + IVec2::Y {
                                // change the platau
                                for child in children {
                                    if let Ok((mut texture_atlas, TileKind::Platau)) =
                                        texture_atlas_q.get_mut(*child)
                                    {
                                        let bitmask_right = assets
                                            .bitmask_from_platau_index(texture_atlas.index)
                                            + BITMASK_BOT;
                                        texture_atlas.index =
                                            assets.platau_index_from_bitmask(bitmask_right);
                                        break;
                                    }
                                }
                            }
                            if current_pos == tile_pos_ivec - IVec2::Y {
                                // change the platau
                                for child in children {
                                    if let Ok((mut texture_atlas, TileKind::Platau)) =
                                        texture_atlas_q.get_mut(*child)
                                    {
                                        let bitmask_right = assets
                                            .bitmask_from_platau_index(texture_atlas.index)
                                            + BITMASK_TOP;
                                        texture_atlas.index =
                                            assets.platau_index_from_bitmask(bitmask_right);
                                        break;
                                    }
                                }
                            }
                            if current_pos == tile_pos_ivec - IVec2::X {
                                // change the wall
                                let (mut texture_atlas, _) =
                                    texture_atlas_q.get_mut(entity).unwrap();
                                let bitmask_right = assets
                                    .bitmask_from_cliff_index(texture_atlas.index)
                                    // uses left here as the bitmask is only in the x
                                    + 2;
                                texture_atlas.index =
                                    assets.cliff_index_from_bitmask(bitmask_right);
                                // change the platau
                                for child in children {
                                    if let Ok((mut texture_atlas, TileKind::Platau)) =
                                        texture_atlas_q.get_mut(*child)
                                    {
                                        let bitmask_right = assets
                                            .bitmask_from_platau_index(texture_atlas.index)
                                            + BITMASK_RIGHT;
                                        texture_atlas.index =
                                            assets.platau_index_from_bitmask(bitmask_right);
                                        break;
                                    }
                                }
                            }
                            if current_pos == tile_pos_ivec + IVec2::X {
                                // change the wall
                                let (mut texture_atlas, _) =
                                    texture_atlas_q.get_mut(entity).unwrap();
                                let bitmask_right = assets
                                    .bitmask_from_cliff_index(texture_atlas.index)
                                    // uses top here as the bitmask is only in the x
                                    + 1;
                                texture_atlas.index =
                                    assets.cliff_index_from_bitmask(bitmask_right);
                                // change the platau
                                for child in children {
                                    if let Ok((mut texture_atlas, TileKind::Platau)) =
                                        texture_atlas_q.get_mut(*child)
                                    {
                                        let bitmask_right = assets
                                            .bitmask_from_platau_index(texture_atlas.index)
                                            + BITMASK_LEFT;
                                        texture_atlas.index =
                                            assets.platau_index_from_bitmask(bitmask_right);
                                        break;
                                    }
                                }
                            }
                            if tile.pos == tile_pos {
                                found = true;
                            }
                        }
                        if !found {
                            let bit_mask_total = check_neighbours_bitmask(
                                tile_pos,
                                tile_q
                                    .iter()
                                    .filter(|(_, _, tile, _)| tile.elevation == 1)
                                    .map(|(_, _, tile, _)| tile),
                            );
                            let bit_mask_total_x = check_neighbours_bitmask_x(
                                tile_pos,
                                tile_q
                                    .iter()
                                    .filter(|(_, _, tile, _)| tile.elevation == 1)
                                    .map(|(_, _, tile, _)| tile),
                            );
                            let wall_idx = assets.cliff_index_from_bitmask(bit_mask_total_x);
                            dbg!(wall_idx);
                            let z = Precedence::Cliff.to_z() * (WORLD_SIZE.y - tile_pos.y) as f32;
                            let wall_sprite =
                                assets.cliff(wall_idx as u8, tile_pos.as_vec2() * TILE_SIZE, z);
                            let cliff_tile =
                                TileBundle::cliff(tile_pos.x, tile_pos.y).with_elevation(1);
                            cmds.spawn((cliff_tile, wall_sprite))
                                .with_children(|parent| {
                                    let crumbs =
                                        assets.sand(WorldAssets::CRUMBS as u8, Vec2::Y, z + 0.1);
                                    let platau_idx =
                                        assets.platau_index_from_bitmask(bit_mask_total);
                                    let platau_sprite =
                                        assets.cliff(platau_idx as u8, Vec2::Y * TILE_SIZE, z);
                                    parent.spawn((crumbs, TileKind::Crumbs));
                                    parent.spawn((platau_sprite, TileKind::Platau));
                                });
                            // cmds.entity(entity).push_children(&[wall, crumb, platau]);
                        }
                    }
                }
            }
        }
    }
}

fn update_editor_ui(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut editor_opts: ResMut<EditorOptions>,
    ui_q: Query<(&EditorTileTypeButton, &Children)>,
    mut select_q: Query<&mut Visibility, With<Selection>>,
) {
    for (editor_button, children) in &ui_q {
        if editor_button.option == TileKind::Cliff && keyboard_input.just_pressed(KeyCode::Digit2) {
            editor_opts.selected = TileKind::Cliff;
            for child in children.iter() {
                if let Ok(mut visibility) = select_q.get_mut(*child) {
                    *visibility = Visibility::Visible;
                }
            }
        }
        if editor_button.option == TileKind::Cliff && keyboard_input.just_pressed(KeyCode::Digit1) {
            for child in children.iter() {
                if let Ok(mut visibility) = select_q.get_mut(*child) {
                    *visibility = Visibility::Hidden;
                }
            }
        }

        if editor_button.option == TileKind::Sand && keyboard_input.just_pressed(KeyCode::Digit1) {
            editor_opts.selected = TileKind::Sand;
            for child in children.iter() {
                if let Ok(mut visibility) = select_q.get_mut(*child) {
                    *visibility = Visibility::Visible;
                }
            }
        }
        if editor_button.option == TileKind::Sand && keyboard_input.just_pressed(KeyCode::Digit2) {
            for child in children.iter() {
                if let Ok(mut visibility) = select_q.get_mut(*child) {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
}

fn setup_editor_ui(mut cmds: Commands, world_assets: Res<WorldAssets>, ui_assets: Res<UiAssets>) {
    let place_sand_icon = AtlasImageBundle {
        background_color: Color::WHITE.into(),
        image: UiImage::new(world_assets.sand_texture.clone()),
        style: Style {
            width: Val::Px(TILE_SIZE),
            height: Val::Px(TILE_SIZE),
            left: Val::Px(0.),
            right: Val::Px(0.0),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            justify_items: JustifyItems::Center,
            ..default()
        },
        texture_atlas: TextureAtlas {
            layout: world_assets.sand_layout.clone(),
            index: 18,
        },
        ..default()
    };
    let place_sand_key = TextBundle {
        text: Text::from_section(
            "1",
            TextStyle {
                font_size: 28.0,
                color: Color::RED,
                ..default()
            },
        )
        .with_justify(JustifyText::Center),
        style: Style {
            top: Val::Px(14.0),
            justify_self: JustifySelf::Center,
            position_type: PositionType::Relative,
            ..default()
        },
        ..default()
    };
    let selected = ImageBundle {
        style: Style {
            position_type: PositionType::Absolute,
            ..default()
        },
        visibility: Visibility::Hidden,
        background_color: Color::WHITE.into(),
        image: UiImage::new(ui_assets.select.clone()),
        ..default()
    };
    cmds.spawn((
        place_sand_icon,
        EditorTileTypeButton {
            option: TileKind::Sand,
            selected: true,
        },
    ))
    .with_children(|parent| {
        parent.spawn(place_sand_key);
        parent.spawn((selected, Selection));
    });
    let place_cliff_icon = AtlasImageBundle {
        background_color: Color::WHITE.into(),
        image: UiImage::new(world_assets.cliff_texture.clone()),
        style: Style {
            width: Val::Px(TILE_SIZE),
            height: Val::Px(TILE_SIZE),
            left: Val::Px(TILE_SIZE * 1.0),
            right: Val::Px(0.0),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            justify_items: JustifyItems::Center,
            ..default()
        },
        texture_atlas: TextureAtlas {
            layout: world_assets.cliff_layout.clone(),
            index: 19,
        },
        ..default()
    };
    let place_cliff_key = TextBundle {
        text: Text::from_section(
            "2",
            TextStyle {
                font_size: 28.0,
                color: Color::RED,
                ..default()
            },
        )
        .with_justify(JustifyText::Center),
        style: Style {
            top: Val::Px(14.0),
            justify_self: JustifySelf::Center,
            position_type: PositionType::Relative,
            ..default()
        },
        ..default()
    };
    let selected = ImageBundle {
        style: Style {
            position_type: PositionType::Absolute,
            ..default()
        },
        visibility: Visibility::Hidden,
        background_color: Color::WHITE.into(),
        image: UiImage::new(ui_assets.select.clone()),
        ..default()
    };
    cmds.spawn((
        place_cliff_icon,
        EditorTileTypeButton {
            option: TileKind::Cliff,
            selected: false,
        },
    ))
    .with_children(|parent| {
        parent.spawn(place_cliff_key);
        parent.spawn((selected, Selection));
    });
}

fn setup_tile_system(mut cmds: Commands, assets: Res<WorldAssets>) {
    for idx in 0..(4 * 7) {
        let tile = Tile::new(idx, 0);
        let coords = tile.from_coordinates(TILE_SIZE);
        let sprite = assets.cliff(idx as u8, coords, 100.);
        cmds.spawn(sprite).with_children(|parent| {
            let text = Text2dBundle {
                transform: Transform::from_translation(Vec3::new(28.0, 28.0, 1.0)),
                text: Text::from_section(
                    idx.to_string(),
                    TextStyle {
                        color: Color::WHITE,
                        font_size: 32.,
                        ..default()
                    },
                )
                .with_justify(JustifyText::Center),
                ..default()
            };
            parent.spawn(text);
        });
    }
}
