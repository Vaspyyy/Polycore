mod ability;
mod collision;
mod combat;
mod constants;
mod dominance;
mod enemy_bot;
mod enemy_bot_ai;
mod evolution;
mod experience;
mod feedback;
mod hotspot;
mod hud;
mod leaderboard;
mod menu;
mod palette;
mod passive;
mod performance;
mod player;
mod profile;
mod profile_ui;
mod projectile;
mod rng;
mod shape;
mod spatial;
mod tank;

use bevy::{
    asset::RenderAssetUsages,
    input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
use std::time::Duration;

#[derive(Component)]
struct ZoomVignette;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SimulationSet {
    RespawnTimers,
    AiDecisions,
    Movement,
    GridRebuild,
    CollisionDamage,
    DeathResolution,
    Progression,
}

fn main() {
    let profile = profile::Profile::load();
    let player_name = menu::PlayerName(profile.data.identity.player_name.clone());
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
        .insert_resource(Time::<Virtual>::from_max_delta(Duration::from_millis(50)))
        .insert_resource(rng::Rng::from_entropy())
        .insert_resource(spatial::SpatialGrid::default())
        .insert_resource(menu::GamePhase::Menu)
        .insert_resource(menu::GameMode::Singleplayer)
        .insert_resource(profile)
        .insert_resource(player_name)
        .insert_resource(menu::NameInputFocus::default())
        .insert_resource(menu::RunStats::default())
        .insert_resource(menu::DeathSummary::default())
        .insert_resource(combat::CombatDeathQueue::default())
        .insert_resource(enemy_bot::EnemyBotResetPending::default())
        .insert_resource(hud::UpgradeState::default())
        .insert_resource(evolution::EvolutionState::default())
        .insert_resource(dominance::DominanceState::default())
        .insert_resource(dominance::LeaderChallengeDirector::default())
        .insert_resource(dominance::PlayerShapeKills::default())
        .insert_resource(dominance::ProfileProgressTracker::default())
        .insert_resource(feedback::FeedbackTracker::default())
        .insert_resource(feedback::CameraShake::default())
        .insert_resource(performance::PerformanceTelemetry::default())
        .add_message::<feedback::CombatFeedback>()
        .add_message::<dominance::AchievementUnlocked>()
        .add_message::<ability::AbilityCast>()
        .configure_sets(
            FixedUpdate,
            (
                SimulationSet::RespawnTimers,
                SimulationSet::AiDecisions,
                SimulationSet::Movement,
                SimulationSet::GridRebuild,
                SimulationSet::CollisionDamage,
                SimulationSet::DeathResolution,
                SimulationSet::Progression,
            )
                .chain(),
        )
        .add_systems(
            Startup,
            (
                setup_camera,
                setup_grid,
                palette::setup_palette_materials,
                tank::setup_tank_assets,
                projectile::setup_projectile_assets,
                ability::setup_abilities,
                shape::setup_shape_assets,
                hotspot::setup_hotspot,
                player::setup_player,
                enemy_bot::setup_enemy_bots,
                shape::setup_xp,
                hud::setup_hud,
                leaderboard::setup_leaderboard,
                evolution::setup_evolution_menu,
                menu::setup_menu,
                dominance::setup_dominance_ui,
                profile_ui::setup_profile_panel,
                experience::setup_experience_ui,
                feedback::setup_feedback,
                performance::setup_performance_overlay,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                ability::player_ability_input,
                ability::bot_ability_decisions,
                ability::execute_ability_casts,
                ability::tick_abilities,
                ability::ensure_ability_rings,
                ability::update_ability_presentation,
            )
                .chain()
                .run_if(menu::is_simulating),
        )
        .add_systems(
            Update,
            (
                performance::toggle_performance_overlay,
                performance::sample_performance,
                performance::flush_performance_log,
            )
                .chain(),
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
                enemy_bot::sync_enemy_bot_name_labels,
                leaderboard::sync_leaderboard_visibility,
                leaderboard::handle_leaderboard_toggle,
                leaderboard::animate_leaderboard,
                hide_zoom_vignette,
                menu::sync_death_summary,
                evolution::queue_evolution_choices,
                evolution::update_evolution_menu,
                evolution::handle_evolution_buttons,
                evolution::update_evolution_hover_description,
                hud::update_upgrade_menu,
            ),
        )
        .add_systems(
            Update,
            (
                feedback::ensure_passive_visuals,
                feedback::update_passive_visuals,
                feedback::update_passive_status,
                feedback::detect_feedback,
                feedback::consume_feedback,
                feedback::update_feedback_effects,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                profile::flush_profile,
                player::sync_player_palette,
                player::sync_player_name_label,
                enemy_bot::update_enemy_bot_health_bars,
                profile_ui::handle_palette_buttons,
                profile_ui::update_profile_panel,
                hotspot::update_hotspot_presentation,
                evolution::refresh_evolution_cards,
                experience::handle_pause_input,
                experience::handle_settings_buttons,
                experience::sync_pause_visibility,
                experience::update_settings_labels,
                experience::apply_window_settings,
                apply_low_power_visuals,
            ),
        )
        .add_systems(
            Update,
            (
                dominance::update_dominance,
                dominance::update_profile_progress,
                dominance::update_leader_indicator,
                dominance::show_achievement_toasts,
                dominance::tick_achievement_toasts,
                dominance::flush_crown_progress_on_exit,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                menu::tick_run_stats.run_if(menu::is_playing),
                player::hide_health_bars_when_not_playing,
            ),
        )
        .add_systems(
            Update,
            (
                camera_zoom,
                player::player_aim,
                player::update_barrel,
                player::update_player_upgrade_stats,
                player::regenerate_player_health,
                player::update_health_bar,
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
            Update,
            tank::update_tank_bodies
                .after(menu::sync_phase_visibility)
                .after(enemy_bot::sync_enemy_bot_visibility),
        )
        .add_systems(
            Update,
            (
                shape::update_shape_health_bars,
                leaderboard::update_leaderboard,
            )
                .run_if(menu::is_simulating),
        )
        .add_systems(FixedUpdate, performance::count_fixed_step)
        .add_systems(
            FixedUpdate,
            (
                tank::tick_protection_and_damage,
                passive::tick_passives,
                enemy_bot_ai::respawn_enemy_bots,
            )
                .in_set(SimulationSet::RespawnTimers)
                .run_if(menu::is_simulating),
        )
        .add_systems(
            FixedUpdate,
            (
                dominance::update_challenge_director,
                enemy_bot_ai::enemy_bot_ai_update,
            )
                .chain()
                .in_set(SimulationSet::AiDecisions)
                .run_if(menu::is_simulating),
        )
        .add_systems(
            FixedUpdate,
            player::player_movement
                .in_set(SimulationSet::Movement)
                .run_if(menu::is_playing),
        )
        .add_systems(
            FixedUpdate,
            (
                shape::shape_knockback_update,
                projectile::projectile_update,
                ability::update_constructs,
            )
                .in_set(SimulationSet::Movement)
                .run_if(menu::is_simulating),
        )
        .add_systems(
            FixedUpdate,
            spatial::rebuild_spatial_grid
                .in_set(SimulationSet::GridRebuild)
                .run_if(menu::is_simulating),
        )
        .add_systems(
            FixedUpdate,
            (
                collision::check_enemy_bot_enemy_bot_collisions,
                ability::resolve_projectile_manipulation,
                collision::check_collisions,
                collision::check_projectile_enemy_bot_collisions,
                collision::check_projectile_player_collisions,
                collision::resolve_pending_shape_splashes,
                collision::resolve_pending_splashes,
                collision::check_player_enemy_bot_collisions,
                collision::check_player_shape_collisions,
                collision::check_enemy_bot_shape_collisions,
                collision::check_shape_shape_collisions,
                ability::resolve_construct_collisions,
            )
                .chain()
                .in_set(SimulationSet::CollisionDamage)
                .run_if(menu::is_simulating),
        )
        .add_systems(
            FixedUpdate,
            combat::resolve_combat_deaths
                .in_set(SimulationSet::DeathResolution)
                .run_if(menu::is_simulating),
        )
        .add_systems(
            FixedUpdate,
            (
                hotspot::update_hotspot,
                shape::shape_spawn,
                shape::check_level_up,
            )
                .in_set(SimulationSet::Progression)
                .run_if(menu::is_simulating),
        )
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Msaa::Sample4));
    commands.spawn((
        ZoomVignette,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        zoom_vignette_gradient(0.0),
        Pickable::IGNORE,
        GlobalZIndex(-10),
        Visibility::Hidden,
    ));
}

