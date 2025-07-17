use bevy::prelude::*;

// flowfield feels like a great method for "course" navigation
// I'm thinking of using the flowfield for general navigation then once nearing the target
// for attacking however it might be best to use a more accurate method
pub struct FlowFieldPlugin<S: States> {
    state: S,
    loading_state: S,
}

impl<S: States> Plugin for FlowFieldPlugin<S> {
    fn build(&self, app: &mut App) {
        app;
    }
}

impl<S: States> FlowFieldPlugin<S> {
    pub fn run_on_state(state: S, loading_state: S) -> Self {
        Self {
            state,
            loading_state,
        }
    }
}
