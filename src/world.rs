use std::time::Duration;

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle};
use bevy::utils::hashbrown::HashMap;
/**
 * This is the plugin for the world, it's animations, and creating blocking
 */
use bevy::{math::U16Vec2, sprite::Anchor};
use bevy_asset_loader::prelude::*;

use crate::GameState;

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

#[derive(Event, Debug)]
pub struct PlaceLandTile {
    pos: U16Vec2,
    ground: Land,
    elevation: u8,
}

impl PlaceLandTile {
    pub fn sand(x: u16, y: u16, elevation: u8) -> Self {
        Self {
            pos: U16Vec2::new(x, y),
            ground: Land::Sand,
            elevation,
        }
    }
}

#[derive(Component, PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub struct Elevation(pub u8);

// todo: Use this to replace the TileKind
#[derive(Component, PartialEq, Eq, Clone, Copy, Debug, Hash)]
enum Land {
    Sand,
    Grass,
}

#[derive(Debug, PartialEq, Eq, Component, Clone)]
struct Platau;
#[derive(Debug, PartialEq, Eq, Component, Clone)]
struct Cliff;

impl Default for GlobalAnimation {
    fn default() -> Self {
        Self {
            timer: Timer::new(ANIMATION_SPEED, TimerMode::Repeating),
            frame: 0,
        }
    }
}

#[derive(AssetCollection, Resource)]
pub struct WorldAssets {
    #[asset(path = "terrain/water/water.png")]
    pub water_texture: Handle<Image>,

    #[asset(path = "terrain/water/foam/foam.png")]
    pub coast_texture: Handle<Image>,

    #[asset(path = "terrain/ground/shadow.png")]
    pub shadow_texture: Handle<Image>,

