use bevy::prelude::*;
use bevy::render::mesh::morph::MorphWeights;

// Define Events for communication
#[derive(Event)]
pub struct EmotionEvent {
    pub emotion: String, // e.g., "happy", "angry"
}

#[derive(Event)]
pub struct LipSyncEvent {
    pub value: f32, // 0.0 to 1.0 representing mouth openness
}

#[derive(Resource, Default)]
pub struct AvatarState {
    pub is_talking: bool,
    pub emotion: String,
}

pub struct ExpressionPlugin;

impl Plugin for ExpressionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<EmotionEvent>()
            .add_event::<LipSyncEvent>()
            .init_resource::<AvatarState>()
            .add_systems(Update, (handle_events, apply_morph_weights, auto_talk_test));
    }
}

fn handle_events(
    mut lip_sync_events: EventReader<LipSyncEvent>,
    mut emotion_events: EventReader<EmotionEvent>,
    mut state: ResMut<AvatarState>,
) {
    for ev in lip_sync_events.read() {
        state.is_talking = ev.value > 0.0;
    }
    for ev in emotion_events.read() {
        state.emotion = ev.emotion.clone();
    }
}

// Temporary test system to simulate talking randomly
fn auto_talk_test(time: Res<Time>, mut state: ResMut<AvatarState>) {
    // Talk for 2 seconds every 4 seconds as a demo
    let t = time.elapsed_seconds() % 4.0;
    state.is_talking = t < 2.0;
}

fn apply_morph_weights(
    time: Res<Time>,
    state: Res<AvatarState>,
    mut query: Query<&mut MorphWeights>,
) {
    let t = time.elapsed_seconds() * 15.0; // Fast oscillation for talking

    // Smooth target value: if talking, oscillate mouth. Otherwise, mouth closed.
    let talk_target = if state.is_talking {
        t.sin().abs() * 0.8 // 0.0 to 0.8
    } else {
        0.0
    };

    let dt = time.delta_seconds();

    for mut morph in &mut query {
        let weights = morph.weights_mut();

        // VRM models usually have face blendshapes on a mesh with many morph targets (>10).
        if weights.len() > 10 {
            // Index 0 in VRM is often 'A' (Mouth Open)
            if weights.len() > 0 {
                // Smoothly interpolate towards target
                weights[0] += (talk_target - weights[0]) * 10.0 * dt;
            }

            // Blink logic (every ~4 seconds)
            let blink_val = if (time.elapsed_seconds() % 4.0) > 3.8 {
                1.0
            } else {
                0.0
            };
            // Index 2 or 3 is often blink for standard VRM, but we can't be 100% sure without parsing VRM extension.
            // Let's guess index 2 is Blink_L and index 3 is Blink_R
            if weights.len() > 3 {
                weights[2] += (blink_val - weights[2]) * 15.0 * dt;
                weights[3] += (blink_val - weights[3]) * 15.0 * dt;
            }
        }
    }
}
