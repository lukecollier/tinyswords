use bevy::{
    math::{I16Vec3, I64Vec2},
    prelude::*,
    utils::{
        hashbrown::HashSet,
        petgraph::{algo::astar, prelude::*},
    },
};

use crate::world::TILE_SIZE;

const COARSE_RESOLUTION: i16 = 32_i16;

#[derive(Resource, Default)]
pub struct Navigation {
    allowed: Vec<Rect>,
    disallowed: Vec<Rect>,
    // todo: Instead we update the nav graph with where has been allowed or disallowed
    nav_graph: UnGraph<I16Vec3, f32>,
}

impl Navigation {
    pub fn is_walkable(&self, xy: Vec2) -> bool {
        self.nav_graph.node_indices().any(|node| {
            let point = self.nav_graph[node];
            let rect = Rect::from_corners(
                (point - COARSE_RESOLUTION).truncate().as_vec2(),
                (point + COARSE_RESOLUTION).truncate().as_vec2(),
            );

            rect.contains(xy)
        })
    }

    // todo(improvement): We can actually increase the resolution along the found path
    // todo(improvement): should be able to handle z
    pub fn path_between_3d(&self, start: Vec3, end: Vec3) -> Vec<Vec3> {
        // this would be quite slow, but _probably_ faster then calculating it ad-hoc... probably?
        let graph = &self.nav_graph;
        // todo: this should ideally be the closest in the direction of travel
        let mut closest_to_start = I16Vec3::ZERO;
        let mut closest_to_end = I16Vec3::ZERO;
        let mut finish_node_opt: Option<NodeIndex> = None;
        let mut start_node_opt: Option<NodeIndex> = None;
        for node_id in graph.node_indices() {
            let point = graph[node_id];
            if start.distance(point.as_vec3()) < start.distance(closest_to_start.as_vec3()) {
                closest_to_start = point;
                start_node_opt = Some(node_id);
            }
            if end.distance(point.as_vec3()) < end.distance(closest_to_end.as_vec3()) {
                closest_to_end = point;
                finish_node_opt = Some(node_id);
            }
        }
        let Some(end_node) = finish_node_opt else {
            return vec![];
        };
        let Some(start_node) = start_node_opt else {
            return vec![];
        };
        // todo(improvement): After we return the A* path, we then make a higher resolution node
        // graph and repeat the above process
        if let Some((_, astar_path)) = astar(
            &graph,
            start_node,
            |finish| finish == end_node,
            |e| *e.weight(),
            |_| 0.0,
        ) {
            return astar_path
                .iter()
                .map(|node| graph[*node].as_vec3())
                .collect();
        } else {
            vec![]
        }
    }

    pub fn debug(&self, mut gizmos: Gizmos) {
        for node_id in self.nav_graph.node_indices() {
            let pos = self.nav_graph[node_id];
            gizmos.circle_2d(pos.truncate().as_vec2(), 2., Color::WHITE);
        }
        for a in self.nav_graph.node_indices() {
            for b in self.nav_graph.node_indices() {
                if self.nav_graph.find_edge(a, b).is_some() {
                    let a_pos = self.nav_graph[a];
                    let b_pos = self.nav_graph[b];
                    gizmos.line_2d(
                        a_pos.truncate().as_vec2(),
                        b_pos.truncate().as_vec2(),
                        Color::WHITE,
                    );
                }
            }
        }
    }
}

