use bevy::prelude::*;
use bevy::ui::UiPlugin;
use bevy_asset_loader::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::EntropyPlugin;
use tinyswords::building::BuildingPlugin;
use tinyswords::camera::CameraPlugin;
use tinyswords::characters::CharacterPlugin;
#[cfg(debug_assertions)]
use tinyswords::diagnostics::DiagnosticsPlugin;
use tinyswords::editor::EditorPlugin;
use tinyswords::game::GamePlugin;
use tinyswords::nav::NavPlugin;
use tinyswords::world::WorldPlugin;
use tinyswords::GameState;
const RUNNING_STATE: [GameState; 2] = [GameState::InEditor, GameState::InGame];

fn main() {
    let loading_state = GameState::AssetLoading;
    let first_state = GameState::InEditor;
    let other_state = GameState::InGame;
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Tiny Swords".into(),
                    name: Some("ts.app".into()),
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
                default_sampler: bevy::image::ImageSamplerDescriptor::nearest(),
            }),
    )
    .init_state::<GameState>()
    .add_loading_state(
        // set our initial state after assets have loaded
        LoadingState::new(GameState::AssetLoading).continue_to_state(first_state.clone()),
    )
    .add_plugins(EntropyPlugin::<WyRand>::default())
    // todo: We need some systems to only load in game and some systems
    .add_plugins(tinyswords::ui::UiPlugin::run_on_state(GameState::InEditor))
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
    .add_plugins(BuildingPlugin::run_on_state_or(
        first_state.clone(),
        other_state.clone(),
        loading_state.clone(),
    ))
    .add_plugins(CameraPlugin::run_on_state_or(
        first_state.clone(),
        other_state.clone(),
        loading_state.clone(),
    ));

    #[cfg(debug_assertions)]
    {
        app.add_plugins(DiagnosticsPlugin::run_on_states(&RUNNING_STATE));
    }

    app.run();
}
