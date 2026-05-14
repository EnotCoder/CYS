use bevy::prelude::*;


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}


fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>
) {
    commands.spawn(Camera2d::default());

    let texture_handle = asset_server.load("v1.png");

    commands.spawn((
        Sprite {
            color: Color::srgb(1.0, 1.0, 1.0),
            custom_size: Some(Vec2::new(100.0, 100.0)),
            image: texture_handle,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::default(),
    ));
}