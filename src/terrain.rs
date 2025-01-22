use bevy::{
    math::Vec2,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Anchor, Material2d, Material2dPlugin},
    utils::HashMap,
};
use bevy_asset_loader::prelude::*;

pub const WORLD_SIZE: usize = 32;
pub const TILE_SIZE_F32: f32 = 64.0;
pub const TILE_EDGE_BUFFER: f32 = TILE_SIZE_F32;
pub const TILE_SIZE_U32: u32 = 64;
pub const TILE_SIZE_I32: i32 = 64;
pub const TILE_SIZE_VEC2: Vec2 = Vec2::new(TILE_SIZE_F32, TILE_SIZE_F32);
pub const TILE_SIZE_UVEC2: UVec2 = UVec2::new(TILE_SIZE_U32, TILE_SIZE_U32);

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

pub struct TerrainPlugin<S: States> {
    state: S,
    loading_state: S,
}

impl<S: States + bevy::state::state::FreelyMutableState> Plugin for TerrainPlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_loading_state(
            LoadingStateConfig::new(self.loading_state.clone()).load_collection::<TerrainAssets>(),
        )
        .add_plugins(Material2dPlugin::<WaterMaterial>::default())
        .insert_resource(TerrainWorld::<WORLD_SIZE>::empty())
        .add_systems(OnEnter(self.state.clone()), on_enter_water)
        .add_systems(OnExit(self.state.clone()), on_exit_water)
        .add_systems(
            Update,
            (update_load_world_to_ecs, update_ecs_when_world_changes)
                .run_if(in_state(self.state.clone())),
        );
    }
}

impl<S: States> TerrainPlugin<S> {
    pub fn run_on_state(state: S, loading_state: S) -> Self {
        Self {
            state,
            loading_state,
        }
    }
}

// todo: Is this a better way for us to interact across systems? Our
#[derive(Resource)]
pub struct TerrainModifyOptions {
    placing: Terrain,
}

pub type TerrainWorldDefault = TerrainWorld<WORLD_SIZE>;

// So the reason we duplicate the data in vert and horizontal is so we can
// quickly access the the neighbours for and find the right tile combinations when we load the map
// in
#[derive(Resource)]
pub struct TerrainWorld<const N: usize> {
    map: [[u8; N]; N],
}

impl Default for TerrainWorld<WORLD_SIZE> {
    fn default() -> Self {
        TerrainWorld::empty()
    }
}

impl<const N: usize> TerrainWorld<N> {
    pub const WATER: u8 = 0;
    pub const SAND: u8 = 16;
    pub const GRASS: u8 = 32;

    fn empty() -> TerrainWorld<N> {
        TerrainWorld {
            map: [[Self::WATER; N]; N],
        }
    }

    pub fn coords_to_world(&self, coords: &Vec2) -> Option<UVec2> {
        let world_coord = coords / TILE_SIZE_F32;
        if world_coord.x >= 0.
            && world_coord.y >= 0.
            && (world_coord.x.floor() as usize) < WORLD_SIZE * N
            && (world_coord.y.floor() as usize) < WORLD_SIZE * N
        {
            Some(world_coord.floor().as_uvec2())
        } else {
            None
        }
    }

    pub(crate) fn set_to_sand(&mut self, pos: &UVec2) -> Result<(), ()> {
        let x = pos.x as usize;
        let y = pos.y as usize;
        if Self::outside_bounds(x, y) {
            return Err(());
        }
        self.map[x][y] = Self::SAND;
        Ok(())
    }

    pub(crate) fn set_to_grass(&mut self, pos: &UVec2) -> Result<(), ()> {
        let x = pos.x as usize;
        let y = pos.y as usize;
        if Self::outside_bounds(x, y) {
            return Err(());
        }
        self.map[x][y] = Self::GRASS;
        Ok(())
    }

    // let's us check neighbours for same values without having to unpack the bytes.
    // We only want to unpack bytes when loading them into our ecs world
    fn is_water(byte: &u8) -> bool {
        byte >= &Self::WATER && byte <= &(Self::WATER + 15)
    }

    fn is_sand(byte: &u8) -> bool {
        byte >= &Self::SAND && byte <= &(Self::SAND + 15)
    }

    fn is_grass(byte: &u8) -> bool {
        byte >= &Self::GRASS && byte <= &(Self::GRASS + 15)
    }

