use std::time::Duration;

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::sprite::{Material2d, Material2dPlugin};
use bevy::utils::hashbrown::HashMap;
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

#[derive(Component)]
struct DespawnOnElevationChange;
#[derive(Component)]
struct DontRegisterLand;
#[derive(Component)]
struct CliffLand;
#[derive(Component)]
struct Crumbs;
#[derive(Component, PartialEq, Eq, Clone, Copy, Debug, Hash)]
struct Platau;
#[derive(Component, PartialEq, Eq, Clone, Copy, Debug, Hash)]
struct Coast;
#[derive(Component, PartialEq, Eq, Clone, Copy, Debug, Hash)]
struct Cliff;
#[derive(Component)]
struct Shadow;

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

    #[asset(texture_atlas_layout(tile_size_x = 192, tile_size_y = 192, columns = 8, rows = 1))]
    pub coast_layout: Handle<TextureAtlasLayout>,

    #[asset(path = "terrain/ground/tilemap_sand.png")]
    pub sand_texture: Handle<Image>,

    #[asset(texture_atlas_layout(tile_size_x = 64, tile_size_y = 64, columns = 5, rows = 4))]
    pub land_layout: Handle<TextureAtlasLayout>,

    #[asset(path = "terrain/ground/tilemap_grass.png")]
    pub grass_texture: Handle<Image>,

    #[asset(path = "terrain/ground/tilemap_cliff.png")]
    pub cliff_texture: Handle<Image>,

    #[asset(texture_atlas_layout(tile_size_x = 64, tile_size_y = 64, columns = 4, rows = 7))]
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

    fn shadow(&self) -> Sprite {
        let texture = self.shadow_texture.clone();
        let mut sprite = Sprite::from_image(texture);
        sprite.anchor = Anchor::Center;
        sprite
    }

    fn coast(&self) -> Sprite {
        let texture = self.coast_texture.clone();
        let layout = self.coast_layout.clone();
        let atlas = TextureAtlas { layout, index: 0 };
        let mut sprite = Sprite::from_atlas_image(texture, atlas);
        sprite.anchor = Anchor::Center;
        sprite
    }

    fn cliff(&self, idx: u8) -> Sprite {
        if idx > 4 * 7 {
            panic!("out of bounds");
        }
        let layout = self.cliff_layout.clone();
        let texture = self.cliff_texture.clone();
        let atlas = TextureAtlas {
            layout,
            index: idx as usize,
        };
        let mut sprite = Sprite::from_atlas_image(texture, atlas);
        sprite.anchor = Anchor::BottomLeft;
        sprite
    }

    fn grass(&self) -> Sprite {
        let layout = self.land_layout.clone();
        let texture = self.grass_texture.clone();
        let atlas = TextureAtlas {
            layout,
            index: WorldAssets::ISOLATE as usize,
        };
        let mut sprite = Sprite::from_atlas_image(texture, atlas);
        sprite.anchor = Anchor::BottomLeft;
        sprite
    }

    fn sand(&self) -> Sprite {
        let layout = self.land_layout.clone();
        let texture = self.sand_texture.clone();
        let atlas = TextureAtlas {
            layout,
            index: WorldAssets::ISOLATE as usize,
        };
        let mut sprite = Sprite::from_atlas_image(texture, atlas);
        sprite.anchor = Anchor::BottomLeft;
        sprite
    }

    fn grass_crumbs(&self) -> Sprite {
        let layout = self.land_layout.clone();
        let texture = self.grass_texture.clone();
        let atlas = TextureAtlas {
            layout,
            index: WorldAssets::CRUMBS as usize,
        };
        let mut sprite = Sprite::from_atlas_image(texture, atlas);
        sprite.anchor = Anchor::BottomLeft;
        sprite
    }

    fn sand_crumbs(&self) -> Sprite {
        let layout = self.land_layout.clone();
        let texture = self.sand_texture.clone();
        let atlas = TextureAtlas {
            layout,
            index: WorldAssets::CRUMBS as usize,
        };
        let mut sprite = Sprite::from_atlas_image(texture, atlas);
        sprite.anchor = Anchor::BottomLeft;
        sprite
    }

    fn cliff_detail(&self, cmds: &mut Commands, height: u8) -> Vec<Entity> {
        assert_ne!(height, 0);
        let wall_idx = self.cliff_index_from_bitmask(0);
        let shadow = self.shadow();
        let platau_idx = self.platau_index_from_bitmask(WorldAssets::ISOLATE as u8);
        let mut children = vec![];
        for i in 1..=height {
            let platau_sprite = self.cliff(platau_idx as u8);
            children.push(
                cmds.spawn((
                    Transform::from_translation(
                        (Vec2::Y * TILE_SIZE * i as f32).extend(0.4 + i as f32),
                    ),
                    platau_sprite,
                    Platau,
                    DespawnOnElevationChange,
                    Elevation(i),
                ))
                .id(),
            );
            let wall_sprite = self.cliff(wall_idx as u8);
            children.push(
                cmds.spawn((
                    wall_sprite,
                    Transform::from_translation(
                        (Vec2::Y * TILE_SIZE * (i - 1) as f32).extend(0.4 + (i - 1) as f32),
                    ),
                    Cliff,
                    DespawnOnElevationChange,
                    Elevation(i - 1),
                ))
                .id(),
            );
        }
        // todo(improvement): Shadow could work like coast lines to automatically get cleaned up
        // via changes
        children.push(
            cmds.spawn((
                shadow,
                Transform::from_translation((TILE_VEC * 0.5).extend(0.05)),
                Shadow,
                DespawnOnElevationChange,
            ))
            .id(),
        );
        children
    }

    pub fn spawn_grass(&self, cmds: &mut Commands, x: u16, y: u16, elevation: u8) -> Entity {
        let tile = TileBundle::new(x, y, elevation);
        cmds.spawn(tile)
            .with_children(|parent| {
                parent.spawn((
                    self.grass(),
                    Transform::from_xyz(0., 0., 0.),
                    Land::Grass,
                    Elevation(0),
                ));
            })
            .id()
    }

    pub fn spawn_empty(&self, cmds: &mut Commands, x: u16, y: u16, elevation: u8) -> Entity {
        let tile = TileBundle::new(x, y, elevation);
        cmds.spawn(tile).id()
    }

    pub fn spawn_sand(&self, cmds: &mut Commands, x: u16, y: u16, elevation: u8) -> Entity {
        let tile = TileBundle::new(x, y, elevation);
        cmds.spawn(tile)
            .with_children(|parent| {
                let sprite = self.sand();
                let transform = Transform::from_translation(Vec3::ZERO);
                parent.spawn((sprite, transform, Land::Sand, Elevation(0)));
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
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(width, height)).into()),
        MeshMaterial2d(materials.add(WaterMaterial {
            color: Color::srgb(7.5, 0.0, 7.5),
        })),
        Transform::from_xyz(width / 2., height / 2., -100.),
    ));
}

