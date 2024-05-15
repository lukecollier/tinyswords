use bevy::prelude::*;
use bevy::render::texture::ImageSamplerDescriptor;
use bevy::window::Cursor;
use bevy_asset_loader::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::EntropyPlugin;
use tinyswords::camera::CameraPlugin;
use tinyswords::characters::CharacterPlugin;
use tinyswords::editor::EditorPlugin;
use tinyswords::game::GamePlugin;
use tinyswords::nav::NavPlugin;
use tinyswords::world::WorldPlugin;
use tinyswords::GameState;

fn main() {
    let loading_state = GameState::AssetLoading;
    let first_state = GameState::InEditor;
    let other_state = GameState::InGame;
    App::new()
        .add_plugins(
            DefaultPlugins
                // todo: https://github.com/rust-windowing/winit/blob/ab33fb8eda45f9a23587465d787a70a309c67ec4/src/changelog/v0.30.md?plain=1#L17
                // the above allows a custom cursor to be set, currently bevy isn't using stable 0.29
                // awaiting https://github.com/bevyengine/bevy/pull/13254
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Tiny Swords".into(),
                        name: Some("ts.app".into()),
                        cursor: Cursor {
                            visible: true,
                            ..default()
                        },
                        resolution: (1270., 720.).into(),
                        present_mode: bevy::window::PresentMode::AutoVsync,
                        window_theme: None,
                        enabled_buttons: bevy::window::EnabledButtons {
                            maximize: true,
                            ..default()
                        },
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin {
                    default_sampler: ImageSamplerDescriptor::nearest(),
                }),
        )
        .insert_resource(Msaa::Off) // stop's texture bleeding
        .init_state::<GameState>()
        .add_loading_state(
            // set our initial state after assets have loaded
            LoadingState::new(GameState::AssetLoading).continue_to_state(first_state.clone()),
        )
        .add_plugins(EntropyPlugin::<WyRand>::default())
        // todo: We need some systems to only load in game and some systems
        .add_plugins(EditorPlugin::run_on_state(
            GameState::InEditor,
            GameState::AssetLoading,
        ))
        .add_plugins(GamePlugin::run_on_state(
            GameState::InGame,
            loading_state.clone(),
        ))
        .add_plugins(WorldPlugin::run_on_state_or(
            first_state.clone(),
            other_state.clone(),
            loading_state.clone(),
        ))
        .add_plugins(CharacterPlugin::run_on_state_or(
            first_state.clone(),
            other_state.clone(),
            loading_state.clone(),
        ))
        .add_plugins(NavPlugin::run_on_state_or(
            first_state.clone(),
            other_state.clone(),
            loading_state.clone(),
        ))
        .add_plugins(CameraPlugin::run_on_state_or(
            first_state.clone(),
            other_state.clone(),
            loading_state.clone(),
        ))
        .run();
}
