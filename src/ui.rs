use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

use crate::GameState;

#[derive(AssetCollection, Resource)]
struct UiAssets {
    #[asset(path = "ui/pointers/cursor.png")]
    cursor: Handle<Image>,
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
    let node = NodeBundle {
        style: Style {
            width: Val::Px(64.0),
            height: Val::Px(64.0),
            position_type: PositionType::Absolute,
            top: Val::Px(128.),
            left: Val::Px(128.),
            ..default()
        },
        transform: Transform::from_xyz(500., 200., 1.),
        // a `NodeBundle` is transparent by default, so to see the image we have to its color to `WHITE`
        background_color: Color::WHITE.into(),
        ..default()
    };

    cmds.spawn((node, UiImage::new(assets.cursor.clone()), FollowCursor));
}

fn update_ui(mut follow_q: Query<&mut Style, With<FollowCursor>>, window_q: Query<&Window>) {
    let window = window_q.single();
    if let Some(cursor_pos) = window.cursor_position() {
        for mut follower in &mut follow_q {
            follower.left = Val::Px(cursor_pos.x - 20.);
            follower.top = Val::Px(cursor_pos.y - 20.);
        }
    }
}
