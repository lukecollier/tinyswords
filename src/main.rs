use bevy::prelude::*;
use bevy::render::texture::ImageSamplerDescriptor;
use bevy::window::Cursor;
use bevy_asset_loader::prelude::*;
use tinyswords::camera::CameraPlugin;
use tinyswords::ui::UiPlugin;
use tinyswords::world::WorldPlugin;
use tinyswords::GameState;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Tiny Swords".into(),
                        name: Some("ts.app".into()),
                        cursor: Cursor {
                            visible: false,
                            ..default()
                        },
                        resolution: (1270., 720.).into(),
                        present_mode: bevy::window::PresentMode::AutoNoVsync,
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
            LoadingState::new(GameState::AssetLoading).continue_to_state(GameState::InGame),
        )
        .add_plugins(WorldPlugin::run_on_state(GameState::InGame))
        .add_plugins(CameraPlugin::run_on_state(GameState::InGame))
        .add_plugins(UiPlugin::run_on_state(GameState::InGame))
        .run();
}