    fn is_same_type(first_byte: &u8, second_byte: &u8) -> bool {
        (Self::is_water(first_byte) && Self::is_water(second_byte))
            || (Self::is_sand(first_byte) && Self::is_sand(second_byte))
            || (Self::is_grass(first_byte) && Self::is_grass(second_byte))
            // sand should connect to grass.
            || (Self::is_sand(first_byte) && Self::is_grass(second_byte))
    }

    fn in_bounds(x: usize, y: usize) -> bool {
        x < N && y < N
    }

    fn outside_bounds(x: usize, y: usize) -> bool {
        !Self::in_bounds(x, y)
    }

    // todo: How do we handle the edges of the map?
    // todo: Can we just reference slices from our map?
    fn get_neighbours(&self, pos: &UVec2) -> [Option<&u8>; 4] {
        let x = pos.x as usize;
        let y = pos.y as usize;
        let top = if Self::in_bounds(x, y + 1) {
            Some(&self.map[x][y + 1])
        } else {
            None
        };
        let bot = if y != 0 && Self::in_bounds(x, y - 1) {
            Some(&self.map[x][y - 1])
        } else {
            None
        };
        let left = if x != 0 && Self::in_bounds(x - 1, y) {
            Some(&self.map[x - 1][y])
        } else {
            None
        };
        let right = if Self::in_bounds(x + 1, y) {
            Some(&self.map[x + 1][y])
        } else {
            None
        };
        [top, left, right, bot]
    }

    fn get_bitmask_sand(&self, pos: &UVec2) -> u8 {
        let x = pos.x as usize;
        let y = pos.y as usize;
        if Self::outside_bounds(x, y) {
            return 0;
        }
        let mut bitmask: u8 = 0;
        for (idx, neighbour) in self.get_neighbours(pos).iter().enumerate() {
            if let Some(neighbour) = neighbour {
                if Self::is_sand(neighbour) {
                    bitmask += 2_u8.pow(idx as u32);
                }
            }
        }
        bitmask
    }

    fn get_bitmask(&self, pos: &UVec2) -> u8 {
        let x = pos.x as usize;
        let y = pos.y as usize;
        if Self::outside_bounds(x, y) {
            return 0;
        }
        let terrain_type = self.map[x][y];
        let mut bitmask: u8 = 0;
        for (idx, neighbour) in self.get_neighbours(pos).iter().enumerate() {
            if let Some(neighbour) = neighbour {
                if Self::is_same_type(&terrain_type, neighbour) {
                    bitmask += 2_u8.pow(idx as u32);
                }
            }
        }
        bitmask
    }

