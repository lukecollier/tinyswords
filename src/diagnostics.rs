use bevy::prelude::*;

pub struct DiagnosticsPlugin<'a, S: States> {
    active_states: &'a [S],
}

#[derive(Component)]
struct DiagnosticsOnly;

impl<S: States> Plugin for DiagnosticsPlugin<'static, S> {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin);
        for state in self.active_states {
            app.add_systems(OnEnter(state.clone()), setup_diagnostics)
                .add_systems(OnExit(state.clone()), teardown_diagnostics);
        }
    }
}

impl<'a, S: States> DiagnosticsPlugin<'a, S> {
    pub fn run_on_states(active_states: &'a [S]) -> Self {
        Self { active_states }
    }
}

fn teardown_diagnostics(
    mut cmds: Commands,
    diagnostics_only_q: Query<Entity, With<DiagnosticsOnly>>,
) {
    for entity in &diagnostics_only_q {
        cmds.entity(entity).despawn_recursive();
    }
}

fn setup_diagnostics(mut cmds: Commands) {
    cmds.spawn(DiagnosticsOnly);
}
