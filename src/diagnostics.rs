use bevy::prelude::*;

pub struct DiagnosticsPlugin<'a, S: States> {
    state: &'a S,
}

#[derive(Component)]
struct DiagnosticsOnly;

impl<S: States> Plugin for DiagnosticsPlugin<'static, S> {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::new(100))
            .add_systems(OnEnter(self.state.clone()), setup_diagnostics)
            .add_systems(OnExit(self.state.clone()), teardown_diagnostics);
    }
}

impl<'a, S: States> DiagnosticsPlugin<'a, S> {
    pub fn run_on_state(state: &'a S) -> Self {
        Self { state }
    }
}

fn teardown_diagnostics(
    mut cmds: Commands,
    diagnostics_only_q: Query<Entity, With<DiagnosticsOnly>>,
) {
    for entity in &diagnostics_only_q {
        cmds.entity(entity).despawn();
    }
}

fn setup_diagnostics(mut cmds: Commands) {
    cmds.spawn(DiagnosticsOnly);
}