fn camera_follow(
    window: Single<&Window>,
    time: Res<Time>,
    profile: Res<profile::Profile>,
    mut shake: ResMut<feedback::CameraShake>,
    player: Query<&Transform, With<player::Player>>,
    mut camera: Query<(&mut Transform, &Projection), (With<Camera>, Without<player::Player>)>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let Ok((mut camera_transform, projection)) = camera.single_mut() else {
        return;
    };
    let zoom = match projection {
        Projection::Orthographic(orthographic) => orthographic.scale,
        _ => 1.0,
    };
    let half = constants::arena_half_extent();
    let max_camera_x = (half - window.width() * zoom / 2.0).max(0.0);
    let max_camera_y = (half - window.height() * zoom / 2.0).max(0.0);

    camera_transform.translation.x = player_transform
        .translation
        .x
        .clamp(-max_camera_x, max_camera_x);
    camera_transform.translation.y = player_transform
        .translation
        .y
        .clamp(-max_camera_y, max_camera_y);
    let offset = feedback::camera_shake_offset(
        &mut shake,
        time.delta_secs(),
        profile.data.settings.screen_shake,
    );
    camera_transform.translation += offset.extend(0.0);
}

fn camera_zoom(
    scroll: Res<AccumulatedMouseScroll>,
    profile: Res<profile::Profile>,
    mut camera: Query<&mut Projection, With<Camera>>,
    mut vignette: Query<(&mut BackgroundGradient, &mut Visibility), With<ZoomVignette>>,
) {
    let scroll_delta = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR,
    };
    if scroll_delta.abs() <= f32::EPSILON {
        return;
    }

    let Ok(mut projection) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(orthographic) = &mut *projection else {
        return;
    };

    let zoom_delta = -scroll_delta * constants::CAMERA_ZOOM_SPEED;
    let resistance = if zoom_delta > 0.0 {
        1.0 - zoom_warning_strength(orthographic.scale) * 0.72
    } else {
        1.0
    };
    let zoom_factor = (zoom_delta * resistance).exp();
    orthographic.scale = (orthographic.scale * zoom_factor)
        .clamp(constants::CAMERA_MIN_ZOOM, constants::CAMERA_MAX_ZOOM);

    let strength = if profile.data.settings.low_power_mode {
        0.0
    } else {
        zoom_warning_strength(orthographic.scale)
    };
    for (mut gradient, mut visibility) in vignette.iter_mut() {
        *visibility = if strength > 0.001 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        *gradient = zoom_vignette_gradient(strength);
    }
}