fn update_meets_grass(
    mut cmds: Commands,
    land_q: Query<
        (Entity, &Land, &Elevation, &GlobalTransform),
        (Added<Land>, Without<DontRegisterLand>),
    >,
    children: Query<&Children>,
    assets: ResMut<WorldAssets>,
    mut land_map: ResMut<LandMap>,
) {
    for (entity, land, Elevation(elevation), transform) in &land_q {
        let tile_pos = (transform.translation().truncate() / TILE_VEC)
            .floor()
            .as_i16vec2();
        if *land == Land::Grass {
            let sand_neighbours =
                land_map.count_neighbours(tile_pos.x, tile_pos.y, *elevation, Land::Sand);
            if sand_neighbours > 0 {
                let sand = cmds
                    .spawn((
                        assets.sand(),
                        Transform::from_xyz(0., 0., -0.1),
                        Land::Sand,
                        DontRegisterLand,
                        Elevation(*elevation),
                    ))
                    .id();
                cmds.entity(entity).add_children(&[sand]);
            }
            let bot_sand =
                land_map.count_neighbours(tile_pos.x, tile_pos.y - 1, *elevation, Land::Sand);
            if bot_sand == 0 {
                if let Some(bot_entity) =
                    land_map.get(tile_pos.x, tile_pos.y - 1, *elevation, Land::Grass)
                {
                    // remove all children (the sand)
                    cmds.entity(*bot_entity).despawn_descendants();
                }
            }
            let top_sand =
                land_map.count_neighbours(tile_pos.x, tile_pos.y + 1, *elevation, Land::Sand);
            if top_sand == 0 {
                if let Some(bot_entity) =
                    land_map.get(tile_pos.x, tile_pos.y + 1, *elevation, Land::Grass)
                {
                    // remove all children (the sand)
                    cmds.entity(*bot_entity).despawn_descendants();
                }
            }
            let left_sand =
                land_map.count_neighbours(tile_pos.x - 1, tile_pos.y, *elevation, Land::Sand);
            if left_sand == 0 {
                if let Some(bot_entity) =
                    land_map.get(tile_pos.x - 1, tile_pos.y, *elevation, Land::Grass)
                {
                    // remove all children (the sand)
                    cmds.entity(*bot_entity).despawn_descendants();
                }
            }
            let right_sand =
                land_map.count_neighbours(tile_pos.x + 1, tile_pos.y, *elevation, Land::Sand);
            if right_sand == 0 {
                if let Some(bot_entity) =
                    land_map.get(tile_pos.x + 1, tile_pos.y, *elevation, Land::Grass)
                {
                    // remove all children (the sand)
                    cmds.entity(*bot_entity).despawn_descendants();
                }
            }
        }
        if *land == Land::Sand {
            let top_sand =
                land_map.count_neighbours(tile_pos.x, tile_pos.y - 1, *elevation, Land::Sand);
            if top_sand > 0 {
                if let Some(bot_entity) =
                    land_map.get(tile_pos.x, tile_pos.y - 1, *elevation, Land::Grass)
                {
                    if let Err(_) = children.get(*bot_entity) {
                        let sand = cmds
                            .spawn((
                                assets.sand(),
                                Transform::from_xyz(0., 0., -0.1),
                                Land::Sand,
                                DontRegisterLand,
                                Elevation(*elevation),
                            ))
                            .id();
                        cmds.entity(*bot_entity).add_children(&[sand]);
                    }
                }
            }

            let left_sand =
                land_map.count_neighbours(tile_pos.x - 1, tile_pos.y, *elevation, Land::Sand);
            if left_sand > 0 {
                if let Some(left_entity) =
                    land_map.get(tile_pos.x - 1, tile_pos.y, *elevation, Land::Grass)
                {
                    if let Err(_) = children.get(*left_entity) {
                        let sand = cmds
                            .spawn((
                                assets.sand(),
                                Transform::from_xyz(0., 0., -0.1),
                                Land::Sand,
                                DontRegisterLand,
                                Elevation(*elevation),
                            ))
                            .id();
                        cmds.entity(*left_entity).add_children(&[sand]);
                    }
                }
            }

            let right_sand =
                land_map.count_neighbours(tile_pos.x + 1, tile_pos.y, *elevation, Land::Sand);
            if right_sand > 0 {
                if let Some(right_entity) =
                    land_map.get(tile_pos.x + 1, tile_pos.y, *elevation, Land::Grass)
                {
                    if let Err(_) = children.get(*right_entity) {
                        let sand = cmds
                            .spawn((
                                assets.sand(),
                                Transform::from_xyz(0., 0., -0.1),
                                Land::Sand,
                                DontRegisterLand,
                                Elevation(*elevation),
                            ))
                            .id();
                        cmds.entity(*right_entity).add_children(&[sand]);
                    }
                }

                let bot_sand =
                    land_map.count_neighbours(tile_pos.x, tile_pos.y + 1, *elevation, Land::Sand);
                if bot_sand > 0 {
                    if let Some(bot_entity) =
                        land_map.get(tile_pos.x, tile_pos.y + 1, *elevation, Land::Grass)
                    {
                        if let Err(_) = children.get(*bot_entity) {
                            let sand = cmds
                                .spawn((
                                    assets.sand(),
                                    Transform::from_xyz(0., 0., -0.1),
                                    Land::Sand,
                                    DontRegisterLand,
                                    Elevation(*elevation),
                                ))
                                .id();
                            cmds.entity(*bot_entity).add_children(&[sand]);
                        }
                    }
                }
            }
        }
    }
}

