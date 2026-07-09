use crate::{constants, hud::UpgradeState, projectile::ShootCooldown};
use bevy::prelude::*;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Barrel;

#[derive(Component)]
pub struct PlayerHealth {
    pub current: u32,
    pub max: u32,
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

const BARREL_LENGTH: f32 = 30.6;
const BARREL_WIDTH: f32 = 6.6;
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

fn barrel_center_distance() -> f32 {
    constants::PLAYER_RADIUS - BARREL_OVERLAP + BARREL_LENGTH / 2.0
}

pub fn muzzle_projectile_distance() -> f32 {
    constants::PLAYER_RADIUS - BARREL_OVERLAP + BARREL_LENGTH + constants::PROJECTILE_RADIUS
}

pub fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let outline_material = materials.add(Color::srgba(
        constants::OUTLINE_COLOR[0],
        constants::OUTLINE_COLOR[1],
        constants::OUTLINE_COLOR[2],
        constants::OUTLINE_COLOR[3],
    ));

    commands
        .spawn((
            Player,
            PlayerHealth {
                current: constants::PLAYER_MAX_HEALTH,
                max: constants::PLAYER_MAX_HEALTH,
            },
            DamageCooldown(0.0),
            Velocity::default(),
            MoveVelocity::default(),
            ShootCooldown(0.0),
            Mesh2d(meshes.add(Circle::new(constants::PLAYER_RADIUS))),
            MeshMaterial2d(materials.add(Color::srgba(
                constants::PLAYER_COLOR[0],
                constants::PLAYER_COLOR[1],
                constants::PLAYER_COLOR[2],
                constants::PLAYER_COLOR[3],
            ))),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Visibility::Hidden,
        ))
        .with_children(|player| {
            player.spawn((
                Mesh2d(meshes.add(Circle::new(
                    constants::PLAYER_RADIUS + constants::OUTLINE_THICKNESS,
                ))),
                MeshMaterial2d(outline_material.clone()),
                Transform::from_xyz(0.0, 0.0, -0.2),
            ));
            player.spawn((
                Mesh2d(meshes.add(Rectangle::new(
                    BARREL_WIDTH + constants::OUTLINE_THICKNESS * 2.0,
                    BARREL_LENGTH + constants::OUTLINE_THICKNESS * 2.0,
                ))),
                MeshMaterial2d(outline_material),
                Transform::from_xyz(0.0, barrel_center_distance(), -0.2),
            ));
        });

    // Turret barrel (rectangle indicating aim direction)
    commands.spawn((
        Barrel,
        Mesh2d(meshes.add(Rectangle::new(BARREL_WIDTH, BARREL_LENGTH))),
        MeshMaterial2d(materials.add(Color::srgba(
            constants::BARREL_COLOR[0],
            constants::BARREL_COLOR[1],
            constants::BARREL_COLOR[2],
            constants::BARREL_COLOR[3],
        ))),
        Transform::from_xyz(0.0, barrel_center_distance(), 1.0),
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

pub fn player_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    upgrades: Res<UpgradeState>,
    mut query: Query<
        (
            &mut Transform,
            &mut Velocity,
            &mut MoveVelocity,
            &mut DamageCooldown,
        ),
        With<Player>,
    >,
) {
    let Ok((mut transform, mut velocity, mut move_velocity, mut damage_cooldown)) =
        query.single_mut()
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

    let direction = direction.normalize_or_zero();
    let dt = time.delta_secs();
    let movement_speed = upgrades.movement_speed();
    let target_velocity = direction * movement_speed;
    let acceleration = movement_speed / constants::PLAYER_ACCEL_TIME;
    move_velocity.0 = approach_velocity(move_velocity.0, target_velocity, acceleration * dt);
    transform.translation += (move_velocity.0 + velocity.0).extend(0.0) * dt;

    let damping = (1.0 - constants::PLAYER_KNOCKBACK_DAMPING * dt).clamp(0.0, 1.0);
    velocity.0 *= damping;
    damage_cooldown.0 = (damage_cooldown.0 - dt).max(0.0);

    let half = constants::arena_half_extent() - constants::PLAYER_RADIUS;
    transform.translation.x = transform.translation.x.clamp(-half, half);
    transform.translation.y = transform.translation.y.clamp(-half, half);
}

pub fn update_player_upgrade_stats(
    upgrades: Res<UpgradeState>,
    mut player: Query<&mut PlayerHealth, With<Player>>,
) {
    if !upgrades.is_changed() {
        return;
    }

    let Ok(mut health) = player.single_mut() else {
        return;
    };
    let upgraded_max = upgrades.max_health();
    if health.max == upgraded_max {
        return;
    }

    let missing_health = health.max.saturating_sub(health.current);
    health.max = upgraded_max;
    health.current = upgraded_max.saturating_sub(missing_health);
}

pub fn regenerate_player_health(
    time: Res<Time>,
    upgrades: Res<UpgradeState>,
    mut heal_progress: Local<f32>,
    mut player: Query<&mut PlayerHealth, With<Player>>,
) {
    let regen_per_second = upgrades.health_regen_per_second();
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
    let heal_amount = heal_progress.floor() as u32;
    if heal_amount == 0 {
        return;
    }

    health.current = (health.current + heal_amount).min(health.max);
    *heal_progress -= heal_amount as f32;
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
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut transform) = query.single_mut() else {
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
        transform.rotation =
            Quat::from_rotation_z(delta.y.atan2(delta.x) - std::f32::consts::FRAC_PI_2);
    }
}

pub fn update_barrel(
    player: Query<&Transform, (With<Player>, Without<Barrel>)>,
    mut barrel: Query<&mut Transform, (With<Barrel>, Without<Player>)>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let Ok(mut barrel_transform) = barrel.single_mut() else {
        return;
    };

    let direction = player_transform.rotation * Vec3::Y;
    barrel_transform.translation =
        player_transform.translation + direction * barrel_center_distance();
    barrel_transform.translation.z = 1.0;
    barrel_transform.rotation = player_transform.rotation;
}

pub fn update_health_bar(
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

    let health_fraction = (health.current as f32 / health.max as f32).clamp(0.0, 1.0);
    let bar_position =
        player_transform.translation + Vec3::new(0.0, constants::HEALTH_BAR_OFFSET_Y, 0.0);

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
        let barrel_tip_distance = constants::PLAYER_RADIUS - BARREL_OVERLAP + BARREL_LENGTH;

        assert!(muzzle_projectile_distance() > barrel_tip_distance);
    }
}
