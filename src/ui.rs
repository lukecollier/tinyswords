use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

use crate::GameState;

#[derive(AssetCollection, Resource)]
pub struct UiAssets {
    #[asset(path = "ui/pointers/cursor_no_space.png")]
    cursor: Handle<Image>,
    #[asset(path = "ui/pointers/select.png")]
    pub select: Handle<Image>,

    #[asset(path = "ui/banners/banner_vertical.png")]
    banner_texture: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 64., tile_size_y = 64., columns = 3, rows = 3))]
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
            LoadingStateConfig::new(GameState::AssetLoading).load_collection::<UiAssets>(),
        )
        .add_systems(OnEnter(self.state.clone()), setup_ui)
        .add_systems(Update, update_ui.run_if(in_state(self.state.clone())));
    }
}

impl<S: States> UiPlugin<S> {
    pub fn run_on_state(state: S) -> Self {
        Self { state }
    }
}

fn setup_ui(mut cmds: Commands, assets: Res<UiAssets>) {
    let ui_cursor = ImageBundle {
        style: Style {
            width: Val::Px(22.0),
            height: Val::Px(30.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        z_index: ZIndex::Global(100),
        background_color: Color::WHITE.into(),
        image: UiImage::new(assets.cursor.clone()),
        ..default()
    };
    cmds.spawn((ui_cursor, FollowCursor));
}

fn update_ui(mut follow_q: Query<&mut Style, With<FollowCursor>>, window_q: Query<&Window>) {
    let window = window_q.single();
    if let Some(cursor_pos) = window.cursor_position() {
        for mut follower in &mut follow_q {
            follower.left = Val::Px(cursor_pos.x);
            follower.top = Val::Px(cursor_pos.y);
        }
    }
}
