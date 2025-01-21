use bevy::{math::I16Vec3, prelude::*};
use petgraph::{algo::astar, prelude::*};

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
                    //navigation
                    //    .nav_graph
                    //    .add_edge(start_index, index, 1.41421356237);
                }
            }
        }
    }
}

pub fn update_nav(pos_q: Query<&GlobalTransform>) {}

pub struct NavPlugin<S: States> {
    state: S,
    loading_state: S,
}

impl<S: States> Plugin for NavPlugin<S> {
    fn build(&self, app: &mut App) {
        app.init_resource::<Navigation>()
            .add_systems(
                OnTransition {
                    exited: self.loading_state.clone(),
                    entered: self.state.clone(),
                },
                setup_nav,
            )
            .add_systems(
                Update,
                (update_nav, update_nav_graph_changed).run_if(in_state(self.state.clone())),
            );
    }
}

impl<S: States> NavPlugin<S> {
    pub fn run_on_state(state: S, loading_state: S) -> Self {
        Self {
            state,
            loading_state,
        }
    }
}
