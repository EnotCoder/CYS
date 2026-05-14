use bevy::prelude::*;
use bevy::window::WindowResolution;

fn main() {
    App::new()
        // Этот метод меняет фильтрацию по умолчанию с Linear на Nearest
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: WindowResolution::new(800, 600),
                        title: "Little Secret".to_string(),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
        )
        .add_systems(Startup, setup)
        .add_systems(Update, controls)
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Box;

fn controls(
    mut player_query: Query<&mut Transform, With<Player>>,
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
){
    let mut transform = match player_query.single_mut() {
        Ok(t) => t,
        Err(_) => return,
    };

    let mut direction = Vec3::ZERO;

    if input.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if input.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if input.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }
    if input.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    
    if direction != Vec3::ZERO {
        direction = direction.normalize();
    }
    
    let speed = 300.0;
    transform.translation += direction * speed * time.delta_secs();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>
) {
    commands.spawn(Camera2d::default());

    let texture_handle = asset_server.load("player.png");
    let box_texture = asset_server.load("box.png");

    commands.spawn((
        Player,
        Sprite {
            color: Color::srgb(1.0, 1.0, 1.0),
            custom_size: Some(Vec2::new(100.0, 100.0)),
            image: texture_handle,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::default(),
    ));

    commands.spawn((
        Box,
        Sprite {
            color: Color::srgb(1.0, 1.0, 1.0),
            custom_size: Some(Vec2::new(100.0, 100.0)),
            image: box_texture,
            ..default()
        },
        Transform::from_xyz(100.0, 0.0, 11.0),
        Visibility::default(),
    ));
}