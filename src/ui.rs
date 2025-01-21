use bevy::prelude::*;
use bevy::winit::cursor::{CursorIcon, CustomCursor};
use bevy_asset_loader::prelude::*;

use crate::AppState;

#[derive(AssetCollection, Resource)]
pub struct UiAssets {
    #[asset(path = "ui/pointers/cursor_no_space.png")]
    cursor: Handle<Image>,
    #[asset(path = "ui/pointers/select.png")]
    pub select: Handle<Image>,

    #[asset(path = "ui/banners/banner_vertical.png")]
    banner_texture: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 64, tile_size_y = 64, columns = 3, rows = 3))]
    banner_layout: Handle<TextureAtlasLayout>,
}

#[derive(Component)]
pub struct FollowCursor;

pub struct UiPlugin<S: States> {
    state: S,
}

impl<S: States> Plugin for UiPlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_loading_state(
            LoadingStateConfig::new(AppState::AssetLoading).load_collection::<UiAssets>(),
        )
        .add_systems(OnExit(AppState::AssetLoading), setup_cursor);
    }
}

impl<S: States> UiPlugin<S> {
    pub fn run_on_state(state: S) -> Self {
        Self { state }
    }
}

fn setup_cursor(
    mut cmds: Commands,
    assets: Res<UiAssets>,
    window_entity: Single<Entity, With<Window>>,
) {
    let cursor_image: CursorIcon = CustomCursor::Image {
        handle: assets.cursor.clone(),
        hotspot: (0, 0),
    }
    .into();
    cmds.entity(*window_entity).insert(cursor_image.clone());
}