fn update_coastline(
    mut cmds: Commands,
    world_assets: ResMut<WorldAssets>,
    query: Query<(Entity, &Tile), Added<Tile>>,
    children_q: Query<&Children, With<Tile>>,
    coast_q: Query<Entity, With<Coast>>,
    tile_map: Res<TileMap>,
) {
    for (entity, tile) in &query {
        let x = tile.pos.x as i32;
        let y = tile.pos.y as i32;
        let neighbours = tile_map.count_neighbours(x, y);
        if neighbours < 4 {
            let coast_entity = cmds
                .spawn((
                    world_assets.coast(),
                    Transform::from_translation((TILE_VEC * 0.5).extend(-100.0)),
                    GloballyAnimated::new(7),
                    Coast,
                ))
                .id();
            cmds.entity(entity).add_children(&[coast_entity]);
        }

        if tile_map.count_neighbours(x, y + 1) == 4 {
            if let Some(entity) = tile_map.get_entity(x, y + 1) {
                if let Ok(children) = children_q.get(*entity) {
                    for child in children {
                        coast_q.get(*child).ok().map(|entity| {
                            cmds.entity(entity).despawn_recursive();
                        });
                    }
                }
            };
        }
        if tile_map.count_neighbours(x, y - 1) == 4 {
            if let Some(entity) = tile_map.get_entity(x, y - 1) {
                if let Ok(children) = children_q.get(*entity) {
                    for child in children {
                        coast_q.get(*child).ok().map(|entity| {
                            cmds.entity(entity).despawn_recursive();
                        });
                    }
                }
            };
        }
        if tile_map.count_neighbours(x + 1, y) == 4 {
            if let Some(entity) = tile_map.get_entity(x + 1, y) {
                if let Ok(children) = children_q.get(*entity) {
                    for child in children {
                        coast_q.get(*child).ok().map(|entity| {
                            cmds.entity(entity).despawn_recursive();
                        });
                    }
                }
            };
        }
        if tile_map.count_neighbours(x - 1, y) == 4 {
            if let Some(entity) = tile_map.get_entity(x - 1, y) {
                if let Ok(children) = children_q.get(*entity) {
                    for child in children {
                        coast_q.get(*child).ok().map(|entity| {
                            cmds.entity(entity).despawn_recursive();
                        });
                    }
                }
            };
        }
    }
}

