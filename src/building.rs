use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use std::time::Duration;

pub const ANIMATION_SPEED: Duration = Duration::from_millis(100);

#[derive(AssetCollection, Resource)]
pub struct BuildingAssets {
    #[asset(path = "factions/knights/buildings/castle/castle_blue.png")]
    pub castle_texture: Handle<Image>,
    #[asset(path = "factions/knights/buildings/castle/castle_construction.png")]
    pub castle_construction_texture: Handle<Image>,
    #[asset(path = "factions/knights/buildings/castle/castle_destroyed.png")]
    pub castle_destroyed_texture: Handle<Image>,

    #[asset(path = "factions/knights/buildings/house/house_blue.png")]
    pub house_texture: Handle<Image>,
    #[asset(path = "factions/knights/buildings/house/house_construction.png")]
    pub house_construction_texture: Handle<Image>,
    #[asset(path = "factions/knights/buildings/house/douse_destroyed.png")]
    pub house_destroyed_texture: Handle<Image>,

    #[asset(path = "factions/knights/buildings/tower/tower_blue.png")]
    pub tower_texture: Handle<Image>,
    #[asset(path = "factions/knights/buildings/tower/tower_construction.png")]
    pub tower_construction_texture: Handle<Image>,
    #[asset(path = "factions/knights/buildings/tower/tower_destroyed.png")]
    pub tower_destroyed_texture: Handle<Image>,
}
impl BuildingAssets {}

pub struct BuildingPlugin<S: States> {
    state: S,
    loading_state: S,
}

impl<S: States + bevy::state::state::FreelyMutableState> Plugin for BuildingPlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_loading_state(
            LoadingStateConfig::new(self.loading_state.clone()).load_collection::<BuildingAssets>(),
        );
    }
}

impl<S: States> BuildingPlugin<S> {
    pub fn run_on_state(state: S, loading_state: S) -> Self {
        Self {
            state,
            loading_state,
        }
    }
}