    #[asset(texture_atlas_layout(tile_size_x = 192., tile_size_y = 192., columns = 8, rows = 1))]
    pub coast_layout: Handle<TextureAtlasLayout>,

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

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct WaterMaterial {
    color: Color,
}

impl Material2d for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/water.wgsl".into()
    }
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

    // cliff's are different, we basically need to know if it's a singular or double
    fn cliff_index_from_bitmask(&self, bitmask: u8) -> usize {
        match bitmask {
            // idependent piece
            0 => 23,
            // left
            1 => 14,
            // right
            2 => 12,
            // centre
            3 => 13,
            _ => 23,
        }
    }

    fn bitmask_from_cliff_index(&self, idx: usize) -> u8 {
        match idx {
            14 | 22 => 1,
            13 | 21 => 3,
            12 | 20 => 2,
            23 | 15 => 0,
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

    fn shadow(&self, xy: Vec2, z: f32) -> SpriteSheetBundle {
        let texture = self.shadow_texture.clone();
        SpriteSheetBundle {
            sprite: Sprite {
                anchor: Anchor::Center,
                ..default()
            },
            texture,
            transform: Transform::from_translation(xy.extend(z)),
            ..default()
        }
    }

    fn coast(&self, xy: Vec2, z: f32) -> SpriteSheetBundle {
        let texture = self.coast_texture.clone();
        let layout = self.coast_layout.clone();
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
    // todo: We need a wildcard tile that will match anything placed next to it
    fn sand(&self, xy: Vec2, z: f32) -> SpriteSheetBundle {
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
                index: WorldAssets::ISOLATE as usize,
            },
            ..default()
        }
    }

    fn sand_from(&self, idx: u8, xy: Vec2, z: f32) -> SpriteSheetBundle {
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

    fn spawn_sea_cliff(&self, cmds: &mut Commands, xy: U16Vec2, height: u8) {
        assert_ne!(height, 0);
        let base_tile = TileBundle::sea_cliff(xy.x, xy.y, height);
        let z_offset = base_tile.elevation.0 as f32 + (WORLD_SIZE.y - xy.y) as f32;
        cmds.spawn(base_tile).with_children(|parent| {
            let wall_idx = self.cliff_index_from_bitmask(0);
            let shadow = self.shadow(TILE_VEC * 0.5, z_offset);
            let platau_idx = self.platau_index_from_bitmask(WorldAssets::ISOLATE as u8);
            let platau_sprite = self.cliff(
                platau_idx as u8,
                Vec2::Y * TILE_SIZE * height as f32,
                z_offset + 0.3,
            );
            parent.spawn((
                self.coast(TILE_VEC * 0.5, -z_offset - 1.0),
                GloballyAnimated::new(7),
            ));
            parent.spawn((platau_sprite, Platau, Elevation(height)));
            for i in 1..=height {
                let wall_sprite = self.cliff(
                    wall_idx as u8,
                    Vec2::Y * TILE_SIZE * (i - 1) as f32,
                    z_offset + 0.1,
                );
                parent.spawn((wall_sprite, Cliff, Elevation(i)));
            }
            parent.spawn(shadow);
        });
    }

    fn spawn_ground(&self, cmds: &mut Commands, xy: U16Vec2) {
        let elevation = 0;
        let tile = TileBundle::ground(xy.x, xy.y, elevation);
        cmds.spawn((tile, Land::Sand)).with_children(|parent| {
            let z_offset = (WORLD_SIZE.y - xy.y) as f32 + elevation as f32;
            let sprite = self.sand(Vec2::ZERO, z_offset);
            parent.spawn((sprite, Land::Sand, Elevation(elevation)));
            parent.spawn((
                self.coast(TILE_VEC * 0.5, -z_offset - 1.0),
                GloballyAnimated::new(7),
            ));
        });
    }
}

fn setup_water(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WaterMaterial>>,
) {
    let width = TILE_SIZE as f32 * WORLD_SIZE.x as f32;
    let height = TILE_SIZE as f32 * WORLD_SIZE.y as f32;
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Rectangle::new(width, height)).into(),
        transform: Transform::from_xyz(width / 2., height / 2., -100.),
        material: materials.add(WaterMaterial { color: Color::BLUE }),
        ..default()
    });
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
            (setup_tile_system, setup_water),
        )
        .init_resource::<GlobalAnimation>()
        .add_plugins(Material2dPlugin::<WaterMaterial>::default())
        .add_event::<PlaceLandTile>()
        .add_event::<AdjustElevation>()
        .add_systems(
            Update,
            (
                // update_ground_atlas_index,
                update_added_ground_atlas_index,
                update_platau_atlas_index,
                update_cliff_atlas_index,
                place_land,
                update_animated_tiles,
            )
                .run_if(in_state(self.state.clone())),
        );
    }
}

impl<S: States> WorldPlugin<S> {
    pub fn run_on_state(state: S) -> Self {
        Self { state }
    }
}

// this works p. well, the only adjustment would be land is split into grass and sand but also both
#[derive(Debug, Clone)]
enum TileType {
    Land,
    SeaCliff { has_top: bool },
    LandCliff { has_top: bool },
}

#[derive(Component, Debug, Clone)]
pub struct Tile {
    pub pos: U16Vec2,
    tile_type: TileType,
}

impl Tile {
    fn new(x: u16, y: u16, tile_type: TileType) -> Self {
        Self {
            pos: U16Vec2::new(x, y),
            tile_type,
        }
    }
    fn ground(x: u16, y: u16) -> Self {
        Self::new(x, y, TileType::Land)
    }

    fn sea_cliff(x: u16, y: u16) -> Self {
        Self::new(x, y, TileType::SeaCliff { has_top: false })
    }

    fn add_top(&mut self) {
        match &mut self.tile_type {
            TileType::SeaCliff { has_top } => *has_top = true,
            TileType::LandCliff { has_top } => *has_top = true,
            _ => {}
        }
    }
}

#[derive(Bundle, Clone)]
struct TileBundle {
    tile: Tile,
    elevation: Elevation,
    transform: TransformBundle,
    visibility: VisibilityBundle,
}

