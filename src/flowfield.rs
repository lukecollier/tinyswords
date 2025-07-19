use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::*;
use bevy::{
    color::palettes::css::{GREEN, WHITE},
    prelude::*,
};

const GRID_SIZE: usize = 32;
const CELL_SIZE: f32 = 64.;

// flowfield feels like a great method for "course" navigation
// I'm thinking of using the flowfield for general navigation then once nearing the target
// for attacking however it might be best to use a more accurate method
pub struct FlowFieldPlugin<S: States> {
    state: S,
}

impl<S: States> Plugin for FlowFieldPlugin<S> {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlowFields>().add_systems(
            Update,
            (
                update_flow_field_generation,
                debug_show_flow_field,
                update_clean_flow_field_cache,
            )
                .run_if(in_state(self.state.clone())),
        );
    }
}

impl<S: States> FlowFieldPlugin<S> {
    pub fn run_on_state(state: S) -> Self {
        Self { state }
    }
}

pub(crate) type DefaultSizeFlowField = FlowField<GRID_SIZE>;

#[derive(Debug, Resource, Default, Clone)]
pub struct FlowFields {
    fields: HashMap<UVec2, DefaultSizeFlowField>,
    impassable: HashSet<UVec2>,
}

impl FlowFields {
    // todo: Remove dependency on TerrainWorld, add accessor and handle in editor
    pub(crate) fn set_impassable(&mut self, point: UVec2) {
        self.impassable.insert(point);
    }

    pub(crate) fn set_passable(&mut self, point: &UVec2) {
        self.impassable.remove(point);
    }
    /// Creates a person with the given name.
    ///
    /// # Examples
    ///
    /// ```
    /// // You can have rust code between fences inside the comments
    /// // If you pass --test to `rustdoc`, it will even test it for you!
    /// ```
    pub(crate) fn is_walkable(&self, world_pos: &Vec2) -> bool {
        let grid_pos = DefaultSizeFlowField::world_to_grid(world_pos);
        !self.impassable.contains(&grid_pos)
    }

    fn get(&self, target: &UVec2) -> Option<DefaultSizeFlowField> {
        self.fields.get(target).cloned()
    }

    fn get_or_generate(&mut self, target: &UVec2) -> DefaultSizeFlowField {
        if let Some(field) = self.fields.get(target) {
            field.clone()
        } else {
            let field = DefaultSizeFlowField::build_flow_field(target, &self.impassable)
                .expect("Failed to build flowfield");
            self.fields.insert(*target, field.clone());
            field
        }
    }
}

// We will want to cache these flow fields,
// this makes their memory footprint somewhat important
// their calculation time is also very important
// todo: We might need to store the flowfield on the heap instead of the stack
// this should help avoid blowing the stack
#[derive(Debug, Clone)]
pub struct FlowField<const N: usize> {
    field: [[u8; N]; N],
}

impl<const N: usize> FlowField<N> {
    const UP: u8 = 0;
    const UP_RIGHT: u8 = 1;
    const RIGHT: u8 = 2;
    const DOWN_RIGHT: u8 = 3;
    const DOWN: u8 = 4;
    const DOWN_LEFT: u8 = 5;
    const LEFT: u8 = 6;
    const UP_LEFT: u8 = 7;
    const TARGET: u8 = 8;
    const IMPASSABLE: u8 = 9;
    fn get(&self, world_translation: Vec2) -> Vec2 {
        let grid_pos = DefaultSizeFlowField::world_to_grid(&world_translation);
        Self::u8_to_vector(&self.field[grid_pos.x as usize][grid_pos.y as usize])
            .expect("Failed to find vector from field")
    }

    pub(crate) fn world_to_grid(world_pos: &Vec2) -> UVec2 {
        world_pos.as_uvec2() / UVec2::splat(CELL_SIZE as u32)
    }

    fn set_grid(grid: &mut [[u8; N]; N], pos: IVec2, value: u8) {
        grid[pos.x as usize][pos.y as usize] = value;
    }