fn zoom_warning_strength(scale: f32) -> f32 {
    let raw_strength = ((scale - constants::CAMERA_SOFT_MAX_ZOOM)
        / (constants::CAMERA_MAX_ZOOM - constants::CAMERA_SOFT_MAX_ZOOM))
        .clamp(0.0, 1.0);
    raw_strength * raw_strength * (3.0 - 2.0 * raw_strength)
}

fn hide_zoom_vignette(
    phase: Res<menu::GamePhase>,
    mut vignette: Query<&mut Visibility, With<ZoomVignette>>,
) {
    if !phase.is_changed() || *phase == menu::GamePhase::Playing {
        return;
    }
    for mut visibility in vignette.iter_mut() {
        *visibility = Visibility::Hidden;
    }
}

fn apply_low_power_visuals(
    profile: Res<profile::Profile>,
    mut cameras: Query<&mut Msaa, With<Camera2d>>,
    mut vignette: Query<&mut Visibility, With<ZoomVignette>>,
    mut last_low_power: Local<Option<bool>>,
) {
    let low_power = profile.data.settings.low_power_mode;
    if *last_low_power == Some(low_power) {
        return;
    }
    *last_low_power = Some(low_power);
    for mut msaa in &mut cameras {
        *msaa = if low_power { Msaa::Off } else { Msaa::Sample4 };
    }
    if low_power {
        for mut visibility in &mut vignette {
            *visibility = Visibility::Hidden;
        }
    }
}