    fn get_tile_from(&self, pos: &UVec2) -> Option<TerrainTile> {
        let (x, y) = (pos.x as usize, pos.y as usize);
        if (pos.x as usize) >= N || (pos.y as usize) >= N {
            return None;
        } else {
            let byte = self.map[x][y];
            return TerrainTile::from_byte(byte).ok();
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum Terrain {
    Sand,
    Grass,
    Water,
}

#[derive(Component, Debug, PartialEq)]
#[require(Transform)]
pub(crate) struct TerrainTile {
    terrain: Terrain,
    height: u8,
}

impl TerrainTile {
    fn from_byte(byte: u8) -> Result<Self, String> {
        Ok(byte.try_into()?)
    }
}

impl TryFrom<u8> for TerrainTile {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        fn u8_to_bool_array(value: u8) -> [bool; 8] {
            let mut result = [false; 8];

            for i in 0..8 {
                result[7 - i] = (value >> i) & 1 != 0;
            }

            result
        }
        fn nibble_to_u8(arr: &[bool]) -> u8 {
            let mut result = 0u8;

            for (i, &bit) in arr.iter().enumerate() {
                if bit {
                    result |= 1 << (3 - i); // Set the bit at position `7 - i`
                }
            }

            result
        }
        let binary = u8_to_bool_array(value);
        let terrain_type = &binary[0..4];
        let terrain_height = &binary[4..8];
        let terrain_type = match nibble_to_u8(terrain_type) {
            0 => Terrain::Water,
            1 => Terrain::Sand,
            2 => Terrain::Grass,
            num => return Err(format!("Unknown terrain type with id: [{}]", num)),
        };
        let terrain_height = nibble_to_u8(terrain_height);
        Ok(TerrainTile {
            terrain: terrain_type,
            height: terrain_height,
        })
    }
}

struct TerrainView;
impl TerrainView {
    pub(crate) fn resolve_positions(
        xy: Vec2,
        view: Rect,
        logical_tiles_positions: Vec<Vec2>,
    ) -> Vec<UVec2> {
        let new_view = view.inflate(TILE_EDGE_BUFFER);
        let tiles = new_view.size() / TILE_SIZE_F32;
        let new_start_at = (xy / TILE_SIZE_VEC2 - (new_view.half_size() / TILE_SIZE_F32))
            .max(Vec2::ZERO)
            .as_uvec2();
        let mut added_positions = Vec::with_capacity((tiles.x * tiles.y) as usize);
        for x in 0..=tiles.x.ceil() as u32 {
            for y in 0..=tiles.y.ceil() as u32 {
                let pos = new_start_at + UVec2::new(x, y);
                if !logical_tiles_positions.contains(&(pos * TILE_SIZE_U32).as_vec2()) {
                    added_positions.push(pos);
                }
            }
        }
        added_positions
    }
}

// alternatively could communicate using events, the problem being we need to check every loaded
// ecs component for changes against the tile_map.
fn update_ecs_when_world_changes(
    mut commands: Commands,
    terrain: Res<TerrainWorldDefault>,
    assets: Res<TerrainAssets>,
    mut tile_q: Query<(Entity, &mut TerrainTile, &mut Sprite, &Transform)>,
) {
    if terrain.is_changed() {
        // todo: Handle water as a special case, we don't store water in our ecs so we need to do
        // something special
        // todo: If our tile is currently a water tile we won't change it, we need to spawn a new
        // tile :think:
        for (entity, mut terrain_tile, mut sprite, transform) in tile_q.iter_mut() {
            let Some(pos) = terrain.coords_to_world(&transform.translation.truncate()) else {
                continue;
            };
            if let Some(candidate_tile) = terrain.get_tile_from(&pos) {
                if *terrain_tile != candidate_tile {
                    terrain_tile.terrain = candidate_tile.terrain;
                    terrain_tile.height = candidate_tile.height;
                    let image = assets.tile_to_image(&terrain_tile);
                    sprite.image = image;
                    if terrain_tile.terrain == Terrain::Grass && terrain.get_bitmask_sand(&pos) > 0
                    {
                        let sand_bitmask = terrain.get_bitmask_sand(&pos);
                        let index = TerrainAssets::index_from_bitmask(sand_bitmask);
                        let texture_atlas = TextureAtlas {
                            layout: assets.land_layout.clone(),
                            index,
                        };
                        // todo: We need to spawn a sand texture as a child under our grass if we're at
                        // a border of grass and sand. The texture should just be the middle piece of
                        // sand.
                        let mut sprite =
                            Sprite::from_atlas_image(assets.sand_texture.clone(), texture_atlas);
                        sprite.anchor = Anchor::BottomLeft;
                        commands.entity(entity).with_children(|parent| {
                            parent.spawn((sprite, Transform::from_xyz(0., 0., -1.)));
                        });
                    }
                }
            }
            if let Some(ref mut texture_atlas) = sprite.texture_atlas {
                let bitmask = terrain.get_bitmask(&pos);
                let index = TerrainAssets::index_from_bitmask(bitmask);
                texture_atlas.index = index;
            }
        }
    }
}

// todo: If grass spawns next to sand it should spawn a sand image underneath it as well.
fn update_load_world_to_ecs(
    mut commands: Commands,
    terrain: ResMut<TerrainWorldDefault>,
    assets: Res<TerrainAssets>,
    camera_q: Single<(&Camera, &GlobalTransform, &OrthographicProjection), Changed<Transform>>,
    tile_q: Query<(Entity, &Transform), With<TerrainTile>>,
) {
    let (camera, camera_transform, projection) = camera_q.into_inner();
    dbg!(projection.area, camera_transform);
    if let Some(rect) = camera.logical_viewport_rect() {
        let rect = Rect::from_corners(rect.min * projection.scale, rect.max * projection.scale);
        if rect.min.x >= 0. && rect.min.y >= 0. && rect.max.x >= 0. && rect.max.y >= 0. {
            let urect = rect.as_urect();
            let camera_viewport = urect.clone();
            let camera_xy = camera_transform.translation().xy().clone();
            let tiles: Vec<Vec2> = tile_q
                .iter()
                .map(|(_, transform)| transform.translation.xy())
                .collect();
            let added = TerrainView::resolve_positions(camera_xy, rect, tiles);
            for pos in &added {
                if let Some(tile) = terrain.get_tile_from(pos) {
                    let tile_terrain = tile.terrain.clone();
                    let z = match tile_terrain {
                        Terrain::Water => 0.,
                        Terrain::Sand => 1.,
                        Terrain::Grass => 2.,
                    };

                    let bitmask = terrain.get_bitmask(pos);
                    let index = TerrainAssets::index_from_bitmask(bitmask);
                    let texture_atlas = TextureAtlas {
                        layout: assets.land_layout.clone(),
                        index,
                    };
                    let mut sprite =
                        Sprite::from_atlas_image(assets.tile_to_image(&tile), texture_atlas);
                    sprite.anchor = Anchor::BottomLeft;
                    let pos_transform =
                        Transform::from_translation((pos * TILE_SIZE_U32).as_vec2().extend(z));
                    let mut spawned = commands.spawn((sprite, pos_transform, tile));
                    if tile_terrain == Terrain::Grass && terrain.get_bitmask_sand(&pos) > 0 {
                        let sand_bitmask = terrain.get_bitmask_sand(&pos);
                        let index = TerrainAssets::index_from_bitmask(sand_bitmask);
                        let texture_atlas = TextureAtlas {
                            layout: assets.land_layout.clone(),
                            index,
                        };
                        // todo: We need to spawn a sand texture as a child under our grass if we're at
                        // a border of grass and sand. The texture should just be the middle piece of
                        // sand.
                        let mut sprite =
                            Sprite::from_atlas_image(assets.sand_texture.clone(), texture_atlas);
                        sprite.anchor = Anchor::BottomLeft;
                        spawned.with_children(|parent| {
                            parent.spawn((sprite, Transform::from_xyz(0., 0., -1.)));
                        });
                    }
                }
            }
            // if we added tiles we probably need to remove some tiles.
            if !added.is_empty() {
                let tile_size = TILE_EDGE_BUFFER;
                let start_at = camera_xy - camera_viewport.half_size().as_vec2() - (tile_size / 2.);
                let current_view =
                    Rect::from_corners(start_at, start_at + rect.size()).inflate(tile_size);
                for (entity, transform) in &tile_q {
                    let tile_rect = Rect::from_corners(
                        transform.translation.xy(),
                        transform.translation.xy() + TILE_SIZE_F32,
                    );
                    if current_view.intersect(tile_rect).is_empty() {
                        commands.entity(entity).despawn_recursive();
                    }
                }
                // for now we only clean up old entities when added has changed
            }
            //todo: we shouldn't update our terrain view until we find deltas
            // once we've calculated deltas etc etc, we replace our TerrainView so we can calculate our
            // deltas (what need's spawning in /despawning)
        }
    }
}

#[derive(Component)]
struct Water;

fn on_enter_water(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WaterMaterial>>,
) {
    let size = TILE_SIZE_F32 * WORLD_SIZE as f32;
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(size, size)).into()),
        MeshMaterial2d(materials.add(WaterMaterial {
            color: Color::srgb(7.5, 0.0, 7.5),
        })),
        Transform::from_xyz(size / 2., size / 2., -100.),
        Water,
    ));
}

fn on_exit_water(mut commands: Commands, water_q: Single<Entity, With<Water>>) {
    let size = TILE_SIZE_F32 * WORLD_SIZE as f32;
    let entity = water_q.into_inner();
    commands.entity(entity).despawn_recursive();
}

#[derive(AssetCollection, Resource)]
pub struct TerrainAssets {
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

impl TerrainAssets {
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

    fn index_from_bitmask(bitmask: u8) -> usize {
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

    fn tile_to_image(&self, tile: &TerrainTile) -> Handle<Image> {
        match tile.terrain {
            Terrain::Sand => self.sand_texture.clone(),
            Terrain::Grass => self.grass_texture.clone(),
            Terrain::Water => self.water_texture.clone(),
        }
    }
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
