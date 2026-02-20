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

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((Camera3dBundle {
        transform: Transform::from_xyz(0.0, 1.5, 4.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        ..default()
    },));

    // Light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(4.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Ambient Light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.5,
    });

    // Avatar Placeholder (Cuboid)
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::default()),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.2, 0.7, 0.9),
                metallic: 0.5,
                perceptual_roughness: 0.5,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 1.0, 0.0).with_scale(Vec3::new(1.0, 2.0, 0.5)),
            ..default()
        },
        AvatarComponent,
    ));
}

#[derive(Component)]
struct AvatarComponent;

fn rotate_avatar(time: Res<Time>, mut query: Query<&mut Transform, With<AvatarComponent>>) {
    for mut transform in &mut query {
        transform.rotate_y(0.5 * time.delta_seconds());
    }
}
