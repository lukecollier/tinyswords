use std::time::Duration;

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle};
use bevy::utils::hashbrown::HashMap;
use bevy::utils::HashSet;
/**
 * This is the plugin for the world, it's animations, and creating blocking
 */
use bevy::{math::U16Vec2, sprite::Anchor};
use bevy_asset_loader::prelude::*;

pub const WORLD_SIZE: U16Vec2 = U16Vec2::new(32, 32);
pub const TILE_SIZE: f32 = 64.0;
pub const TILE_VEC: Vec2 = Vec2::new(TILE_SIZE, TILE_SIZE);

pub fn map_bounds() -> Rect {
    Rect::new(
        0.,
        0.,
        TILE_SIZE * WORLD_SIZE.x as f32,
        TILE_SIZE * WORLD_SIZE.y as f32,
    )
}

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

#[derive(Component, PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub struct Elevation(pub u8);

impl Default for Elevation {
    fn default() -> Self {
        Self(0)
    }
}

// todo(improvement): Use this to replace the TileKind
// search children of tile for land types and their elevation
#[derive(Component, PartialEq, Eq, Clone, Copy, Debug, Hash)]
enum Land {
    Sand,
    Grass,
}

#[derive(Component, PartialEq, Eq, Clone, Copy, Debug, Hash)]
struct Platau;
#[derive(Component, PartialEq, Eq, Clone, Copy, Debug, Hash)]
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
    pub land_layout: Handle<TextureAtlasLayout>,

    #[asset(path = "terrain/ground/tilemap_grass.png")]
    pub grass_texture: Handle<Image>,

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
    fn grass(&self, xy: Vec2, z: f32) -> SpriteSheetBundle {
        let layout = self.land_layout.clone();
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
                index: WorldAssets::ISOLATE as usize,
            },
            ..default()
        }
    }
    fn sand(&self, xy: Vec2, z: f32) -> SpriteSheetBundle {
        let layout = self.land_layout.clone();
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

    fn cliff_detail(&self, cmds: &mut Commands, xy: U16Vec2, height: u8) -> Vec<Entity> {
        assert_ne!(height, 0);
        let z_offset = height as f32 + (WORLD_SIZE.y - xy.y) as f32;
        let wall_idx = self.cliff_index_from_bitmask(0);
        let shadow = self.shadow(TILE_VEC * 0.5, z_offset);
        let platau_idx = self.platau_index_from_bitmask(WorldAssets::ISOLATE as u8);
        let mut children = vec![];
        children.push(
            cmds.spawn((
                self.coast(TILE_VEC * 0.5, -z_offset - 1.0),
                GloballyAnimated::new(7),
            ))
            .id(),
        );
        for i in 1..=height {
            let platau_sprite = self.cliff(
                platau_idx as u8,
                Vec2::Y * TILE_SIZE * i as f32,
                z_offset + 0.3 + i as f32,
            );
            children.push(cmds.spawn((platau_sprite, Platau, Elevation(i))).id());
            let wall_sprite = self.cliff(
                wall_idx as u8,
                Vec2::Y * TILE_SIZE * (i - 1) as f32,
                z_offset + 0.1 + i as f32,
            );
            children.push(cmds.spawn((wall_sprite, Cliff, Elevation(i - 1))).id());
        }
        children.push(cmds.spawn(shadow).id());
        children
    }

    pub fn spawn_grass_on_sand(
        &self,
        cmds: &mut Commands,
        x: u16,
        y: u16,
        elevation: u8,
    ) -> Entity {
        let tile = TileBundle::new(x, y, elevation);
        cmds.spawn((tile, Land::Grass))
            .with_children(|parent| {
                let z_offset = (WORLD_SIZE.y - y) as f32 + elevation as f32;
                parent.spawn((
                    self.sand(Vec2::ZERO, z_offset - 0.01),
                    Land::Sand,
                    Elevation(elevation),
                ));
                parent.spawn((
                    self.grass(Vec2::ZERO, z_offset),
                    Land::Grass,
                    Elevation(elevation),
                ));
                parent.spawn((
                    self.coast(TILE_VEC * 0.5, -z_offset - 1.0),
                    GloballyAnimated::new(7),
                ));
            })
            .id()
    }

    pub fn spawn_sand(&self, cmds: &mut Commands, x: u16, y: u16, elevation: u8) -> Entity {
        let tile = TileBundle::new(x, y, elevation);
        cmds.spawn((tile, Land::Sand))
            .with_children(|parent| {
                let z_offset = (WORLD_SIZE.y - y) as f32 + elevation as f32;
                let sprite = self.sand(Vec2::ZERO, z_offset);
                parent.spawn((sprite, Land::Sand, Elevation(elevation)));
                parent.spawn((
                    self.coast(TILE_VEC * 0.5, -z_offset - 1.0),
                    GloballyAnimated::new(7),
                ));
            })
            .id()
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
    or_state: S,
    loading_state: S,
}

impl<S: States> Plugin for WorldPlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_loading_state(
            LoadingStateConfig::new(self.loading_state.clone()).load_collection::<WorldAssets>(),
        )
        .add_systems(
            OnTransition {
                from: self.loading_state.clone(),
                to: self.or_state.clone(),
            },
            (setup_tile_system, setup_water),
        )
        .add_systems(
            OnTransition {
                from: self.loading_state.clone(),
                to: self.state.clone(),
            },
            (setup_tile_system, setup_water),
        )
        .init_resource::<GlobalAnimation>()
        .init_resource::<TileMap>()
        .add_plugins(Material2dPlugin::<WaterMaterial>::default())
        .add_systems(
            Update,
            (
                update_remove_tile,
                update_register_new_tile,
                update_added_land_atlas_index,
                update_changed_cliff_atlas_index,
                update_changed_platau_atlas_index,
                update_animated_tiles,
                update_tile_elevation,
            )
                .run_if(in_state(self.state.clone()).or_else(in_state(self.or_state.clone()))),
        );
    }
}

