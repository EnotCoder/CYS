use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, camera_controls)
        .run();
}

#[derive(Component)]
struct RotatingCube;

#[derive(Component)]
struct MainCamera;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) { 
    //cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(5.0, 0.1, 5.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 0.0),
            ..default()
        })),
        Transform::from_xyz(0.0, -1.5, 0.0),
        RotatingCube,
    ));
    
    //light
    commands.spawn((
        DirectionalLight {
            illuminance: 1000.0,
            ..default()
        },
        Transform::from_xyz(3.0, 5.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    
    // Камера для 3D
    commands.spawn((
        MainCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 5.0),
    ));
}


fn camera_controls(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<MainCamera>>,
) {
    let mut transform = match query.single_mut() {
        Ok(t) => t,
        Err(_) => return,
    };
    
    let mut direction = Vec3::ZERO;
    let speed = 5.0;
    let rotation_speed = 1.0;

    let forward = transform.forward().as_vec3();
    let right = transform.right().as_vec3();
    let up = transform.up().as_vec3();
    
    if keyboard.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction -= right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction += right;
    }
    
    if direction != Vec3::ZERO {
        direction = direction.normalize();
        transform.translation += direction * speed * time.delta_secs();
    }

    if keyboard.pressed(KeyCode::KeyQ) {
        transform.rotate_y(rotation_speed * time.delta_secs());
    }
    if keyboard.pressed(KeyCode::KeyE) {
        transform.rotate_y(-rotation_speed * time.delta_secs());
    }

}