use bevy::prelude::*;
use bevy_vrm::mtoon::MtoonSun;
use bevy_vrm::{VrmBundle, VrmPlugin};

pub struct AvatarPlugin;

impl Plugin for AvatarPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Amadeus Avatar".into(),
                transparent: true,
                decorations: false,
                window_level: bevy::window::WindowLevel::AlwaysOnTop,
                resolution: (400., 600.).into(),
                ..default()
            }),
            ..default()
        }))
        // VrmPlugin supports actual .vrm files with SpringBone physics
        .add_plugins(VrmPlugin)
        .insert_resource(ClearColor(Color::NONE))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, animate_idle_pose);
    }
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn((Camera3dBundle {
        camera: Camera {
            clear_color: ClearColorConfig::None,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 1.2, 2.5).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        ..default()
    },));

    // Light with MtoonSun marker for proper VRM MToon shading
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance: 10000.0,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        MtoonSun,
    ));

    // Ambient Light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
    });

    // Load VRM using VrmBundle (v0.0.9 API with SpringBones support)
    commands.spawn((
        VrmBundle {
            vrm: asset_server.load("model/vrm/KurisuMakise.vrm"),
            scene_bundle: SceneBundle {
                transform: Transform::from_xyz(0.0, -0.2, 0.0)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
                    .with_scale(Vec3::splat(1.0)),
                ..default()
            },
            ..default()
        },
        AvatarComponent,
    ));
}

#[derive(Component)]
struct AvatarComponent;

// Procedural idle animation (slight breathing)
fn animate_idle_pose(time: Res<Time>, mut bones: Query<(&Name, &mut Transform)>) {
    let t = time.elapsed_seconds();
    let breathe = (t * 2.0).sin() * 0.01; // subtle breathing scale

    for (name, mut transform) in &mut bones {
        let n = name.as_str();

        match n {
            "J_Bip_C_Chest" | "Chest" | "Spine1" => {
                // Breathing: slightly scale the chest
                transform.scale = Vec3::new(1.0 + breathe, 1.0 + breathe, 1.0 + breathe * 1.5);
            }
            _ => {}
        };
    }
}
