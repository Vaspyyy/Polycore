use crate::{
    combat::CombatStats,
    constants,
    evolution::{self, BarrelSpec, EvolutionState},
    hud::UpgradeState,
    projectile::ShootCooldown,
    tank::{RecentDamage, SpawnProtection, TankOutline},
};
use bevy::prelude::*;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Barrel {
    pub owner: Entity,
    pub slot: usize,
    pub outline: bool,
}

#[derive(Component)]
pub struct PlayerNameLabel;

#[derive(Component)]
pub struct PlayerHealth {
    pub current: f32,
    pub max: f32,
}

#[derive(Component)]
pub struct DamageCooldown(pub f32);

#[derive(Component)]
pub struct HealthBarBack;

#[derive(Component)]
pub struct HealthBarFill;

#[derive(Component, Default)]
pub struct Velocity(pub Vec2);

#[derive(Component, Default)]
pub struct MoveVelocity(pub Vec2);

const BARREL_OVERLAP: f32 = 2.0;

#[derive(Clone, Copy)]
pub enum TankIconPartShape {
    Circle { diameter: f32 },
    Rectangle { width: f32, height: f32 },
}

#[derive(Clone, Copy)]
pub struct TankIconPart {
    pub shape: TankIconPartShape,
    pub offset: Vec2,
    pub rotation: f32,
    pub color: [f32; 4],
}

pub fn tank_icon_parts() -> Vec<TankIconPart> {
    vec![
        TankIconPart {
            shape: TankIconPartShape::Rectangle {
                width: 10.0,
                height: 46.0,
            },
            offset: Vec2::new(22.0, 22.0),
            rotation: -0.75,
            color: constants::BARREL_COLOR,
        },
        TankIconPart {
            shape: TankIconPartShape::Circle { diameter: 54.0 },
            offset: Vec2::ZERO,
            rotation: 0.0,
            color: constants::PLAYER_COLOR,
        },
    ]
}

fn barrel_center_distance(length: f32, radius: f32) -> f32 {
    radius - BARREL_OVERLAP + length / 2.0
}

pub fn muzzle_projectile_distance(
    length: f32,
    evolution: &EvolutionState,
    projectile_radius: f32,
) -> f32 {
    crate::tank::radius(evolution) - BARREL_OVERLAP + length + projectile_radius
}

pub fn barrel_local_axes(angle_offset: f32) -> (Vec2, Vec2) {
    (
        Vec2::new(-angle_offset.sin(), angle_offset.cos()),
        Vec2::new(angle_offset.cos(), angle_offset.sin()),
    )
}

fn barrel_transform(spec: BarrelSpec, outline: bool, evolution: &EvolutionState) -> Transform {
    let (forward, right) = barrel_local_axes(spec.angle_offset);
    let center = forward * barrel_center_distance(spec.length, crate::tank::radius(evolution))
        + right * spec.lateral_offset;
    let outline_growth = if outline {
        constants::OUTLINE_THICKNESS * 2.0
    } else {
        0.0
    };

    Transform {
        translation: Vec3::new(
            center.x,
            center.y,
            if outline {
                crate::tank::BARREL_OUTLINE_Z_OFFSET
            } else {
                crate::tank::BARREL_FILL_Z_OFFSET
            },
        ),
        rotation: Quat::from_rotation_z(spec.angle_offset),
        scale: Vec3::new(
            spec.width + outline_growth,
            spec.length + outline_growth,
            1.0,
        ),
    }
}