fn zoom_vignette_gradient(strength: f32) -> BackgroundGradient {
    let edge_alpha = 0.68 * strength;
    BackgroundGradient::from(RadialGradient {
        position: UiPosition::CENTER,
        shape: RadialGradientShape::FarthestCorner,
        stops: vec![
            ColorStop::percent(Color::NONE, 52.0),
            ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, edge_alpha * 0.22), 72.0),
            ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, edge_alpha), 100.0),
        ],
        ..default()
    })
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
    let mut grid_rectangles = Vec::new();
    let mut x = -half;
    while x <= half {
        grid_rectangles.push((Vec2::new(x, 0.0), Vec2::new(thickness, extent)));
        x += spacing;
    }
    let mut y = -half;
    while y <= half {
        grid_rectangles.push((Vec2::new(0.0, y), Vec2::new(extent, thickness)));
        y += spacing;
    }
    commands.spawn((
        Mesh2d(meshes.add(rectangle_batch_mesh(&grid_rectangles))),
        MeshMaterial2d(grid_material),
        Transform::from_xyz(0.0, 0.0, grid_z),
    ));

    let border_thickness = constants::BORDER_THICKNESS;
    let mut border_rectangles = Vec::with_capacity(4);
    for x in [-half, half] {
        border_rectangles.push((
            Vec2::new(x, 0.0),
            Vec2::new(border_thickness, extent + border_thickness),
        ));
    }
    for y in [-half, half] {
        border_rectangles.push((
            Vec2::new(0.0, y),
            Vec2::new(extent + border_thickness, border_thickness),
        ));
    }
    commands.spawn((
        Mesh2d(meshes.add(rectangle_batch_mesh(&border_rectangles))),
        MeshMaterial2d(border_material),
        Transform::from_xyz(0.0, 0.0, border_z),
    ));
}

fn rectangle_batch_mesh(rectangles: &[(Vec2, Vec2)]) -> Mesh {
    let mut positions = Vec::with_capacity(rectangles.len() * 4);
    let mut indices = Vec::with_capacity(rectangles.len() * 6);
    for (center, size) in rectangles {
        let half = *size * 0.5;
        let base = positions.len() as u32;
        positions.extend([
            [center.x - half.x, center.y - half.y, 0.0],
            [center.x + half.x, center.y - half.y, 0.0],
            [center.x + half.x, center.y + half.y, 0.0],
            [center.x - half.x, center.y + half.y, 0.0],
        ]);
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(Indices::U32(indices))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_systems_initialize_without_query_aliases() {
        let mut world = World::new();

        let mut update = Schedule::default();
        update.add_systems((
            ability::player_ability_input,
            ability::bot_ability_decisions,
            ability::execute_ability_casts,
            ability::tick_abilities,
            ability::ensure_ability_rings,
            ability::update_ability_presentation,
            hotspot::update_hotspot_presentation,
        ));
        update.initialize(&mut world).unwrap();

        let mut fixed = Schedule::default();
        fixed.add_systems((
            ability::update_constructs,
            ability::resolve_projectile_manipulation,
            ability::resolve_construct_collisions,
            hotspot::update_hotspot,
        ));
        fixed.initialize(&mut world).unwrap();
    }

    #[test]
    fn arena_rectangles_are_batched_into_single_meshes() {
        let mesh = rectangle_batch_mesh(&[
            (Vec2::ZERO, Vec2::new(2.0, 10.0)),
            (Vec2::X, Vec2::new(10.0, 2.0)),
        ]);
        assert_eq!(mesh.count_vertices(), 8);
        assert_eq!(mesh.indices().map(Indices::len), Some(12));
    }

    #[test]
    fn virtual_time_caps_stall_recovery_at_three_fixed_steps() {
        let virtual_time = Time::<Virtual>::from_max_delta(Duration::from_millis(50));
        let fixed_time = Time::<Fixed>::default();
        assert_eq!(virtual_time.max_delta(), Duration::from_millis(50));
        assert_eq!(fixed_time.timestep(), Duration::from_micros(15_625));
        assert_eq!(
            virtual_time.max_delta().as_nanos() / fixed_time.timestep().as_nanos(),
            3
        );
    }

    #[test]
    fn low_power_switches_msaa_and_hides_vignette() {
        let mut world = World::new();
        let mut profile = profile::Profile::test_with_path(None);
        profile.data.settings.low_power_mode = true;
        world.insert_resource(profile);
        let camera = world.spawn((Camera2d, Msaa::Sample4)).id();
        let vignette = world.spawn((ZoomVignette, Visibility::Visible)).id();
        let mut schedule = Schedule::default();
        schedule.add_systems(apply_low_power_visuals);
        schedule.run(&mut world);
        assert_eq!(*world.get::<Msaa>(camera).unwrap(), Msaa::Off);
        assert_eq!(
            *world.get::<Visibility>(vignette).unwrap(),
            Visibility::Hidden
        );

        world
            .resource_mut::<profile::Profile>()
            .data
            .settings
            .low_power_mode = false;
        schedule.run(&mut world);
        assert_eq!(*world.get::<Msaa>(camera).unwrap(), Msaa::Sample4);
    }
}