impl<S: States> WorldPlugin<S> {
    pub fn run_on_state_or(state: S, or_state: S, loading_state: S) -> Self {
        Self {
            state,
            or_state,
            loading_state,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct Tile {
    pub pos: U16Vec2,
}

impl Tile {
    fn new(x: u16, y: u16) -> Self {
        Self {
            pos: U16Vec2::new(x, y),
        }
    }
}

#[derive(Bundle, Clone)]
pub struct TileBundle {
    pub tile: Tile,
    pub elevation: Elevation,
    pub transform: TransformBundle,
    pub visibility: VisibilityBundle,
}

#[derive(Component)]
pub struct DeleteTile;

impl Default for TileBundle {
    fn default() -> Self {
        TileBundle {
            tile: Tile::new(0, 0),
            elevation: Elevation::default(),
            visibility: VisibilityBundle::default(),
            transform: TransformBundle::default(),
        }
    }
}

impl TileBundle {
    pub fn new(x: u16, y: u16, elevation: u8) -> Self {
        TileBundle {
            tile: Tile::new(x, y),
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
            ..default()
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

#[derive(Resource, Default, Debug)]
pub struct TileMap {
    tiles: HashMap<(u16, u16), u8>,
}

impl TileMap {
    pub fn is_occupied(&self, x: u16, y: u16) -> bool {
        self.tiles.contains_key(&(x as u16, y as u16))
    }

    pub fn get_elevation(&self, x: u16, y: u16) -> Option<&u8> {
        self.tiles.get(&(x as u16, y as u16))
    }

    pub fn occupy(&mut self, x: u16, y: u16, elevation: u8) {
        self.tiles.insert((x as u16, y as u16), elevation);
    }
}

fn update_register_new_tile(
    tiles_q: Query<(&Tile, &Elevation), Added<Tile>>,
    mut tile_map: ResMut<TileMap>,
) {
    for (tile, elevation) in &tiles_q {
        tile_map.tiles.insert((tile.pos.x, tile.pos.y), elevation.0);
    }
}

fn update_remove_tile(
    mut cmds: Commands,
    removed_tiles_q: Query<(Entity, &Tile), With<DeleteTile>>,
    mut tile_map: ResMut<TileMap>,
) {
    for (entity, tile) in &removed_tiles_q {
        tile_map.tiles.remove(&(tile.pos.x, tile.pos.y));
        if let Some(found) = cmds.get_entity(entity) {
            found.despawn_recursive();
        }
    }
}

// todo(improvement): We should only spawn crumbs when the tiles surrounding the point at the base
// are
fn update_tile_elevation(
    mut cmds: Commands,
    tiles_q: Query<(Entity, Ref<Elevation>, &Tile)>,
    children_q: Query<&Children>,
    assets: Res<WorldAssets>,
) {
    for (entity, elevation, tile) in &tiles_q {
        if elevation.is_changed() || elevation.is_added() {
            // 1. we clear the children completely
            // 2. we spawn a new stack of children dependent on the world around the tile map
            // - if tile below is water, spawn a sea cliff
            // - if tile below is land, spawn a land cliff
            if elevation.0 > 0 {
                if let Ok(children) = children_q.get(entity) {
                    children.iter().for_each(|child| {
                        cmds.entity(*child).despawn_recursive();
                    });
                }
                let details = assets.cliff_detail(&mut cmds, tile.pos, elevation.0);
                cmds.entity(entity).push_children(&details);
            }
        }
    }
}

// todo: Elevation can be replaced by the z index on the transform
fn update_added_land_atlas_index(
    mut tiles_q: Query<(&mut TextureAtlas, &GlobalTransform, &Elevation, Ref<Land>)>,
    assets: Res<WorldAssets>,
) {
    let mut tiles: HashMap<(i16, i16, Elevation, Land), bool> =
        HashMap::with_capacity(tiles_q.iter().len());
    // todo: Cache this in resource so we can avoid recalculations
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

fn update_changed_cliff_atlas_index(
    mut tiles_q: Query<(&mut TextureAtlas, &GlobalTransform, &Elevation, Ref<Cliff>)>,
    assets: Res<WorldAssets>,
) {
    let mut tiles: HashMap<(i16, i16, Elevation, Cliff), bool> =
        HashMap::with_capacity(tiles_q.iter().len());
    // todo: Cache this in resource so we can avoid recalculations
    for (_, transform, elevation, cliff) in &tiles_q {
        let tile_pos = (transform.translation().truncate() / TILE_VEC)
            .floor()
            .as_i16vec2();
        tiles.insert(
            (tile_pos.x, tile_pos.y, *elevation, *cliff),
            cliff.is_added(),
        );
    }
    // we then use the coordinates to get a map of the neighbours and their land type
    for (mut atlas, transform, elevation, cliff) in &mut tiles_q {
        let tile_pos = (transform.translation().truncate() / TILE_VEC)
            .floor()
            .as_i16vec2();
        if cliff.is_added() {
            let mut bitmask_total = 0;
            bitmask_total += tiles.contains_key(&(tile_pos.x - 1, tile_pos.y, *elevation, *cliff))
                as u8
                * 2_u8.pow(0);
            bitmask_total += tiles.contains_key(&(tile_pos.x + 1, tile_pos.y, *elevation, *cliff))
                as u8
                * 2_u8.pow(1);
            atlas.index = assets.cliff_index_from_bitmask(bitmask_total as u8);
        } else {
            if let Some(true) = tiles.get(&(tile_pos.x - 1, tile_pos.y, *elevation, *cliff)) {
                let bitmask_left = assets.bitmask_from_cliff_index(atlas.index) + 1;
                atlas.index = assets.cliff_index_from_bitmask(bitmask_left);
            }
            if let Some(true) = tiles.get(&(tile_pos.x + 1, tile_pos.y, *elevation, *cliff)) {
                let bitmask_right = assets.bitmask_from_cliff_index(atlas.index) + 2;
                atlas.index = assets.cliff_index_from_bitmask(bitmask_right);
            }
        }
    }
}

fn update_changed_platau_atlas_index(
    mut tiles_q: Query<(&mut TextureAtlas, &GlobalTransform, &Elevation, Ref<Platau>)>,
    assets: Res<WorldAssets>,
) {
    // todo: Cache using a Local<T> resoruce
    let mut tiles: HashMap<(i16, i16, Elevation, Platau), bool> =
        HashMap::with_capacity(tiles_q.iter().len());
    // todo: Cache this in resource so we can avoid recalculations
    for (_, transform, elevation, platau) in &tiles_q {
        let tile_pos = (transform.translation().truncate() / TILE_VEC)
            .floor()
            .as_i16vec2();
        tiles.insert(
            (tile_pos.x, tile_pos.y, *elevation, *platau),
            platau.is_added(),
        );
    }
    // we then use the coordinates to get a map of the neighbours and their land type
    for (mut atlas, transform, elevation, platau) in &mut tiles_q {
        let tile_pos = (transform.translation().truncate() / TILE_VEC)
            .floor()
            .as_i16vec2();
        if platau.is_added() {
            let mut bitmask_total = 0;
            bitmask_total += tiles.contains_key(&(tile_pos.x, tile_pos.y + 1, *elevation, *platau))
                as u8
                * 2_u8.pow(0);
            bitmask_total += tiles.contains_key(&(tile_pos.x - 1, tile_pos.y, *elevation, *platau))
                as u8
                * 2_u8.pow(1);
            bitmask_total += tiles.contains_key(&(tile_pos.x + 1, tile_pos.y, *elevation, *platau))
                as u8
                * 2_u8.pow(2);
            bitmask_total += tiles.contains_key(&(tile_pos.x, tile_pos.y - 1, *elevation, *platau))
                as u8
                * 2_u8.pow(3);
            atlas.index = assets.platau_index_from_bitmask(bitmask_total as u8);
        } else {
            if let Some(true) = tiles.get(&(tile_pos.x, tile_pos.y - 1, *elevation, *platau)) {
                let bitmask_up = assets.bitmask_from_platau_index(atlas.index) + BITMASK_BOT;
                atlas.index = assets.platau_index_from_bitmask(bitmask_up);
            }
            if let Some(true) = tiles.get(&(tile_pos.x + 1, tile_pos.y, *elevation, *platau)) {
                let bitmask_left = assets.bitmask_from_platau_index(atlas.index) + BITMASK_RIGHT;
                atlas.index = assets.platau_index_from_bitmask(bitmask_left);
            }
            if let Some(true) = tiles.get(&(tile_pos.x - 1, tile_pos.y, *elevation, *platau)) {
                let bitmask_right = assets.bitmask_from_platau_index(atlas.index) + BITMASK_LEFT;
                atlas.index = assets.platau_index_from_bitmask(bitmask_right);
            }
            if let Some(true) = tiles.get(&(tile_pos.x, tile_pos.y + 1, *elevation, *platau)) {
                let bitmask_down = assets.bitmask_from_platau_index(atlas.index) + BITMASK_TOP;
                atlas.index = assets.platau_index_from_bitmask(bitmask_down);
            }
        }
    }
}

fn setup_tile_system(mut cmds: Commands, assets: Res<WorldAssets>) {}
