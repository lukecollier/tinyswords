use bevy::{
    math::{I16Vec2, I64Vec2},
    prelude::*,
    utils::{
        hashbrown::HashSet,
        petgraph::{algo::astar, prelude::*},
    },
};

use crate::{
    camera::MainCamera,
    world::{TILE_SIZE, WORLD_SIZE},
};

pub struct NavPlugin<S: States> {
    state: S,
}

#[derive(Component)]
struct Navigator {
    navigate_to: Entity,
}

impl Navigator {
    pub fn navigate_to(navigate_to: Entity) -> Navigator {
        Navigator { navigate_to }
    }
}

// todo: We can make it even easier, by taking the corners of all blockd areas and connecting them
// when no connecting intersection is found
// can also aggregate rects that are connected to each other
//
fn nav_graph_2d(blocked: Vec<Rect>, bounds: Rect) -> UnGraph<I16Vec2, f32> {
    // todo: Should be as big as the entity that's moving
    let resolution = 32 as i64;
    let nodes = ((bounds.max.x - bounds.min.x) / resolution as f32) as usize
        * ((bounds.max.y - bounds.min.y) / resolution as f32) as usize;
    let mut graph: UnGraph<I16Vec2, f32> = Graph::with_capacity(nodes + 2, nodes + 2);

    for x in ((bounds.min.x as i16)..(bounds.max.x as i16)).step_by(resolution as usize) {
        for y in ((bounds.min.y as i16)..(bounds.max.y as i16)).step_by(resolution as usize) {
            let point = I16Vec2::new(x, y);
            let mut found = false;
            for block in &blocked {
                if block.contains(point.as_vec2()) {
                    found = true;
                    break;
                }
            }
            if !found {
                graph.add_node(point);
            }
        }
    }
    let points: HashSet<_> = graph
        .node_indices()
        .map(|node| {
            let point = graph[node];
            (point.x as i64, point.y as i64, node)
        })
        .collect();

    // connecting the nodes is definitely fooked
    for (x, y, node) in points.iter() {
        for (ox, oy, onode) in points.iter() {
            if x == ox && y == oy {
                continue;
            }
            if (x - ox).abs() <= resolution && (y - oy).abs() <= resolution {
                if x == ox || y == oy {
                    graph.add_edge(*node, *onode, 1.);
                    continue;
                }
                // if we're a diagonal should be weight 2 or something
                graph.add_edge(*node, *onode, (2.0 as f32).sqrt());
            }
        }
    }
    graph
}

// 1. Fill 2d points from the start to the end, we can check the X and Y connecting points
// if they're blocked we stop moving in that direction
// 2. Cull 2d points inside a blocked are
// 3. Remove points that are not connected to the initial point
// 4. If there is no path, return an empty vector
// 5. Perform a pathfinding algorithm to find the shortest path between start and end
pub fn path_between_2d(blocked: Vec<Rect>, bounds: Rect, start: Vec2, end: Vec2) -> Vec<Vec2> {
    // todo: Should be as big as the entity that's moving
    let resolution = 32 as i64;
    let nodes = ((bounds.max.x - bounds.min.x) / resolution as f32) as usize
        * ((bounds.max.y - bounds.min.y) / resolution as f32) as usize;
    let mut graph: UnGraph<I64Vec2, f32> = Graph::with_capacity(nodes + 2, nodes + 2);
    let begin_at = graph.add_node(start.as_i64vec2());
    let fin = graph.add_node(end.as_i64vec2());

    let mut closest_to_start = I64Vec2::ZERO;
    let mut closest_to_end = I64Vec2::ZERO;

    for x in ((bounds.min.x as i64)..(bounds.max.x as i64)).step_by(resolution as usize) {
        for y in ((bounds.min.y as i64)..(bounds.max.y as i64)).step_by(resolution as usize) {
            let point = I64Vec2::new(x, y);
            if start.distance(point.as_vec2()) < start.distance(closest_to_start.as_vec2()) {
                closest_to_start = point;
            }
            if end.distance(point.as_vec2()) < end.distance(closest_to_end.as_vec2()) {
                closest_to_end = point;
            }
            let mut found = false;
            for block in &blocked {
                if block.contains(point.as_vec2()) {
                    found = true;
                    break;
                }
            }
            if !found {
                graph.add_node(point);
            }
        }
    }
    let points: HashSet<_> = graph
        .node_indices()
        .map(|node| {
            let point = graph[node];
            (point.x as i64, point.y as i64, node)
        })
        .collect();

    // connecting the nodes is definitely fooked
    for (x, y, node) in points.iter() {
        for (ox, oy, onode) in points.iter() {
            if x == ox && y == oy {
                continue;
            }
            if (x - ox).abs() <= resolution && (y - oy).abs() <= resolution {
                if x == ox || y == oy {
                    graph.add_edge(*node, *onode, 1.);
                    continue;
                }
                // if we're a diagonal should be weight 2 or something
                graph.add_edge(*node, *onode, (2.0 as f32).sqrt());
            }
        }
    }

    if let Some((_, path)) = astar(
        &graph,
        begin_at,
        |finish| finish == fin,
        |e| *e.weight(),
        |_| 0.0,
    ) {
        path.iter().map(|node| graph[*node].as_vec2()).collect()
    } else {
        vec![]
    }
}

