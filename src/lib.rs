use bevy::prelude::*;

pub mod building;
pub mod camera;
pub mod characters;
#[cfg(debug_assertions)]
pub mod diagnostics;
pub mod editor;
pub mod flowfield;
pub mod game;
pub mod nav;
pub mod terrain;
pub mod ui;
pub mod world;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum AppState {
    #[default]
    AssetLoading,
    InGame,
    Menu,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, SubStates)]
#[source(AppState = AppState::InGame)]
pub enum InGameState {
    Running,
    #[default]
    InEditor,
    Paused,
    Saving,
    Loading,
}

#[cfg(debug_assertions)]
const IS_DEBUG: bool = true;
#[cfg(not(debug_assertions))]
const IS_DEBUG: bool = false;
