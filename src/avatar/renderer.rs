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
            .add_systems(Update, animate_idle_pose);
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
    // VRM is glTF binary — use .glb symlink so Bevy's glTF loader recognizes it
    let avatar_scene: Handle<Scene> = asset_server.load("model/vrm/KurisuMakise.glb#Scene0");

    commands.spawn((
        SceneBundle {
            scene: avatar_scene,
            // Face the camera by rotating 180 degrees (PI radians) around the Y axis
            transform: Transform::from_xyz(0.0, -0.2, 0.0)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
                .with_scale(Vec3::splat(1.0)),
            ..default()
        },
        AvatarComponent,
    ));
}

#[derive(Component)]
struct AvatarComponent;

// Procedural idle animation (A-pose and slight breathing)
fn animate_idle_pose(time: Res<Time>, mut bones: Query<(&Name, &mut Transform)>) {
    let t = time.elapsed_seconds();
    let breathe = (t * 2.0).sin() * 0.01; // subtle breathing scale

    for (name, mut transform) in &mut bones {
        let n = name.as_str();

        // Put arms down to A-pose from T-pose
        match n {
            "J_Bip_L_UpperArm" | "LeftArm" => {
                transform.rotation = Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 1.25);
            }
            "J_Bip_R_UpperArm" | "RightArm" => {
                transform.rotation = Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, -1.25);
            }
            "J_Bip_L_LowerArm" | "LeftForeArm" => {
                transform.rotation = Quat::from_euler(EulerRot::XYZ, 0.2, 0.0, 0.2);
            }
            "J_Bip_R_LowerArm" | "RightForeArm" => {
                transform.rotation = Quat::from_euler(EulerRot::XYZ, 0.2, 0.0, -0.2);
            }
            "J_Bip_C_Chest" | "Chest" | "Spine1" => {
                // Breathing: slightly scale the chest
                transform.scale = Vec3::new(1.0 + breathe, 1.0 + breathe, 1.0 + breathe * 1.5);
            }
            _ => Default::default(),
        };
    }
}
