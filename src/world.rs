/**
 * This is the plugin for the world, it's animations, and creating blocking
 */
use bevy::app::{App, Plugin};

pub struct WorldPlugin;

impl WorldPlugin {
    pub fn new() -> Self {
        WorldPlugin {}
    }
}

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        // add things to your app here
    }
}
