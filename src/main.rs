mod collision;
mod constants;
mod enemy_bot;
mod evolution;
mod hud;
mod menu;
mod player;
mod projectile;
mod rng;
mod shape;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: constants::WINDOW_TITLE.into(),
                    resolution: (
                        constants::WINDOW_WIDTH as u32,
                        constants::WINDOW_HEIGHT as u32,
                    )
                        .into(),
                    ..default()
                }),
                ..default()
            }),
        )
        .insert_resource(ClearColor(Color::srgba(
            constants::BG_COLOR[0],
            constants::BG_COLOR[1],
            constants::BG_COLOR[2],
            constants::BG_COLOR[3],
        )))
        .insert_resource(rng::Rng::new(12345))
        .insert_resource(menu::GamePhase::Menu)
        .insert_resource(menu::GameMode::Singleplayer)
        .insert_resource(menu::PlayerName::load())
        .insert_resource(menu::NameInputFocus::default())
        .insert_resource(menu::RunStats::default())
        .insert_resource(menu::DeathSummary::default())
        .insert_resource(hud::UpgradeState::default())
        .insert_resource(evolution::EvolutionState::default())
        .add_systems(
            Startup,
            (
                setup_camera,
                setup_grid,
                player::setup_player,
                enemy_bot::setup_enemy_bots,
                shape::setup_xp,
                hud::setup_hud,
                evolution::setup_evolution_menu,
                menu::setup_menu,
            ),
        )
        .add_systems(
            Update,
            (
                menu::handle_play_button,
                menu::handle_mode_buttons,
                menu::update_mode_highlight,
                menu::handle_name_field_clicks,
                menu::handle_name_keyboard,
                menu::sync_player_name_text,
                menu::handle_death_buttons,
                menu::sync_phase_visibility,
                enemy_bot::sync_enemy_bot_visibility,
                menu::sync_death_summary,
                evolution::queue_evolution_choices,
                evolution::update_evolution_menu,
                evolution::handle_evolution_buttons,
                evolution::update_evolution_hover_description,
                hud::update_upgrade_menu,
                menu::tick_run_stats.run_if(menu::is_playing),
            ),
        )
        .add_systems(
            Update,
            (
                player::player_aim,
                player::update_barrel,
                player::update_player_upgrade_stats,
                player::regenerate_player_health,
                player::update_health_bar,
                enemy_bot::update_enemy_bot_health_bars,
                enemy_bot::spin_enemy_bot_turrets,
                shape::update_shape_health_bars,
                projectile::shoot_projectile,
                camera_follow,
                hud::update_hud,
                hud::animate_upgrade_fills,
                hud::handle_upgrade_buttons,
                hud::handle_upgrade_hotkeys,
            )
                .chain()
                .run_if(menu::is_playing),
        )
        .add_systems(
            FixedUpdate,
            (
                player::player_movement,
                enemy_bot::enemy_bot_knockback_update,
                shape::shape_knockback_update,
                projectile::projectile_update,
                shape::shape_spawn,
                collision::check_collisions,
                collision::check_projectile_enemy_bot_collisions,
                collision::check_player_enemy_bot_collisions,
                collision::check_player_shape_collisions,
                collision::check_enemy_bot_shape_collisions,
                collision::check_shape_shape_collisions,
                shape::check_level_up,
            )
                .chain()
                .run_if(menu::is_playing),
        )
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn camera_follow(
    player: Query<&Transform, With<player::Player>>,
    mut camera: Query<&mut Transform, (With<Camera>, Without<player::Player>)>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };
    let half = constants::arena_half_extent();
    let max_camera_x = (half - constants::WINDOW_WIDTH / 2.0).max(0.0);
    let max_camera_y = (half - constants::WINDOW_HEIGHT / 2.0).max(0.0);

    camera_transform.translation.x = player_transform
        .translation
        .x
        .clamp(-max_camera_x, max_camera_x);
    camera_transform.translation.y = player_transform
        .translation
        .y
        .clamp(-max_camera_y, max_camera_y);
}

fn setup_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let grid_material = materials.add(Color::srgba(
        constants::GRID_COLOR[0],
        constants::GRID_COLOR[1],
        constants::GRID_COLOR[2],
        constants::GRID_COLOR[3],
    ));
    let border_material = materials.add(Color::srgba(
        constants::BORDER_COLOR[0],
        constants::BORDER_COLOR[1],
        constants::BORDER_COLOR[2],
        constants::BORDER_COLOR[3],
    ));
    let extent = constants::GRID_EXTENT;
    let spacing = constants::GRID_SPACING;
    let half = constants::arena_half_extent();
    let thickness = 2.0;
    let grid_z = -10.0;
    let border_z = -9.0;

    let vertical_line = meshes.add(Rectangle::new(thickness, extent));
    let horizontal_line = meshes.add(Rectangle::new(extent, thickness));

    let mut x = -half;
    while x <= half {
        commands.spawn((
            Mesh2d(vertical_line.clone()),
            MeshMaterial2d(grid_material.clone()),
            Transform::from_xyz(x, 0.0, grid_z),
        ));
        x += spacing;
    }

    let mut y = -half;
    while y <= half {
        commands.spawn((
            Mesh2d(horizontal_line.clone()),
            MeshMaterial2d(grid_material.clone()),
            Transform::from_xyz(0.0, y, grid_z),
        ));
        y += spacing;
    }

    let border_thickness = constants::BORDER_THICKNESS;
    let vertical_border = meshes.add(Rectangle::new(border_thickness, extent + border_thickness));
    let horizontal_border = meshes.add(Rectangle::new(extent + border_thickness, border_thickness));
    for x in [-half, half] {
        commands.spawn((
            Mesh2d(vertical_border.clone()),
            MeshMaterial2d(border_material.clone()),
            Transform::from_xyz(x, 0.0, border_z),
        ));
    }
    for y in [-half, half] {
        commands.spawn((
            Mesh2d(horizontal_border.clone()),
            MeshMaterial2d(border_material.clone()),
            Transform::from_xyz(0.0, y, border_z),
        ));
    }
}
