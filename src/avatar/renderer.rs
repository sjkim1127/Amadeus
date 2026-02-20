use bevy::prelude::*;

pub struct AvatarPlugin;

impl Plugin for AvatarPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Amadeus Avatar".into(),
                transparent: true,
                decorations: false,
                window_level: bevy::window::WindowLevel::AlwaysOnTop,
                resolution: (400., 600.).into(),
                ..default()
            }),
            ..default()
        }),))
            .insert_resource(ClearColor(Color::NONE))
            .add_systems(Startup, setup_scene)
            .add_systems(Update, rotate_avatar);
    }
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera — framing upper body / head of humanoid
    commands.spawn((Camera3dBundle {
        transform: Transform::from_xyz(0.0, 1.2, 2.5).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        ..default()
    },));

    // Light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Ambient Light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
    });

    // Load VRM model as glTF scene
    let avatar_scene: Handle<Scene> = asset_server.load("model/vrm/KurisuMakise.vrm#Scene0");

    commands.spawn((
        SceneBundle {
            scene: avatar_scene,
            transform: Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(1.0)),
            ..default()
        },
        AvatarComponent,
    ));
}

#[derive(Component)]
struct AvatarComponent;

fn rotate_avatar(time: Res<Time>, mut query: Query<&mut Transform, With<AvatarComponent>>) {
    for mut transform in &mut query {
        transform.rotate_y(0.3 * time.delta_seconds());
    }
}
