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