pub struct WorldPlugin<S: States> {
    state: S,
    loading_state: S,
}

impl<S: States + bevy::state::state::FreelyMutableState> Plugin for WorldPlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_loading_state(
            LoadingStateConfig::new(self.loading_state.clone()).load_collection::<WorldAssets>(),
        )
        .add_systems(
            OnTransition {
                exited: self.loading_state.clone(),
                entered: self.state.clone(),
            },
            (setup_tile_system, setup_water),
        )
        .init_resource::<GlobalAnimation>()
        .init_resource::<TileMap>()
        .init_resource::<LandMap>()
        .add_plugins(Material2dPlugin::<WaterMaterial>::default())
        // these nust happen in the PreUpdate, this is so the resource is up-to-date when the next
        // Update comes around. PostUpdate won't work as the GlobalTransform need's to be worked
        // out by then.
        .add_systems(
            PreUpdate,
            (
                update_register_tile,
                update_register_land,
                // these should happen after the land registers to avoid race conidtions
                update_remove_land.after(update_register_land),
                update_remove_tile.after(update_register_tile),
            ),
        )
        .add_systems(
            Update,
            (
                update_coastline,
                update_added_crumbs,
                update_crumbs_placed_cliff,
                update_meets_grass,
                update_added_land_atlas_index,
                update_changed_cliff_atlas_index,
                update_changed_platau_atlas_index,
                update_animated_tiles,
                update_tile_elevation,
            )
                .run_if(in_state(self.state.clone())),
        );
    }
}

