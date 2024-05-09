use bevy::prelude::*;
use bevy::render::texture::ImageSamplerDescriptor;
use bevy::window::Cursor;
use bevy_asset_loader::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::EntropyPlugin;
use tinyswords::camera::CameraPlugin;
use tinyswords::editor::EditorPlugin;
use tinyswords::nav::NavPlugin;
use tinyswords::ui::UiPlugin;
use tinyswords::unit::UnitPlugin;
use tinyswords::world::WorldPlugin;
use tinyswords::GameState;

fn main() {
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
            LoadingState::new(GameState::AssetLoading).continue_to_state(GameState::InGame),
        )
        .add_plugins(EntropyPlugin::<WyRand>::default())
        .add_plugins(EditorPlugin::run_on_state(GameState::InGame))
        .add_plugins(WorldPlugin::run_on_state(GameState::InGame))
        .add_plugins(NavPlugin::run_on_state(GameState::InGame))
        .add_plugins(CameraPlugin::run_on_state(GameState::InGame))
        // .add_plugins(UiPlugin::run_on_state(GameState::InGame))
        .add_plugins(UnitPlugin::run_on_state(GameState::InGame))
        .run();
}
