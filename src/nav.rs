use bevy::{math::I16Vec3, prelude::*};
use petgraph::{algo::astar, prelude::*};

use crate::{terrain::TerrainWorld, world::TILE_SIZE};

const COARSE_RESOLUTION: i16 = 32_i16;

#[derive(Resource, Default)]
pub struct Navigation {
    nav_graph: UnGraph<I16Vec3, f32>,
}

impl Navigation {
    // I think navigation can actually be done extremely quickly on the fly
    // the alternative is to have nodes be marked as "occupied" when a unit is on or near them
    // which could be a bit of a head ache
    //
    // one way would be to have the rough mesh that finds the path then along that path
    // create another node graph with much finer resolution that gives a smoother looking
    // navigation
    //
    // In this regard we keep things somewhat simple but allow for more minute navigation details
    //
    // For example each node could then have 32 nodes placed ontop of it which would be good enough
    // :TM: for a smooth navigation experience. We can mark nodes that are coliding with entities
    //
    // this allows us to make informed choices, for example if we're attack moving then we need to
    // search around the rough nodes and see if theres an enemy setting that as the target instead
    //
    // For the above to work we need to first implement attack and movement
    // Walk could use the fine grained nav mesh
    //
    // 1. I need to know the "size" of my units
    pub fn rebuild_from_terrain<const N: usize>(&mut self, world: &TerrainWorld<N>) {
        let mut nav_graph: UnGraph<I16Vec3, f32> = default();
        for coord in world.non_water_coordinates() {
            nav_graph.add_node(I16Vec3::new(
                (coord.x + (TILE_SIZE / 2.)) as i16,
                (coord.y + (TILE_SIZE / 2.)) as i16,
                0,
            ));
        }
        for start_index in nav_graph.node_indices() {
            let start_node = nav_graph[start_index];
            let x = start_node.x;
            let y = start_node.y;
            for index in nav_graph.node_indices() {
                let node = nav_graph[index];
                let ox = node.x;
                let oy = node.y;
                if x == ox && y == oy || nav_graph.contains_edge(start_index, index) {
                    continue;
                }
                if (x - ox).abs() <= COARSE_RESOLUTION * 2
                    && (y - oy).abs() <= COARSE_RESOLUTION * 2
                {
                    if x == ox || y == oy {
                        nav_graph.add_edge(start_index, index, 1.);
                        continue;
                    }
                    //navigation
                    //    .nav_graph
                    //    .add_edge(start_index, index, 1.41421356237);
                }
            }
        }
        self.nav_graph = nav_graph;
    }

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

// todo: Cache the whole nav path in a resource
// then have an update for when new blockers are added
fn setup_nav(mut pathing: ResMut<Navigation>) {}

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
            .add_systems(Update, (update_nav).run_if(in_state(self.state.clone())));
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
