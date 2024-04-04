use bevy::prelude::*;

pub mod camera;
pub mod ui;
pub mod world;
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameState {
    #[default]
    AssetLoading,
    InMainMenu,
    Paused,
    InGame,
}

#[cfg(debug_assertions)]
const IS_DEBUG: bool = true;
#[cfg(not(debug_assertions))]
const IS_DEBUG: bool = false;
