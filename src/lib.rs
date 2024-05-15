use bevy::prelude::*;

pub mod camera;
pub mod characters;
pub mod editor;
pub mod game;
pub mod nav;
pub mod ui;
pub mod world;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameState {
    #[default]
    AssetLoading,
    InGame,
    InEditor,
}

#[cfg(debug_assertions)]
const IS_DEBUG: bool = true;
#[cfg(not(debug_assertions))]
const IS_DEBUG: bool = false;

// todo: Move events here