    // 0 is up,
    // 1 is up_right,
    // 2 is right,
    // 3 is down_right,
    // 4 is down,
    // 5 is down_left,
    // 6 is left,
    // 7 is up_left,
    // 8 is target,
    fn u8_to_vector(value: &u8) -> anyhow::Result<Vec2> {
        match value {
            0 => Ok(Vec2::new(0., 1.)),
            1 => Ok(Vec2::new(1., 1.)),
            2 => Ok(Vec2::new(1., 0.)),
            3 => Ok(Vec2::new(1., -1.)),
            4 => Ok(Vec2::new(0., -1.)),
            5 => Ok(Vec2::new(-1., -1.)),
            6 => Ok(Vec2::new(-1., 0.)),
            7 => Ok(Vec2::new(-1., 1.)),
            8 => Ok(Vec2::new(0., 0.)),
            &u8::MAX => Ok(Vec2::new(0., 0.)),
            &Self::IMPASSABLE => Ok(Vec2::new(0., 0.)),
            _ => Err(anyhow!("direction not recognised")),
        }
    }

    fn vector_to_u8(vec: IVec2) -> anyhow::Result<u8> {
        match (vec.x, vec.y) {
            (0, 1) => Ok(0),
            (1, 1) => Ok(1),
            (1, 0) => Ok(2),
            (1, -1) => Ok(3),
            (0, -1) => Ok(4),
            (-1, -1) => Ok(5),
            (-1, 0) => Ok(6),
            (-1, 1) => Ok(7),
            (0, 0) => Ok(8),
            _ => Err(anyhow!("direction not recognised")),
        }
    }

    fn build_flow_field(
        target: &UVec2,
        impassable: &HashSet<UVec2>,
    ) -> anyhow::Result<FlowField<N>> {
        if impassable.contains(target) {
            return Err(anyhow!("target is in impassable area"));
            // todo: Find the nearest grid? Or just error?
            // Maybe the logic for finding the closest walkable area is better suited in game
        }
        let target = target.as_ivec2();
        let mut field = [[0_u8; N]; N];
        let mut costs = [[0_u8; N]; N];
        let grid_area = IRect::new(0, 0, N as i32 - 1, N as i32 - 1);
        if !grid_area.contains(target) {
            return Err(anyhow!("out of bounds error"));
        }
        let mut queue: VecDeque<IVec2> = VecDeque::new();
        queue.push_back(target);
        let mut seen = HashSet::new();
        seen.insert(target);
        for blocked in impassable {
            let as_ivec = blocked.as_ivec2();
            seen.insert(as_ivec);
            Self::set_grid(&mut costs, as_ivec, u8::MAX);
        }
        // todo: If we're surrounded by blockers we should prioritise horizontal / vertical
        // movement. The only way we end up walking into the sea currently is by pesky diagonals
        // pushing us into the water
        while let Some(root) = queue.pop_front() {
            let IVec2 { x, y } = root;
            let cost = costs[x as usize][y as usize];
            let up = root + IVec2::Y;
            let up_right = root + IVec2::ONE;
            let right = root + IVec2::X;
            let down_right = root + IVec2::X - IVec2::Y;
            let down = root - IVec2::Y;
            let down_left = root - IVec2::ONE;
            let left = root - IVec2::X;
            let up_left = root + IVec2::Y - IVec2::X;
            let orthogonal = [up, right, down, left];
            let diagonals = [up_right, down_right, down_left, up_left];
            for pos in orthogonal {
                if seen.insert(pos) && grid_area.contains(pos) {
                    queue.push_back(pos);
                    Self::set_grid(&mut costs, pos, cost + 1);
                }
            }
            for pos in diagonals {
                if seen.insert(pos) && grid_area.contains(pos) {
                    queue.push_back(pos);
                    Self::set_grid(&mut costs, pos, cost + 2);
                }
            }
            let (mut dir, mut min_cost): (IVec2, u8) = (IVec2::MAX, u8::MAX);
            for pos in diagonals {
                if grid_area.contains(pos) {
                    let ncost = costs[pos.x as usize][pos.y as usize];
                    if min_cost > ncost {
                        let direction = pos - root;
                        dir = direction;
                        min_cost = ncost;
                    }
                }
            }
            for pos in orthogonal {
                if grid_area.contains(pos) {
                    let ncost = costs[pos.x as usize][pos.y as usize];
                    if min_cost > ncost {
                        let direction = pos - root;
                        dir = direction;
                        min_cost = ncost;
                    }
                }
            }
            if dir == IVec2::MAX || min_cost == u8::MAX {
                return Err(anyhow!(
                    "we couldn't find a direction for the flow field {}",
                    root
                ));
            }
            Self::set_grid(&mut field, root, Self::vector_to_u8(dir)?);
            // now we can calculate our vector directions for the current coords since we know all
            // the surrounding costs
        }
        for blocked in impassable {
            let as_ivec = blocked.as_ivec2();
            Self::set_grid(&mut field, as_ivec, u8::MAX);
        }
        Self::set_grid(&mut field, target, Self::IMPASSABLE);
        Ok(FlowField { field })
    }
}