//todo(improvement): We should check when allowed nav points have blockers in their tile.
//If they don't they won't need the finer resolution.
//furthermore if theres no blockers between two paths we can also merge them into a single point in
//the path
pub fn nav_graph_from_path(
    allowed: &Vec<Rect>,
    blocked: &Vec<Rect>,
    coarse_path: Vec<I64Vec2>,
    resolution: usize,
) -> UnGraph<I64Vec2, f32> {
    let fine_size = TILE_SIZE as i64;
    let half_size = TILE_SIZE as i64 / 2;
    let nodes_number =
        (coarse_path.len() as f32 * (fine_size as f32 / resolution as f32)).ceil() as usize;
    let mut graph: UnGraph<I64Vec2, f32> = Graph::with_capacity(nodes_number, nodes_number);
    for waypoint in coarse_path {
        for x in (-half_size..=half_size).step_by(resolution as usize) {
            for y in (-half_size..=half_size).step_by(resolution as usize) {
                let point = waypoint + I64Vec2::new(x, y);
                let mut found = true;
                for allow in allowed {
                    if allow.contains(point.as_vec2()) {
                        found = false;
                    }
                }
                for block in blocked {
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
    }
    let points: HashSet<_> = graph
        .node_indices()
        .map(|node| {
            let point = graph[node];
            (point.x as i64, point.y as i64, node)
        })
        .collect();

    for (x, y, node) in points.iter() {
        for (ox, oy, onode) in points.iter() {
            if x == ox && y == oy || graph.contains_edge(*node, *onode) {
                continue;
            }
            if (x - ox).abs() <= resolution as i64 * 2 && (y - oy).abs() <= resolution as i64 * 2 {
                if x == ox || y == oy {
                    graph.add_edge(*node, *onode, 1.);
                    continue;
                }
                graph.add_edge(*node, *onode, 1.41421356237);
            }
        }
    }
    graph
}

// pub fn nav_graph_from(
//     allowed: &Vec<Rect>,
//     blocked: &Vec<Rect>,
//     bounds: &Rect,
// ) -> UnGraph<I16Vec3, f32> {
//     fn exclusive_contains(rect: &Rect, point: &Vec2) -> bool {
//         (point.cmpgt(rect.min) & point.cmplt(rect.max)).all()
//     }
//     // todo: Should be as big as the entity that's moving
//     let nodes = ((bounds.max.x - bounds.min.x) / COARSE_RESOLUTION as f32) as usize
//         * ((bounds.max.y - bounds.min.y) / COARSE_RESOLUTION as f32) as usize;
//     let mut graph: UnGraph<I64Vec2, f32> = Graph::with_capacity(nodes + 2, nodes + 2);
//     for x in ((bounds.min.x as i64)..(bounds.max.x as i64)).step_by(COARSE_RESOLUTION as usize) {
//         for y in ((bounds.min.y as i64)..(bounds.max.y as i64)).step_by(COARSE_RESOLUTION as usize)
//         {
//             let point = I64Vec2::new(x, y);
//             let mut found = true;
//             for allow in allowed {
//                 if exclusive_contains(allow, &point.as_vec2()) {
//                     found = false;
//                 }
//             }
//             for block in blocked {
//                 if exclusive_contains(block, &point.as_vec2()) {
//                     found = true;
//                     break;
//                 }
//             }
//             if !found {
//                 graph.add_node(point);
//             }
//         }
//     }
//     let points: HashSet<_> = graph
//         .node_indices()
//         .map(|node| {
//             let point = graph[node];
//             (point.x as i16, point.y as i16, node)
//         })
//         .collect();

//     for (x, y, node) in points.iter() {
//         for (ox, oy, onode) in points.iter() {
//             if x == ox && y == oy || graph.contains_edge(*node, *onode) {
//                 continue;
//             }
//             if (x - ox).abs() <= COARSE_RESOLUTION * 2 && (y - oy).abs() <= COARSE_RESOLUTION * 2 {
//                 if x == ox || y == oy {
//                     graph.add_edge(*node, *onode, 1.);
//                     continue;
//                 }
//                 graph.add_edge(*node, *onode, 1.41421356237);
//             }
//         }
//     }
//     graph
// }

// todo: For a better solution, use a nav mesh, find the points, then offset the points by the
// normal of the two connecting lines to avoid a wonky looking path

// need's a GlobalTransform, blocks navigation through this entity
#[derive(Component)]
pub struct NavSquare {
    pub size: Vec2,
    pub walkable: bool,
}

#[derive(Bundle)]
pub struct NavBundle {
    transform: TransformBundle,
    blocker: NavSquare,
}

impl NavBundle {
    pub fn blocked(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::from_xy(Vec2::new(x, y), Vec2::new(width, height), false)
    }

    pub fn allowed(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::from_xy(Vec2::new(x, y), Vec2::new(width, height), true)
    }
    pub fn from_xy(xy: Vec2, size: Vec2, walkable: bool) -> Self {
        Self {
            transform: TransformBundle::from_transform(Transform::from_translation(xy.extend(0.0))),
            blocker: NavSquare { size, walkable },
        }
    }
}

// todo: Cache the whole nav path in a resource
// then have an update for when new blockers are added
fn setup_nav(mut pathing: ResMut<Navigation>) {}

// todo(improvement): Building the graph everytime is wasteful, also this method of NavSquares
// doesn't seem useful. We could instead add the nodes at discrete points and then connect them
fn update_nav_graph_changed(
    mut navigation: ResMut<Navigation>,
    changed_nav_q: Query<
        (Ref<GlobalTransform>, Ref<NavSquare>),
        (
            Or<(Changed<NavSquare>, Changed<GlobalTransform>)>,
            With<NavSquare>,
        ),
    >,
) {
    for (transform, nav_square) in &changed_nav_q {
        let position = transform.translation().truncate();
        let area = Rect::from_corners(position, position + nav_square.size);
        if nav_square.walkable {
            navigation.allowed.push(area);
        } else {
            navigation.disallowed.push(area);
        }
        if transform.is_changed() || nav_square.is_changed() {
            let start_index = navigation
                .nav_graph
                .add_node(area.center().extend(0.0).as_i16vec3());
            let start_node = navigation.nav_graph[start_index];
            let x = start_node.x;
            let y = start_node.y;
            for index in navigation.nav_graph.node_indices() {
                let node = navigation.nav_graph[index];
                let ox = node.x;
                let oy = node.y;
                if x == ox && y == oy || navigation.nav_graph.contains_edge(start_index, index) {
                    continue;
                }
                if (x - ox).abs() <= COARSE_RESOLUTION * 2
                    && (y - oy).abs() <= COARSE_RESOLUTION * 2
                {
                    if x == ox || y == oy {
                        navigation.nav_graph.add_edge(start_index, index, 1.);
                        continue;
                    }
                    navigation
                        .nav_graph
                        .add_edge(start_index, index, 1.41421356237);
                }
            }
        }
    }
}

pub fn update_nav(pos_q: Query<&GlobalTransform>) {}

pub struct NavPlugin<S: States> {
    state: S,
    or_state: S,
    loading_state: S,
}

impl<S: States> Plugin for NavPlugin<S> {
    fn build(&self, app: &mut App) {
        app.init_resource::<Navigation>()
            .add_systems(
                OnTransition {
                    from: self.loading_state.clone(),
                    to: self.state.clone(),
                },
                setup_nav,
            )
            .add_systems(
                OnTransition {
                    from: self.loading_state.clone(),
                    to: self.or_state.clone(),
                },
                setup_nav,
            )
            .add_systems(
                Update,
                (update_nav, update_nav_graph_changed)
                    .run_if(in_state(self.state.clone()).or_else(in_state(self.or_state.clone()))),
            );
    }
}

impl<S: States> NavPlugin<S> {
    pub fn run_on_state_or(state: S, or_state: S, loading_state: S) -> Self {
        Self {
            state,
            or_state,
            loading_state,
        }
    }
}