// todo: For a better solution, use a nav mesh, find the points, then offset the points by the
// normal of the two connecting lines to avoid a wonky looking path

// need's a GlobalTransform, blocks navigation through this entity
#[derive(Component)]
struct NavSquare {
    size: Vec2,
    walkable: bool,
}

#[derive(Bundle)]
struct NavBundle {
    transform: TransformBundle,
    blocker: NavSquare,
}

impl NavBundle {
    pub fn blocked(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::from_xy(Vec2::new(x, y), Vec2::new(width, height), true)
    }

    pub fn allowed(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::from_xy(Vec2::new(x, y), Vec2::new(width, height), false)
    }
    pub fn from_xy(xy: Vec2, size: Vec2, walkable: bool) -> Self {
        Self {
            transform: TransformBundle::from_transform(Transform::from_translation(xy.extend(0.0))),
            blocker: NavSquare { size, walkable },
        }
    }
}

fn setup_nav(
    mut cmds: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
}

fn update_nav(navigator_q: Query<(Entity, &Navigator)>, pos_q: Query<&GlobalTransform>) {}

fn debug_blockers(mut gizmos: Gizmos, blocker_q: Query<(&GlobalTransform, &NavSquare)>) {
    for (pos, blocker) in blocker_q.iter() {
        if blocker.walkable {
            gizmos.rect_2d(pos.translation().xy(), 0., blocker.size, Color::GREEN)
        } else {
            gizmos.rect_2d(pos.translation().xy(), 0., blocker.size, Color::RED)
        }
    }
}

fn update_add_blockers(
    mut cmds: Commands,
    window_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
) {
    if let Ok(window) = window_q.get_single() {
        for (camera, camera_transform) in camera_q.iter() {
            if let Some(cursor_pos) = window.cursor_position() {
                if let Some(world_cursor_pos) =
                    camera.viewport_to_world_2d(camera_transform, cursor_pos)
                {
                    if mouse_button.just_pressed(MouseButton::Right) {
                        cmds.spawn(NavBundle::allowed(
                            (world_cursor_pos.x - world_cursor_pos.x % 64.) + 32.,
                            (world_cursor_pos.y - world_cursor_pos.y % 64.) + 32.,
                            64.,
                            64.,
                        ));
                    }
                    if mouse_button.just_pressed(MouseButton::Left) {
                        cmds.spawn(NavBundle::blocked(
                            (world_cursor_pos.x - world_cursor_pos.x % 64.) + 32.,
                            (world_cursor_pos.y - world_cursor_pos.y % 64.) + 32.,
                            64.,
                            64.,
                        ));
                    }
                }
            }
        }
    }
}

fn debug_nav(
    mut gizmos: Gizmos,
    navigator_q: Query<(&GlobalTransform, &Navigator)>,
    pos_q: Query<&GlobalTransform>,
    blocker_q: Query<(&GlobalTransform, &NavSquare)>,
) {
    let blocked: Vec<_> = blocker_q
        .iter()
        .map(|(pos, blocker)| {
            let half_x = blocker.size.x / 2.;
            let half_y = blocker.size.y / 2.;
            Rect::new(
                pos.translation().x - half_x,
                pos.translation().y - half_y,
                pos.translation().x + half_x,
                pos.translation().y + half_y,
            )
        })
        .collect();
    let map_bounds = Rect::new(
        0.,
        0.,
        TILE_SIZE * WORLD_SIZE.x as f32,
        TILE_SIZE * WORLD_SIZE.y as f32,
    );
    for (position, navigator) in navigator_q.iter() {
        let end = pos_q.get(navigator.navigate_to).unwrap();
        let path = path_between_2d(
            blocked.clone(),
            map_bounds,
            position.translation().truncate(),
            end.translation().truncate(),
        );
        for point in path.windows(2) {
            gizmos.arrow_2d(point[0], point[1], Color::BLUE);
        }
    }
}

impl<S: States> Plugin for NavPlugin<S> {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(self.state.clone()), (setup_nav))
            .add_systems(
                Update,
                (update_nav, debug_nav, debug_blockers).run_if(in_state(self.state.clone())),
            );
    }
}

impl<S: States> NavPlugin<S> {
    pub fn run_on_state(state: S) -> Self {
        Self { state }
    }
}