#[derive(Component)]
pub struct FlowFieldActor {
    // the actors current target
    pub(crate) target: Vec2,
    // the direction to follow to get to the target
    pub(crate) steering: Vec2,
}

impl FlowFieldActor {
    pub(crate) fn new(target: Vec2) -> Self {
        Self {
            target,
            steering: Vec2::ZERO,
        }
    }
}

#[derive(Component)]
pub struct FlowFieldDebugging;

// todo: We should handle the transform changing and update the flow field
// todo: We need to remove flowfields from the cache when their no longer in use
// todo: We need to check if we've entered a new grid section before running this
fn update_flow_field_generation(
    mut actor_q: Query<(&mut FlowFieldActor, &Transform)>,
    mut flow_fields: ResMut<FlowFields>,
) {
    for (mut actor, transform) in actor_q.iter_mut() {
        let target_pos = DefaultSizeFlowField::world_to_grid(&actor.target);
        let flow_field = flow_fields.get_or_generate(&target_pos);
        let steering = flow_field.get(transform.translation.truncate());
        //todo: Here we accidentally make walking through walls possible, if we reach a wall we
        // just move straight through it. A good way to fix this is to ensure we never walk into
        // walls through the flowfield.
        if steering == Vec2::ZERO {
            actor.steering = (actor.target - transform.translation.truncate()).normalize_or_zero();
        } else {
            actor.steering = steering;
        }
    }
}

// we clear the cache when no actor is currently using a flowfield in the cache
fn update_clean_flow_field_cache(
    actor_q: Query<&FlowFieldActor>,
    mut flow_fields: ResMut<FlowFields>,
) {
    let in_use = actor_q
        .iter()
        .map(|actor| DefaultSizeFlowField::world_to_grid(&actor.target))
        .collect::<HashSet<UVec2>>();
    let keys_in_use = flow_fields
        .fields
        .keys()
        // this is probs expensive
        .copied()
        .collect::<HashSet<UVec2>>();
    for key in keys_in_use.difference(&in_use) {
        info!("cleaned {}", &key);
        flow_fields.fields.remove(key);
    }
}

fn debug_show_flow_field(
    target_q: Query<&FlowFieldActor, With<FlowFieldDebugging>>,
    flow_fields: Res<FlowFields>,
    mut gizmos: Gizmos,
) {
    for actor in &target_q {
        let target_pos = actor.target.as_uvec2() / UVec2::splat(CELL_SIZE as u32);
        if let Some(flow_field) = flow_fields.get(&target_pos) {
            for (x, col) in flow_field.field.iter().enumerate() {
                for (y, cell) in col.iter().enumerate() {
                    let half_grid_size = Vec2::splat(CELL_SIZE / 2.);
                    let start =
                        Vec2::new(x as f32 * CELL_SIZE, y as f32 * CELL_SIZE) + half_grid_size;
                    let end = Vec2::new(x as f32 * CELL_SIZE, y as f32 * CELL_SIZE)
                        + half_grid_size
                        + (DefaultSizeFlowField::u8_to_vector(cell).unwrap() * half_grid_size);
                    gizmos.arrow_2d(start, end, GREEN);
                    gizmos.rect_2d(
                        Isometry2d::new(start, Rot2::IDENTITY),
                        Vec2::splat(CELL_SIZE),
                        WHITE,
                    );
                }
            }
        }
    }
}