impl TileBundle {
    fn sea_cliff(x: u16, y: u16, elevation: u8) -> Self {
        TileBundle {
            tile: Tile::sea_cliff(x, y),
            elevation: Elevation(elevation),
            visibility: VisibilityBundle {
                visibility: Visibility::Visible,
                ..default()
            },
            transform: TransformBundle::from_transform(Transform::from_xyz(
                x as f32 * TILE_SIZE,
                y as f32 * TILE_SIZE,
                0.,
            )),
        }
    }
    fn ground(x: u16, y: u16, elevation: u8) -> Self {
        TileBundle {
            tile: Tile::ground(x, y),
            elevation: Elevation(elevation),
            visibility: VisibilityBundle {
                visibility: Visibility::Visible,
                ..default()
            },
            transform: TransformBundle::from_transform(Transform::from_xyz(
                x as f32 * TILE_SIZE,
                y as f32 * TILE_SIZE,
                0.,
            )),
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
// todo: can use https://docs.rs/image/0.25.1/image/struct.ImageBuffer.html so we flatten tiles
// into a single texture
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

#[derive(Event, Debug)]
struct AdjustElevation {
    pos: U16Vec2,
    elevation: u8,
}

fn update_added_ground_atlas_index(
    mut tiles_q: Query<(&mut TextureAtlas, &GlobalTransform, &Elevation, Ref<Land>)>,
    assets: Res<WorldAssets>,
) {
    // build the full tile map, I GUESS. Can probably cache
    let mut tiles: HashMap<(i16, i16, Elevation, Land), bool> =
        HashMap::with_capacity(tiles_q.iter().len());
    for (_, transform, elevation, land) in &tiles_q {
        let tile_pos = (transform.translation().truncate() / TILE_VEC)
            .floor()
            .as_i16vec2();
        tiles.insert((tile_pos.x, tile_pos.y, *elevation, *land), land.is_added());
    }
    // we then use the coordinates to get a map of the neighbours and their land type
    for (mut atlas, transform, elevation, land) in &mut tiles_q {
        let tile_pos = (transform.translation().truncate() / TILE_VEC)
            .floor()
            .as_i16vec2();
        if land.is_added() {
            let mut bitmask_total = 0;
            bitmask_total += tiles.contains_key(&(tile_pos.x, tile_pos.y + 1, *elevation, *land))
                as u8
                * 2_u8.pow(0);
            bitmask_total += tiles.contains_key(&(tile_pos.x - 1, tile_pos.y, *elevation, *land))
                as u8
                * 2_u8.pow(1);
            bitmask_total += tiles.contains_key(&(tile_pos.x + 1, tile_pos.y, *elevation, *land))
                as u8
                * 2_u8.pow(2);
            bitmask_total += tiles.contains_key(&(tile_pos.x, tile_pos.y - 1, *elevation, *land))
                as u8
                * 2_u8.pow(3);
            atlas.index = assets.index_from_bitmask(bitmask_total as u8);
        } else {
            if let Some(true) = tiles.get(&(tile_pos.x, tile_pos.y - 1, *elevation, *land)) {
                let bitmask_up = assets.bitmask_from_index(atlas.index) + BITMASK_BOT;
                atlas.index = assets.index_from_bitmask(bitmask_up);
            }
            if let Some(true) = tiles.get(&(tile_pos.x + 1, tile_pos.y, *elevation, *land)) {
                let bitmask_left = assets.bitmask_from_index(atlas.index) + BITMASK_RIGHT;
                atlas.index = assets.index_from_bitmask(bitmask_left);
            }
            if let Some(true) = tiles.get(&(tile_pos.x - 1, tile_pos.y, *elevation, *land)) {
                let bitmask_right = assets.bitmask_from_index(atlas.index) + BITMASK_LEFT;
                atlas.index = assets.index_from_bitmask(bitmask_right);
            }
            if let Some(true) = tiles.get(&(tile_pos.x, tile_pos.y + 1, *elevation, *land)) {
                let bitmask_down = assets.bitmask_from_index(atlas.index) + BITMASK_TOP;
                atlas.index = assets.index_from_bitmask(bitmask_down);
            }
        }
    }
}

fn update_cliff_atlas_index(
    mut update_tile_read: EventReader<AdjustElevation>,
    mut tile_sprite_q: Query<(&mut TextureAtlas, &GlobalTransform, &Elevation), With<Cliff>>,
    assets: Res<WorldAssets>,
) {
    // could collect all the read's into a vector then bulk update
    for AdjustElevation {
        pos: target_position,
        elevation: target_elevation,
    } in update_tile_read.read()
    {
        let update_pos = target_position.as_ivec2();
        let mut bitmask_total: u8 = 0;
        for (mut texture_atlas, tile_pos) in
            &mut tile_sprite_q
                .iter_mut()
                .filter_map(|(atlas, transform, Elevation(elevation))| {
                    if elevation == target_elevation {
                        let tile_pos = (transform.translation().truncate() / TILE_VEC)
                            .floor()
                            .as_u16vec2();
                        Some((atlas, tile_pos))
                    } else {
                        None
                    }
                })
        {
            let current_pos = tile_pos.as_ivec2();
            if current_pos == update_pos - IVec2::X {
                bitmask_total += 2_u8.pow(0);
                let bitmask_left = assets.bitmask_from_cliff_index(texture_atlas.index) + 2;
                texture_atlas.index = assets.cliff_index_from_bitmask(bitmask_left);
            }
            if current_pos == update_pos + IVec2::X {
                bitmask_total += 2_u8.pow(1);
                let bitmask_right = assets.bitmask_from_cliff_index(texture_atlas.index) + 1;
                texture_atlas.index = assets.cliff_index_from_bitmask(bitmask_right);
            }
        }
        if let Some((mut atlas, _)) = tile_sprite_q
            .iter_mut()
            .filter_map(|(atlas, transform, Elevation(elevation))| {
                if elevation == target_elevation {
                    let tile_pos = (transform.translation().truncate() / TILE_VEC)
                        .floor()
                        .as_u16vec2();
                    Some((atlas, tile_pos))
                } else {
                    None
                }
            })
            .find(|(_, tile_pos)| tile_pos.as_ivec2() == update_pos)
        {
            let new_idx = assets.cliff_index_from_bitmask(bitmask_total);
            atlas.index = new_idx;
        }
    }
}

// todo: Should probably just do a whole refresh when any tile change occurs as right now it's complex
// to figure out what need's changing where, we can use `Changed<T>` to figure out what need's
// updating
// we still need the neighbours, one method would be to not use the ECS for this. But I'm
// interested in pushing the ECS to see what can occur.
// Eventually I'll take the despawning and respawning of another project and store the rest of the
// ECS in either a file or in memory or both. The cool thing about that is then the ECS will only
// load what the player can feasibly interact with while the rest of the systems store the full map
// info.
fn update_platau_atlas_index(
    mut update_tile_read: EventReader<AdjustElevation>,
    mut tile_sprite_q: Query<(&mut TextureAtlas, &GlobalTransform, &Elevation), With<Platau>>,
    assets: Res<WorldAssets>,
) {
    for AdjustElevation {
        pos: target_position,
        elevation: target_elevation,
    } in update_tile_read.read()
    {
        let update_pos = target_position.as_ivec2() + IVec2::Y; // * target_elevation
        let mut bitmask_total: u8 = 0;
        for (mut texture_atlas, tile_pos) in
            &mut tile_sprite_q
                .iter_mut()
                .filter_map(|(atlas, transform, Elevation(elevation))| {
                    if elevation == target_elevation {
                        let tile_pos = (transform.translation().truncate() / TILE_VEC)
                            .floor()
                            .as_u16vec2();
                        Some((atlas, tile_pos))
                    } else {
                        None
                    }
                })
        {
            let current_pos = tile_pos.as_ivec2();
            if current_pos == update_pos + IVec2::Y {
                bitmask_total += 2_u8.pow(0);
                let bitmask_top =
                    assets.bitmask_from_platau_index(texture_atlas.index) + BITMASK_BOT;
                texture_atlas.index = assets.platau_index_from_bitmask(bitmask_top);
            }
            if current_pos == update_pos - IVec2::X {
                bitmask_total += 2_u8.pow(1);
                let bitmask_left =
                    assets.bitmask_from_platau_index(texture_atlas.index) + BITMASK_RIGHT;
                texture_atlas.index = assets.platau_index_from_bitmask(bitmask_left);
            }
            if current_pos == update_pos + IVec2::X {
                bitmask_total += 2_u8.pow(2);
                let bitmask_right =
                    assets.bitmask_from_platau_index(texture_atlas.index) + BITMASK_LEFT;
                texture_atlas.index = assets.platau_index_from_bitmask(bitmask_right);
            }
            if current_pos == update_pos - IVec2::Y {
                bitmask_total += 2_u8.pow(3);
                let bitmask_bot =
                    assets.bitmask_from_platau_index(texture_atlas.index) + BITMASK_TOP;
                texture_atlas.index = assets.platau_index_from_bitmask(bitmask_bot);
            }
        }
        let new_idx = assets.platau_index_from_bitmask(bitmask_total);
        if let Some((mut atlas, _)) = tile_sprite_q
            .iter_mut()
            .filter_map(|(atlas, transform, Elevation(elevation))| {
                if elevation == target_elevation {
                    let tile_pos = (transform.translation().truncate() / TILE_VEC)
                        .floor()
                        .as_u16vec2();
                    Some((atlas, tile_pos))
                } else {
                    None
                }
            })
            .find(|(_, tile_pos)| tile_pos.as_ivec2() == update_pos)
        {
            atlas.index = new_idx;
        }
    }
}
fn place_land(
    mut cmds: Commands,
    mut tile_q: Query<(Entity, &mut Tile, &mut Elevation)>,
    assets: Res<WorldAssets>,
    mut place_land_events: EventReader<PlaceLandTile>,
) {
    for PlaceLandTile {
        pos: tile_pos,
        ground: _,
        elevation: _,
    } in place_land_events.read().filter(
        |PlaceLandTile {
             ground,
             pos: tile_pos,
             ..
         }| {
            *ground == Land::Sand && tile_pos.x < WORLD_SIZE.x && tile_pos.y < WORLD_SIZE.y
        },
    ) {
        let mut found = false;
        for (entity, mut tile, elevation) in &mut tile_q {
            if tile.pos == *tile_pos && elevation.0 == 0 {
                match tile.tile_type {
                    TileType::SeaCliff { has_top } | TileType::LandCliff { has_top }
                        if !has_top =>
                    {
                        let z_offset = elevation.0 as f32 + (WORLD_SIZE.y - tile_pos.y) as f32;
                        let ground_sprite =
                            assets.sand(Vec2::Y * TILE_SIZE * elevation.0 as f32, z_offset + 0.5);
                        let ground_entity = cmds
                            .spawn((ground_sprite, Land::Sand, Elevation(elevation.0)))
                            .id();
                        cmds.entity(entity).push_children(&[ground_entity]);
                        tile.add_top();
                    }
                    _ => {}
                }
            }
            if tile.pos == *tile_pos {
                found = true;
                break;
            }
        }
        if found == false {
            assets.spawn_ground(&mut cmds, *tile_pos);
        }
    }
}

// todo(KISS): Despawn the tile and spawn the new one, could even go as far to bitmask the tiles
// this would be neat as only the ones at at the bottom edge need a cliff face, if they're
// connected they don't.
//
// Also tiles above the elevation don't need a cliff face if theres a tile below them
//
// grass and sand need an elevation on each tile, so 0 elevation means they're at the bottom of the
// and any elevation above them need's a matching crumbs on the cliff face
//
// The above to say I think boolean rules make sense here :shrug:
// fn update_editor_tile_system(
//     mut cmds: Commands,
//     window_q: Query<&Window>,
//     camera_q: Query<(&Camera, &mut GlobalTransform), With<GameCamera>>,
//     mut tile_q: Query<(Entity, &mut Tile, &mut Elevation)>,
//     mouse_button: Res<ButtonInput<MouseButton>>,
//     assets: Res<WorldAssets>,
//     mut update_ground_write: EventWriter<UpdateLand>,
//     mut update_platau_write: EventWriter<UpdateCliff>,
//     mut gizmos: Gizmos,
// ) {
//     if let Ok(window) = window_q.get_single() {
//         for (camera, camera_transform) in camera_q.iter() {
//             if let Some(cursor_pos) = window.cursor_position() {
//                 if let Some(world_cursor_pos) =
//                     camera.viewport_to_world_2d(camera_transform, cursor_pos)
//                 {
//                     let tile_pos = (world_cursor_pos / TILE_VEC).floor().as_u16vec2();
//                     let mut selected_elevation: u8 = 0;
//                     for (_, tile, elevation) in &tile_q {
//                         if tile.pos == tile_pos {
//                             selected_elevation = elevation.0.max(selected_elevation);
//                             break;
//                         };
//                     }

//                     if mouse_button.just_pressed(MouseButton::Left) {
//                         editor_opts.elevation = selected_elevation;
//                     } else if mouse_button.just_released(MouseButton::Left) {
//                         editor_opts.elevation = 0;
//                     }

//                     gizmos.rect_2d(
//                         tile_pos.as_vec2() * TILE_VEC
//                             + (TILE_VEC / 2.0)
//                             + (Vec2::Y * TILE_VEC / 2.0) * selected_elevation as f32,
//                         0.0,
//                         TILE_VEC * Vec2::new(1.0, 1.0 * (selected_elevation + 1) as f32),
//                         Color::GREEN,
//                     );

//                     if mouse_button.pressed(MouseButton::Left)
//                         && editor_opts.selected == EditorTileType::Sand
//                     {
//                         let mut found = false;
//                         for (entity, mut tile, elevation) in &mut tile_q {
//                             if tile.pos == tile_pos && elevation.0 == editor_opts.elevation {
//                                 match tile.tile_type {
//                                     TileType::SeaCliff { has_top }
//                                     | TileType::LandCliff { has_top }
//                                         if !has_top =>
//                                     {
//                                         let z_offset =
//                                             elevation.0 as f32 + (WORLD_SIZE.y - tile_pos.y) as f32;
//                                         let ground_sprite = assets.sand(
//                                             Vec2::Y * TILE_SIZE * elevation.0 as f32,
//                                             z_offset + 0.5,
//                                         );
//                                         let ground_entity = cmds
//                                             .spawn((
//                                                 ground_sprite,
//                                                 Land::Sand,
//                                                 Elevation(elevation.0),
//                                             ))
//                                             .id();
//                                         cmds.entity(entity).push_children(&[ground_entity]);
//                                         update_ground_write.send(UpdateLand {
//                                             pos: tile_pos + U16Vec2::Y * elevation.0 as u16,
//                                             ground: Land::Sand,
//                                             elevation: elevation.0,
//                                         });
//                                         tile.add_top();
//                                     }
//                                     _ => {}
//                                 }
//                             }
//                             if tile.pos == tile_pos {
//                                 found = true;
//                                 break;
//                             }
//                         }
//                         if found == false && editor_opts.elevation == 0 {
//                             assets.spawn_ground(&mut cmds, tile_pos);
//                             update_ground_write.send(UpdateLand {
//                                 pos: tile_pos,
//                                 ground: Land::Sand,
//                                 elevation: 0,
//                             });
//                         }
//                     }

//                     // todo: If the current height is at the editor opts height +1
//                     // then we search through and check if theres
//                     if mouse_button.pressed(MouseButton::Left)
//                         && editor_opts.selected == EditorTileType::IncreaseElevation
//                     {
//                         let mut found = false;
//                         for (entity, mut tile, mut elevation) in &mut tile_q {
//                             if tile.pos == tile_pos {
//                                 let z_offset =
//                                     elevation.0 as f32 + (WORLD_SIZE.y - tile_pos.y) as f32;
//                                 match tile.tile_type {
//                                     TileType::LandCliff { has_top: false }
//                                     | TileType::SeaCliff { has_top: false }
//                                         if elevation.0 == editor_opts.elevation =>
//                                     {
//                                         elevation.0 = editor_opts.elevation + 1;
//                                         let wall_idx = assets.cliff_index_from_bitmask(0);
//                                         let platau_idx = assets.platau_index_from_bitmask(0);
//                                         let platau_sprite = assets.cliff(
//                                             platau_idx as u8,
//                                             Vec2::Y * TILE_SIZE * elevation.0 as f32,
//                                             z_offset + 0.4,
//                                         );
//                                         let platau_entity = cmds
//                                             .spawn((platau_sprite, Platau, elevation.clone()))
//                                             .id();
//                                         let wall_sprite = assets.cliff(
//                                             wall_idx as u8,
//                                             Vec2::Y * TILE_SIZE * (elevation.0 - 1) as f32,
//                                             z_offset + 0.5,
//                                         );
//                                         let wall_entity = cmds
//                                             .spawn((wall_sprite, Cliff, elevation.clone()))
//                                             .id();
//                                         cmds.entity(entity)
//                                             .push_children(&[platau_entity, wall_entity]);
//                                         update_platau_write.send(UpdateCliff {
//                                             pos: tile_pos
//                                                 + (U16Vec2::Y * editor_opts.elevation as u16),
//                                             elevation: editor_opts.elevation + 1,
//                                         });
//                                     }
//                                     TileType::Land => {
//                                         elevation.0 = editor_opts.elevation + 1;
//                                         let wall_idx = assets.cliff_index_from_bitmask(0);
//                                         let platau_idx = assets.platau_index_from_bitmask(0);
//                                         let platau_sprite = assets.cliff(
//                                             platau_idx as u8,
//                                             Vec2::Y * TILE_SIZE,
//                                             z_offset + 0.4,
//                                         );
//                                         let crumbs = assets.sand_from(
//                                             WorldAssets::CRUMBS as u8,
//                                             Vec2::Y,
//                                             z_offset + 0.3,
//                                         );
//                                         let wall_sprite = assets.cliff(
//                                             wall_idx as u8,
//                                             Vec2::ZERO,
//                                             z_offset + 0.2,
//                                         );
//                                         let crumbs_entity = cmds.spawn(crumbs).id();
//                                         let platau_entity = cmds
//                                             .spawn((platau_sprite, Platau, elevation.clone()))
//                                             .id();
//                                         let wall_entity = cmds
//                                             .spawn((wall_sprite, Cliff, elevation.clone()))
//                                             .id();
//                                         let shadow = assets.shadow(TILE_VEC * 0.5, z_offset + 0.1);
//                                         let shadow_entity = cmds.spawn(shadow).id();
//                                         cmds.entity(entity).push_children(&[
//                                             crumbs_entity,
//                                             platau_entity,
//                                             wall_entity,
//                                             shadow_entity,
//                                         ]);
//                                         update_platau_write.send(UpdateCliff {
//                                             pos: tile_pos,
//                                             elevation: elevation.0,
//                                         });
//                                         tile.tile_type = TileType::LandCliff { has_top: false };
//                                     }
//                                     _ => {}
//                                 }
//                                 found = true;
//                                 break;
//                             }
//                         }
//                         if found == false {
//                             assets.spawn_sea_cliff(&mut cmds, tile_pos, editor_opts.elevation + 1);
//                             update_platau_write.send(UpdateCliff {
//                                 pos: tile_pos + (U16Vec2::Y * editor_opts.elevation as u16),
//                                 elevation: editor_opts.elevation + 1,
//                             });
//                         }
//                     }
//                 }
//             }
//         }
//     }
// }

fn setup_tile_system(mut cmds: Commands, assets: Res<WorldAssets>) {}