pub fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    palette_materials: Res<crate::palette::PaletteMaterials>,
    profile: Res<crate::profile::Profile>,
) {
    let selected = palette_materials.player(profile.data.selected_palette);
    let outline_material = materials.add(Color::srgba(
        constants::OUTLINE_COLOR[0],
        constants::OUTLINE_COLOR[1],
        constants::OUTLINE_COLOR[2],
        constants::OUTLINE_COLOR[3],
    ));

    let player_entity = commands
        .spawn((
            Player,
            PlayerHealth {
                current: constants::PLAYER_MAX_HEALTH,
                max: constants::PLAYER_MAX_HEALTH,
            },
            DamageCooldown(0.0),
            Velocity::default(),
            MoveVelocity::default(),
            CombatStats::default(),
            crate::combat::LifeGeneration::default(),
            ShootCooldown(0.0),
            SpawnProtection::default(),
            RecentDamage::default(),
            crate::passive::PassiveRuntime::default(),
            Mesh2d(meshes.add(Circle::new(constants::PLAYER_RADIUS))),
            MeshMaterial2d(selected.body.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Visibility::Hidden,
        ))
        .insert((
            crate::ability::ActiveAbilityState::default(),
            crate::ability::Slowed::default(),
        ))
        .id();

    commands.spawn((
        TankOutline {
            owner: player_entity,
        },
        Mesh2d(meshes.add(Circle::new(
            constants::PLAYER_RADIUS + constants::OUTLINE_THICKNESS,
        ))),
        MeshMaterial2d(outline_material.clone()),
        Transform::from_xyz(0.0, 0.0, -0.2),
        Visibility::Hidden,
    ));

    let barrel_mesh = meshes.add(Rectangle::new(1.0, 1.0));
    let default_evolution = EvolutionState::default();
    let specs = default_evolution.barrel_specs();
    let owner_transform = Transform::default();
    for slot in 0..evolution::MAX_BARRELS {
        let spec = specs.get(slot).copied().unwrap_or(specs[0]);
        for (outline, material) in [
            (true, outline_material.clone()),
            (false, selected.barrel.clone()),
        ] {
            commands.spawn((
                Barrel {
                    owner: player_entity,
                    slot,
                    outline,
                },
                Mesh2d(barrel_mesh.clone()),
                MeshMaterial2d(material),
                owner_transform.mul_transform(barrel_transform(spec, outline, &default_evolution)),
                Visibility::Hidden,
            ));
        }
    }

    let display_name = if profile.data.identity.player_name.trim().is_empty() {
        "Player"
    } else {
        profile.data.identity.player_name.as_str()
    };
    commands.spawn((
        PlayerNameLabel,
        Text2d::new(display_name),
        TextFont {
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(Color::WHITE),
        TextShadow {
            offset: Vec2::new(1.5, 1.5),
            color: Color::BLACK,
        },
        Transform::from_xyz(0.0, constants::PLAYER_RADIUS + 18.0, 4.0),
        Visibility::Hidden,
    ));

    commands.spawn((
        HealthBarBack,
        Mesh2d(meshes.add(Rectangle::new(
            constants::HEALTH_BAR_WIDTH,
            constants::HEALTH_BAR_HEIGHT,
        ))),
        MeshMaterial2d(materials.add(Color::srgba(
            constants::HEALTH_BAR_BG_COLOR[0],
            constants::HEALTH_BAR_BG_COLOR[1],
            constants::HEALTH_BAR_BG_COLOR[2],
            constants::HEALTH_BAR_BG_COLOR[3],
        ))),
        Transform::from_xyz(0.0, constants::HEALTH_BAR_OFFSET_Y, 2.0),
        Visibility::Hidden,
    ));

    commands.spawn((
        HealthBarFill,
        Mesh2d(meshes.add(Rectangle::new(
            constants::HEALTH_BAR_WIDTH,
            constants::HEALTH_BAR_HEIGHT,
        ))),
        MeshMaterial2d(materials.add(Color::srgba(
            constants::HEALTH_BAR_FILL_COLOR[0],
            constants::HEALTH_BAR_FILL_COLOR[1],
            constants::HEALTH_BAR_FILL_COLOR[2],
            constants::HEALTH_BAR_FILL_COLOR[3],
        ))),
        Transform::from_xyz(0.0, constants::HEALTH_BAR_OFFSET_Y, 3.0),
        Visibility::Hidden,
    ));
}

pub fn sync_player_palette(
    profile: Res<crate::profile::Profile>,
    palettes: Res<crate::palette::PaletteMaterials>,
    mut player: Query<&mut MeshMaterial2d<ColorMaterial>, (With<Player>, Without<Barrel>)>,
    mut barrels: Query<
        (&Barrel, &mut MeshMaterial2d<ColorMaterial>),
        (Without<Player>, Without<crate::tank::TankOutline>),
    >,
    mut last_palette: Local<Option<crate::palette::PaletteId>>,
) {
    let palette = profile.data.selected_palette;
    if *last_palette == Some(palette) {
        return;
    }
    *last_palette = Some(palette);
    let selected = palettes.player(palette);
    if let Ok(mut material) = player.single_mut() {
        material.0 = selected.body.clone();
    }
    for (barrel, mut material) in &mut barrels {
        if !barrel.outline {
            material.0 = selected.barrel.clone();
        }
    }
}

pub fn player_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    upgrades: Res<UpgradeState>,
    evolution: Res<EvolutionState>,
    mut query: Query<
        (
            &mut Transform,
            &mut Velocity,
            &mut MoveVelocity,
            &mut DamageCooldown,
            &crate::passive::PassiveRuntime,
            &crate::ability::ActiveAbilityState,
            &crate::ability::Slowed,
        ),
        With<Player>,
    >,
) {
    let Ok((
        mut transform,
        mut velocity,
        mut move_velocity,
        mut damage_cooldown,
        passive_runtime,
        ability_state,
        slowed,
    )) = query.single_mut()
    else {
        return;
    };

    let mut direction = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    let direction = if ability_state.forces_forward_movement() {
        (transform.rotation * Vec3::Y).xy().normalize_or_zero()
    } else {
        direction.normalize_or_zero()
    };
    let dt = time.delta_secs();
    let movement_speed = upgrades.movement_speed()
        * evolution.movement_multiplier()
        * crate::passive::movement_multiplier(passive_runtime, evolution.current_kind)
        * ability_state.movement_multiplier()
        * slowed.movement_multiplier();
    let target_velocity = direction * movement_speed;
    let acceleration = movement_speed / constants::PLAYER_ACCEL_TIME;
    move_velocity.0 = approach_velocity(move_velocity.0, target_velocity, acceleration * dt);
    transform.translation += (move_velocity.0 + velocity.0).extend(0.0) * dt;

    let damping = (1.0 - constants::PLAYER_KNOCKBACK_DAMPING * dt).clamp(0.0, 1.0);
    velocity.0 *= damping;
    damage_cooldown.0 = (damage_cooldown.0 - dt).max(0.0);

    let half = constants::arena_half_extent() - crate::tank::radius(&evolution);
    transform.translation.x = transform.translation.x.clamp(-half, half);
    transform.translation.y = transform.translation.y.clamp(-half, half);
}

pub fn update_player_upgrade_stats(
    upgrades: Res<UpgradeState>,
    evolution: Res<EvolutionState>,
    mut player: Query<&mut PlayerHealth, With<Player>>,
) {
    if !(upgrades.is_changed() || evolution.is_changed()) {
        return;
    }

    let Ok(mut health) = player.single_mut() else {
        return;
    };
    let upgraded_max = (upgrades.max_health() + evolution.max_health_bonus() as f32).max(40.0);
    if health.max == upgraded_max {
        return;
    }

    let was_alive = health.current > 0.0;
    let missing_health = (health.max - health.current).max(0.0);
    health.max = upgraded_max;
    health.current = adjusted_health_after_max_change(upgraded_max, missing_health, was_alive);
}

fn adjusted_health_after_max_change(max_health: f32, missing_health: f32, was_alive: bool) -> f32 {
    let adjusted = (max_health - missing_health).max(0.0);
    if was_alive {
        adjusted.max(1.0).min(max_health)
    } else {
        0.0
    }
}

pub fn regenerate_player_health(
    time: Res<Time>,
    upgrades: Res<UpgradeState>,
    evolution: Res<EvolutionState>,
    mut heal_progress: Local<f32>,
    mut player: Query<&mut PlayerHealth, With<Player>>,
) {
    let regen_per_second = upgrades.health_regen_per_second() + evolution.health_regen_bonus();
    if regen_per_second <= 0.0 {
        *heal_progress = 0.0;
        return;
    }

    let Ok(mut health) = player.single_mut() else {
        return;
    };
    if health.current >= health.max {
        *heal_progress = 0.0;
        return;
    }

    *heal_progress += regen_per_second * time.delta_secs();
    let heal_amount = heal_progress.floor();
    if heal_amount <= 0.0 {
        return;
    }

    health.current = (health.current + heal_amount).min(health.max);
    *heal_progress -= heal_amount;
}

fn approach_velocity(current: Vec2, target: Vec2, max_delta: f32) -> Vec2 {
    let delta = target - current;
    if delta.length_squared() <= max_delta * max_delta {
        target
    } else {
        current + delta.normalize_or_zero() * max_delta
    }
}

pub fn player_aim(
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &crate::ability::ActiveAbilityState), With<Player>>,
) {
    let Ok((mut transform, ability)) = query.single_mut() else {
        return;
    };

    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(world_pos) = camera.0.viewport_to_world_2d(camera.1, cursor) else {
        return;
    };

    let delta = world_pos - transform.translation.xy();
    if delta.length_squared() > 0.001 {
        let target = delta.y.atan2(delta.x) - std::f32::consts::FRAC_PI_2;
        if ability.limited_turning() {
            let (_, _, current) = transform.rotation.to_euler(EulerRot::XYZ);
            let turn = (target - current + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            transform.rotation = Quat::from_rotation_z(
                current + turn.clamp(-1.2 * time.delta_secs(), 1.2 * time.delta_secs()),
            );
        } else {
            transform.rotation = Quat::from_rotation_z(target);
        }
    }
}

pub fn update_barrel(
    evolution: Res<EvolutionState>,
    player: Query<&Transform, With<Player>>,
    mut barrels: Query<(&Barrel, &mut Transform, &mut Visibility), Without<Player>>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let specs = evolution.barrel_specs();
    for (barrel, mut transform, mut visibility) in barrels.iter_mut() {
        if player.get(barrel.owner).is_err() {
            *visibility = Visibility::Hidden;
            continue;
        }
        let Some(spec) = specs.get(barrel.slot).copied() else {
            *visibility = Visibility::Hidden;
            continue;
        };

        *visibility = Visibility::Visible;
        *transform =
            player_transform.mul_transform(barrel_transform(spec, barrel.outline, &evolution));
    }
}

pub fn sync_player_name_label(
    phase: Res<crate::menu::GamePhase>,
    player_name: Res<crate::menu::PlayerName>,
    evolution: Res<EvolutionState>,
    player: Query<(&Transform, &PlayerHealth), With<Player>>,
    mut labels: Query<
        (&mut Text2d, &mut Transform, &mut Visibility),
        (With<PlayerNameLabel>, Without<Player>),
    >,
) {
    let Ok((player_transform, health)) = player.single() else {
        return;
    };
    let visible = matches!(
        *phase,
        crate::menu::GamePhase::Playing | crate::menu::GamePhase::Paused
    ) && health.current > 0.0;
    let display_name = if player_name.0.trim().is_empty() {
        "Player"
    } else {
        player_name.0.as_str()
    };
    for (mut text, mut transform, mut visibility) in &mut labels {
        if text.as_str() != display_name {
            **text = display_name.to_string();
        }
        transform.translation = Vec3::new(
            player_transform.translation.x,
            player_transform.translation.y + crate::tank::radius(&evolution) + 18.0,
            4.0,
        );
        transform.rotation = Quat::IDENTITY;
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn update_health_bar(
    evolution: Res<EvolutionState>,
    player: Query<(&Transform, &PlayerHealth), With<Player>>,
    mut back: Query<
        (&mut Transform, &mut Visibility),
        (With<HealthBarBack>, Without<Player>, Without<HealthBarFill>),
    >,
    mut fill: Query<
        (&mut Transform, &mut Visibility),
        (With<HealthBarFill>, Without<Player>, Without<HealthBarBack>),
    >,
) {
    let Ok((player_transform, health)) = player.single() else {
        return;
    };
    let Ok((mut back_transform, mut back_visibility)) = back.single_mut() else {
        return;
    };
    let Ok((mut fill_transform, mut fill_visibility)) = fill.single_mut() else {
        return;
    };

    let is_damaged = health.current < health.max;
    *back_visibility = if is_damaged {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    *fill_visibility = if is_damaged {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    let health_fraction = (health.current / health.max).clamp(0.0, 1.0);
    let bar_position = player_transform.translation
        + Vec3::new(0.0, crate::tank::health_bar_offset(&evolution), 0.0);

    back_transform.translation = Vec3::new(bar_position.x, bar_position.y, 2.0);
    back_transform.rotation = Quat::IDENTITY;

    fill_transform.translation = Vec3::new(
        bar_position.x - constants::HEALTH_BAR_WIDTH * (1.0 - health_fraction) / 2.0,
        bar_position.y,
        3.0,
    );
    fill_transform.scale.x = health_fraction;
    fill_transform.rotation = Quat::IDENTITY;
}

pub fn hide_health_bars_when_not_playing(
    phase: Res<crate::menu::GamePhase>,
    mut bars: Query<&mut Visibility, Or<(With<HealthBarBack>, With<HealthBarFill>)>>,
) {
    if phase.is_changed()
        && matches!(
            *phase,
            crate::menu::GamePhase::Menu | crate::menu::GamePhase::Dead
        )
    {
        for mut visibility in &mut bars {
            *visibility = Visibility::Hidden;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_velocity_reaches_max_speed_after_one_second() {
        let acceleration = constants::PLAYER_SPEED / constants::PLAYER_ACCEL_TIME;
        let mut velocity = Vec2::ZERO;
        let target = Vec2::X * constants::PLAYER_SPEED;

        for _ in 0..60 {
            velocity = approach_velocity(velocity, target, acceleration / 60.0);
        }

        assert!((velocity.length() - constants::PLAYER_SPEED).abs() < 0.01);
    }

    #[test]
    fn projectile_spawn_distance_is_past_barrel_tip() {
        let barrel_length = EvolutionState::default().barrel_specs()[0].length;
        let barrel_tip_distance = constants::PLAYER_RADIUS - BARREL_OVERLAP + barrel_length;

        assert!(
            muzzle_projectile_distance(
                barrel_length,
                &EvolutionState::default(),
                constants::PROJECTILE_RADIUS,
            ) > barrel_tip_distance
        );
    }

    #[test]
    fn max_health_reduction_keeps_a_living_player_alive() {
        assert_eq!(adjusted_health_after_max_change(40.0, 45.0, true), 1.0);
        assert_eq!(adjusted_health_after_max_change(40.0, 45.0, false), 0.0);
    }

    #[test]
    fn barrel_world_transform_follows_player_owner() {
        let evolution = EvolutionState::default();
        let spec = evolution.barrel_specs()[0];
        let owner = Transform::from_xyz(120.0, -45.0, 0.0)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        let local = barrel_transform(spec, false, &evolution);
        let outline_local = barrel_transform(spec, true, &evolution);
        let world = owner.mul_transform(local);

        assert!(
            world
                .translation
                .truncate()
                .distance(owner.transform_point(local.translation).truncate())
                < 0.001
        );
        assert!(
            world
                .translation
                .truncate()
                .distance(owner.translation.truncate())
                > constants::PLAYER_RADIUS
        );
        assert!(outline_local.translation.z < local.translation.z);
        assert!(local.translation.z < crate::tank::TANK_OUTLINE_Z_OFFSET);
    }

    #[test]
    fn player_visual_system_queries_are_disjoint() {
        let mut app = App::new();
        app.insert_resource(EvolutionState::default())
            .insert_resource(crate::menu::GamePhase::Playing)
            .insert_resource(crate::menu::PlayerName("Tester".to_string()))
            .add_systems(Update, (update_barrel, sync_player_name_label));

        let player = app
            .world_mut()
            .spawn((
                Player,
                PlayerHealth {
                    current: 50.0,
                    max: 50.0,
                },
                Transform::default(),
            ))
            .id();
        app.world_mut().spawn((
            Barrel {
                owner: player,
                slot: 0,
                outline: false,
            },
            Transform::default(),
            Visibility::Hidden,
        ));
        app.world_mut().spawn((
            PlayerNameLabel,
            Text2d::default(),
            Transform::default(),
            Visibility::Hidden,
        ));

        app.update();
    }
}
