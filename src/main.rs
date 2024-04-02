use bevy::prelude::*;

pub mod world;

#[cfg(debug_assertions)]
const IS_DEBUG: bool = true;
#[cfg(not(debug_assertions))]
const IS_DEBUG: bool = false;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum GameState {
    #[default]
    AssetLoading,
    InMainMenu,
    Paused,
    InGame,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, world::WorldPlugin::new()))
        .run();
}
