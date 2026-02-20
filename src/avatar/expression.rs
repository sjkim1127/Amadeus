use bevy::prelude::*;

// Define Events for communication
#[derive(Event)]
pub struct EmotionEvent {
    pub emotion: String, // e.g., "happy", "angry"
}

#[derive(Event)]
pub struct LipSyncEvent {
    pub viseme: String, // e.g., "aa", "oh"
    pub strength: f32,
}

pub struct ExpressionPlugin;

impl Plugin for ExpressionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<EmotionEvent>()
           .add_event::<LipSyncEvent>()
           // .add_systems(Update, handle_expression)
           ;
    }
}
