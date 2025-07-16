use bevy::prelude::*;
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
use tinyswords::ui::UiPlugin;
use tinyswords::AppState;
use tinyswords::{terrain::*, InGameState};

fn main() {
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
            .set(ImagePlugin::default_nearest()),
    )
    .init_state::<AppState>()
    .init_state::<InGameState>()
    .add_loading_state(
        // set our initial state after assets have loaded
        LoadingState::new(AppState::AssetLoading).continue_to_state(AppState::InGame),
    )
    .add_plugins(EntropyPlugin::<WyRand>::default())
    .add_plugins(UiPlugin::run_on_state(InGameState::InEditor))
    .add_plugins(EditorPlugin::run_on_state(
        InGameState::InEditor,
        AppState::AssetLoading,
    ))
    .add_plugins(GamePlugin::run_on_state(
        InGameState::Running,
        AppState::AssetLoading,
    ))
    .add_plugins(CharacterPlugin::run_on_state(
        AppState::InGame,
        AppState::AssetLoading,
    ))
    .add_plugins(NavPlugin::run_on_state(
        AppState::InGame,
        AppState::AssetLoading,
    ))
    .add_plugins(BuildingPlugin::run_on_state(
        AppState::InGame,
        AppState::AssetLoading,
    ))
    .add_plugins(CameraPlugin::run_on_state(
        AppState::InGame,
        AppState::AssetLoading,
    ))
    .add_plugins(TerrainPlugin::run_on_state(
        AppState::InGame,
        AppState::AssetLoading,
    ));

    #[cfg(debug_assertions)]
    {
        app.add_plugins(DiagnosticsPlugin::run_on_state(&AppState::InGame));
    }

    app.run();
}