impl<S: States> WorldPlugin<S> {
    pub fn run_on_state(state: S, loading_state: S) -> Self {
        Self {
            state,
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
    pub transform: Transform,
    pub visibility: Visibility,
}

impl Default for TileBundle {
    fn default() -> Self {
        TileBundle {
            tile: Tile::new(0, 0),
            elevation: Elevation::default(),
            visibility: Visibility::default(),
            transform: Transform::default(),
        }
    }
}

impl TileBundle {
    pub fn new(x: u16, y: u16, elevation: u8) -> Self {
        let z_offset = elevation as f32 + (WORLD_SIZE.y as f32 - y as f32);
        TileBundle {
            tile: Tile::new(x, y),
            elevation: Elevation(elevation),
            visibility: Visibility::Visible,
            transform: Transform::from_xyz(x as f32 * TILE_SIZE, y as f32 * TILE_SIZE, z_offset),
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
    mut animated_q: Query<(&mut Sprite, &mut GloballyAnimated)>,
    time: Res<Time>,
    mut global_animation: ResMut<GlobalAnimation>,
) {
    if global_animation.frame > usize::MAX {
        global_animation.frame = 0;
    }
    global_animation.timer.tick(time.delta());
    if global_animation.timer.finished() {
        global_animation.frame += 1;
        for (mut sprite, animated) in &mut animated_q {
            sprite.texture_atlas.as_mut().unwrap().index =
                global_animation.frame % animated.max_frames as usize;
        }
    }
}

#[derive(Resource, Default, Debug)]
pub struct LandMap {
    tiles: HashMap<(i16, i16, u8, Land), Entity>,
}

impl LandMap {
    fn count_neighbours(&self, x: i16, y: i16, elevation: u8, land: Land) -> u8 {
        self.tiles.contains_key(&(x + 1, y, elevation, land)) as u8
            + self.tiles.contains_key(&(x - 1, y, elevation, land)) as u8
            + self.tiles.contains_key(&(x, y + 1, elevation, land)) as u8
            + self.tiles.contains_key(&(x, y - 1, elevation, land)) as u8
    }
    fn get(&mut self, x: i16, y: i16, elevation: u8, land: Land) -> Option<&Entity> {
        self.tiles.get(&(x, y, elevation, land))
    }

    fn contains(&self, x: i16, y: i16, elevation: u8, land: Land) -> bool {
        self.tiles.contains_key(&(x, y, elevation, land))
    }

    fn remove_by_entity(&mut self, entity: Entity) -> Option<Entity> {
        for (pos, e) in self.tiles.clone() {
            if e == entity {
                return self.tiles.remove(&pos);
            }
        }
        return None;
    }
}

#[derive(Resource, Default, Debug)]
pub struct TileMap {
    tiles: HashMap<(u16, u16), (u8, Entity)>,
}

impl TileMap {
    pub fn count_neighbours(&self, x: i32, y: i32) -> u8 {
        self.contains(x + 1, y) as u8
            + self.contains(x - 1, y) as u8
            + self.contains(x, y + 1) as u8
            + self.contains(x, y - 1) as u8
    }

    pub fn count_neighbours_elevation_above(&self, x: i32, y: i32, elevation: u8) -> u8 {
        if x < 0 || y < 0 {
            return 0;
        }
        let x = x as u16;
        let y = y as u16;
        self.get_elevation(x + 1, y)
            .map(|e| *e > elevation)
            .unwrap_or(false) as u8
            + self
                .get_elevation(x - 1, y)
                .map(|e| *e > elevation)
                .unwrap_or(false) as u8
            + self
                .get_elevation(x, y + 1)
                .map(|e| *e > elevation)
                .unwrap_or(false) as u8
            + self
                .get_elevation(x, y - 1)
                .map(|e| *e > elevation)
                .unwrap_or(false) as u8
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        self.tiles.contains_key(&(x as u16, y as u16))
    }

    pub fn get_elevation(&self, x: u16, y: u16) -> Option<&u8> {
        self.tiles
            .get(&(x as u16, y as u16))
            .map(|(elevation, _)| elevation)
    }

    pub fn get_entity(&self, x: i32, y: i32) -> Option<&Entity> {
        if x < 0 || y < 0 {
            return None;
        }
        self.tiles
            .get(&(x as u16, y as u16))
            .map(|(_, entity)| entity)
    }

    fn remove_by_entity(&mut self, entity: Entity) -> Option<(u8, Entity)> {
        for (pos, (_, e)) in self.tiles.clone() {
            if e == entity {
                return self.tiles.remove(&pos);
            }
        }
        return None;
    }

    pub fn get(&self, x: i32, y: i32) -> Option<&(u8, Entity)> {
        if x < 0 || y < 0 {
            return None;
        }
        self.tiles.get(&(x as u16, y as u16))
    }

    pub fn insert(&mut self, x: u16, y: u16, elevation: u8, entity: Entity) {
        self.tiles.insert((x as u16, y as u16), (elevation, entity));
    }
}

pub fn update_register_tile(
    tiles_q: Query<(Entity, &Tile, &Elevation), Added<Tile>>,
    mut tile_map: ResMut<TileMap>,
) {
    for (entity, tile, elevation) in &tiles_q {
        tile_map
            .tiles
            .insert((tile.pos.x, tile.pos.y), (elevation.0, entity));
    }
}

fn update_register_land(
    tiles_q: Query<
        (Entity, &GlobalTransform, &Land, &Elevation),
        (Added<Land>, Without<DontRegisterLand>),
    >,
    mut tile_map: ResMut<LandMap>,
) {
    for (entity, transform, land, elevation) in &tiles_q {
        let tile_pos = (transform.translation().truncate() / TILE_VEC)
            .floor()
            .as_i16vec2();
        tile_map
            .tiles
            .insert((tile_pos.x, tile_pos.y, elevation.0, *land), entity);
    }
}

// todo(improvement): This is very slow operation!!! O(2n + nlogn) or something, need's fixing
fn update_remove_land(mut removed: RemovedComponents<Tile>, mut tile_map: ResMut<LandMap>) {
    for entity in removed.read() {
        tile_map.remove_by_entity(entity);
    }
}

// todo(improvement): This is very slow operation!!! O(2n + nlogn) or something, need's fixing
fn update_remove_tile(mut removed: RemovedComponents<Tile>, mut tile_map: ResMut<TileMap>) {
    for entity in removed.read() {
        tile_map.remove_by_entity(entity);
    }
}

fn update_tile_elevation(
    mut cmds: Commands,
    tiles_q: Query<(Entity, Ref<Elevation>), With<Tile>>,
    children_q: Query<&Children>,
    despawn_q: Query<Entity, With<DespawnOnElevationChange>>,
    assets: Res<WorldAssets>,
) {
    for (entity, elevation) in &tiles_q {
        if elevation.is_changed() || elevation.is_added() {
            if elevation.0 > 0 {
                if let Ok(children) = children_q.get(entity) {
                    children.iter().for_each(|child| {
                        if let Some(entity) = despawn_q.get(*child).ok() {
                            cmds.entity(entity).despawn_recursive();
                        }
                    });
                }
                let details = assets.cliff_detail(&mut cmds, elevation.0);
                cmds.entity(entity).add_children(&details);
            }
        }
    }
}

fn update_crumbs_placed_cliff(
    mut cmds: Commands,
    cliff_q: Query<(Entity, &GlobalTransform, &Elevation), (Added<Cliff>, Without<CliffLand>)>,
    assets: Res<WorldAssets>,
    land_map: Res<LandMap>,
    tile_map: Res<TileMap>,
) {
    for (entity, transform, Elevation(elevation)) in &cliff_q {
        let tile_pos = (transform.translation().truncate() / TILE_VEC)
            .floor()
            .as_i16vec2();
        for (x, y) in &[
            (tile_pos.x + 1, tile_pos.y),
            (tile_pos.x - 1, tile_pos.y),
            (tile_pos.x, tile_pos.y - 1),
            (tile_pos.x, tile_pos.y + 1),
        ] {
            let mut land = Option::<Land>::None;
            if land_map.contains(*x, *y, *elevation, Land::Sand) {
                land = Some(Land::Sand);
            }
            if land_map.contains(*x, *y, *elevation, Land::Grass) {
                land = Some(Land::Grass);
            }
            if tile_map
                .get_elevation(*x as u16, *y as u16)
                .is_some_and(|o_elevation| o_elevation > elevation)
            {
                land = None;
            }
            if let Some(Land::Grass) = land {
                cmds.entity(entity).despawn_descendants();
                let crumbs = cmds
                    .spawn((
                        assets.grass_crumbs(),
                        Transform::from_xyz(0., 0., 0.7),
                        Crumbs,
                        CliffLand,
                        Elevation(*elevation),
                    ))
                    .id();
                let grass = cmds
                    .spawn((
                        assets.grass(),
                        Transform::from_xyz(0., 0., -0.1),
                        Land::Grass,
                        CliffLand,
                        Elevation(*elevation),
                    ))
                    .id();
                cmds.entity(entity).add_children(&[grass, crumbs]);
            }
            if let Some(Land::Sand) = land {
                cmds.entity(entity).despawn_descendants();
                let crumbs = cmds
                    .spawn((
                        assets.sand_crumbs(),
                        Transform::from_xyz(0., 0., 0.6),
                        Crumbs,
                        CliffLand,
                        Elevation(*elevation),
                    ))
                    .id();
                let sand = cmds
                    .spawn((
                        assets.sand(),
                        Transform::from_xyz(0., 0., -0.1),
                        Land::Sand,
                        CliffLand,
                        Elevation(*elevation),
                    ))
                    .id();
                cmds.entity(entity).add_children(&[crumbs, sand]);
            }
        }
    }
}

fn update_added_crumbs(
    mut cmds: Commands,
    tiles_q: Query<
        (&GlobalTransform, &Elevation, &Land),
        (Added<Land>, (Without<CliffLand>, Without<DontRegisterLand>)),
    >,
    assets: Res<WorldAssets>,
    tile_map: Res<TileMap>,
    land_map: Res<LandMap>,
) {
    for (transform, Elevation(elevation), land) in &tiles_q {
        let tile_pos = (transform.translation().truncate() / TILE_VEC)
            .floor()
            .as_i16vec2();
        for (x, y) in &[
            (tile_pos.x + 1, tile_pos.y),
            (tile_pos.x - 1, tile_pos.y),
            (tile_pos.x, tile_pos.y - 1),
            (tile_pos.x, tile_pos.y + 1),
        ] {
            let mut neighbours = 0;
            for (ox, oy) in &[(*x + 1, *y), (*x - 1, *y), (*x, *y + 1), (*x, *y - 1)] {
                if let Some(neighbour_elevation) = tile_map.get_elevation(*ox as u16, *oy as u16) {
                    if *neighbour_elevation > *elevation {
                        continue;
                    }
                    if land_map.contains(*ox, *oy, *elevation, Land::Sand)
                        || land_map.contains(*ox, *oy, *elevation, Land::Grass)
                    {
                        neighbours += 1;
                    }
                }
            }
            if neighbours == 1 {
                if let Some((candidate_elevation, entity)) = tile_map.get(*x as i32, *y as i32) {
                    if candidate_elevation > elevation {
                        if let Land::Grass = land {
                            let crumbs = cmds
                                .spawn((
                                    assets.grass_crumbs(),
                                    Transform::from_xyz(0., 0., 0.7),
                                    Crumbs,
                                    CliffLand,
                                    DontRegisterLand,
                                    Elevation(*elevation),
                                ))
                                .id();
                            let grass = cmds
                                .spawn((
                                    assets.grass(),
                                    Transform::from_xyz(0., 0., -0.1),
                                    Land::Grass,
                                    CliffLand,
                                    Elevation(*elevation),
                                ))
                                .id();
                            cmds.entity(*entity).add_children(&[grass, crumbs]);
                        }
                        if let Land::Sand = land {
                            let crumbs = cmds
                                .spawn((
                                    assets.sand_crumbs(),
                                    Transform::from_xyz(0., 0., 0.6),
                                    Crumbs,
                                    CliffLand,
                                    DontRegisterLand,
                                    Elevation(*elevation),
                                ))
                                .id();
                            let sand = cmds
                                .spawn((
                                    assets.sand(),
                                    Transform::from_xyz(0., 0., -0.1),
                                    Land::Sand,
                                    CliffLand,
                                    Elevation(*elevation),
                                ))
                                .id();
                            cmds.entity(*entity).add_children(&[crumbs, sand]);
                        }
                    }
                }
            }
        }
    }
}

fn update_added_land_atlas_index(
    mut tiles_q: Query<(&mut Sprite, &GlobalTransform, &Elevation, Ref<Land>)>,
    assets: Res<WorldAssets>,
) {
    let mut tiles: HashMap<(i16, i16, Elevation, Land), bool> =
        HashMap::with_capacity(tiles_q.iter().len());
    for (_, transform, elevation, land) in &tiles_q {
        let tile_pos = (transform.translation().truncate() / TILE_VEC)
            .floor()
            .as_i16vec2();
        tiles.insert((tile_pos.x, tile_pos.y, *elevation, *land), land.is_added());
    }
    for (mut sprite, transform, elevation, land) in &mut tiles_q {
        if let Some(ref mut atlas) = &mut sprite.texture_atlas {
            let tile_pos = (transform.translation().truncate() / TILE_VEC)
                .floor()
                .as_i16vec2();
            if land.is_added() {
                let mut bitmask_total = 0;
                bitmask_total +=
                    tiles.contains_key(&(tile_pos.x, tile_pos.y + 1, *elevation, *land)) as u8
                        * 2_u8.pow(0);
                bitmask_total +=
                    tiles.contains_key(&(tile_pos.x - 1, tile_pos.y, *elevation, *land)) as u8
                        * 2_u8.pow(1);
                bitmask_total +=
                    tiles.contains_key(&(tile_pos.x + 1, tile_pos.y, *elevation, *land)) as u8
                        * 2_u8.pow(2);
                bitmask_total +=
                    tiles.contains_key(&(tile_pos.x, tile_pos.y - 1, *elevation, *land)) as u8
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
}

fn update_changed_cliff_atlas_index(
    mut tiles_q: Query<(&mut Sprite, &GlobalTransform, &Elevation, Ref<Cliff>)>,
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
    for (mut sprite, transform, elevation, cliff) in &mut tiles_q {
        if let Some(ref mut atlas) = &mut sprite.texture_atlas {
            let tile_pos = (transform.translation().truncate() / TILE_VEC)
                .floor()
                .as_i16vec2();
            if cliff.is_added() {
                let mut bitmask_total = 0;
                bitmask_total +=
                    tiles.contains_key(&(tile_pos.x - 1, tile_pos.y, *elevation, *cliff)) as u8
                        * 2_u8.pow(0);
                bitmask_total +=
                    tiles.contains_key(&(tile_pos.x + 1, tile_pos.y, *elevation, *cliff)) as u8
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
}

fn update_changed_platau_atlas_index(
    mut tiles_q: Query<(&mut Sprite, &GlobalTransform, &Elevation, Ref<Platau>)>,
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
    for (mut sprite, transform, elevation, platau) in &mut tiles_q {
        if let Some(ref mut atlas) = &mut sprite.texture_atlas {
            let tile_pos = (transform.translation().truncate() / TILE_VEC)
                .floor()
                .as_i16vec2();
            if platau.is_added() {
                let mut bitmask_total = 0;
                bitmask_total +=
                    tiles.contains_key(&(tile_pos.x, tile_pos.y + 1, *elevation, *platau)) as u8
                        * 2_u8.pow(0);
                bitmask_total +=
                    tiles.contains_key(&(tile_pos.x - 1, tile_pos.y, *elevation, *platau)) as u8
                        * 2_u8.pow(1);
                bitmask_total +=
                    tiles.contains_key(&(tile_pos.x + 1, tile_pos.y, *elevation, *platau)) as u8
                        * 2_u8.pow(2);
                bitmask_total +=
                    tiles.contains_key(&(tile_pos.x, tile_pos.y - 1, *elevation, *platau)) as u8
                        * 2_u8.pow(3);
                atlas.index = assets.platau_index_from_bitmask(bitmask_total as u8);
            } else {
                if let Some(true) = tiles.get(&(tile_pos.x, tile_pos.y - 1, *elevation, *platau)) {
                    let bitmask_up = assets.bitmask_from_platau_index(atlas.index) + BITMASK_BOT;
                    atlas.index = assets.platau_index_from_bitmask(bitmask_up);
                }
                if let Some(true) = tiles.get(&(tile_pos.x + 1, tile_pos.y, *elevation, *platau)) {
                    let bitmask_left =
                        assets.bitmask_from_platau_index(atlas.index) + BITMASK_RIGHT;
                    atlas.index = assets.platau_index_from_bitmask(bitmask_left);
                }
                if let Some(true) = tiles.get(&(tile_pos.x - 1, tile_pos.y, *elevation, *platau)) {
                    let bitmask_right =
                        assets.bitmask_from_platau_index(atlas.index) + BITMASK_LEFT;
                    atlas.index = assets.platau_index_from_bitmask(bitmask_right);
                }
                if let Some(true) = tiles.get(&(tile_pos.x, tile_pos.y + 1, *elevation, *platau)) {
                    let bitmask_down = assets.bitmask_from_platau_index(atlas.index) + BITMASK_TOP;
                    atlas.index = assets.platau_index_from_bitmask(bitmask_down);
                }
            }
        }
    }
}

fn setup_tile_system(mut cmds: Commands, assets: Res<WorldAssets>) {}
