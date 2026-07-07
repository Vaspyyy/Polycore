mod constants;
mod rng;
mod player;
mod projectile;
mod shape;
mod collision;
mod hud;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: constants::WINDOW_TITLE.into(),
                resolution: (constants::WINDOW_WIDTH as u32, constants::WINDOW_HEIGHT as u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgba(
            constants::BG_COLOR[0],
            constants::BG_COLOR[1],
            constants::BG_COLOR[2],
            constants::BG_COLOR[3],
        )))
        .insert_resource(rng::Rng::new(12345))
        .add_systems(Startup, (
            setup_camera,
            player::setup_player,
            shape::setup_xp,
            hud::setup_hud,
        ))
        .add_systems(Update, (
            player::player_aim,
            projectile::shoot_projectile,
            camera_follow,
            draw_grid,
            hud::update_hud,
        ).chain())
        .add_systems(FixedUpdate, (
            player::player_movement,
            projectile::projectile_update,
            shape::shape_spawn,
            collision::check_collisions,
            shape::check_level_up,
        ).chain())
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn camera_follow(
    player: Query<&Transform, With<player::Player>>,
    mut camera: Query<&mut Transform, (With<Camera>, Without<player::Player>)>,
) {
    let Ok(player_transform) = player.single() else { return };
    let Ok(mut camera_transform) = camera.single_mut() else { return };
    camera_transform.translation.x = player_transform.translation.x;
    camera_transform.translation.y = player_transform.translation.y;
}

fn draw_grid(mut gizmos: Gizmos) {
    let color = Color::srgba(
        constants::GRID_COLOR[0],
        constants::GRID_COLOR[1],
        constants::GRID_COLOR[2],
        constants::GRID_COLOR[3],
    );
    let extent = constants::GRID_EXTENT;
    let spacing = constants::GRID_SPACING;
    let half = extent / 2.0;

    let start = -half;
    let mut x = start;
    while x <= half {
        gizmos.line_2d(Vec2::new(x, -half), Vec2::new(x, half), color);
        x += spacing;
    }
    let mut y = start;
    while y <= half {
        gizmos.line_2d(Vec2::new(-half, y), Vec2::new(half, y), color);
        y += spacing;
    }
}
