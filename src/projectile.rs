use crate::{
    constants,
    evolution::EvolutionState,
    hud::UpgradeState,
    player::{self, Player, Velocity},
    rng::Rng,
};
use bevy::prelude::*;

#[derive(Component)]
pub struct Projectile;

#[derive(Component)]
pub struct Lifetime(pub f32);

#[derive(Component)]
pub struct ShootCooldown(pub f32);

#[derive(Component)]
pub struct ProjectileDamage(pub u32);

#[derive(Component)]
pub struct ProjectilePenetration(pub u32);

#[derive(Component)]
pub struct ProjectileKnockback(pub f32);

pub fn shoot_projectile(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    upgrades: Res<UpgradeState>,
    evolution: Res<EvolutionState>,
    mut rng: ResMut<Rng>,
    mut player_query: Query<(&Transform, &mut ShootCooldown), With<Player>>,
) {
    let Ok((transform, mut cooldown)) = player_query.single_mut() else {
        return;
    };

    cooldown.0 -= time.delta_secs();
    if cooldown.0 > 0.0 {
        return;
    }

    if !mouse.pressed(MouseButton::Left) {
        return;
    }

    cooldown.0 = upgrades.reload_cooldown() * evolution.reload_multiplier();

    let projectile_mesh = meshes.add(Circle::new(constants::PROJECTILE_RADIUS));
    let projectile_material = materials.add(Color::srgba(
        constants::PROJECTILE_COLOR[0],
        constants::PROJECTILE_COLOR[1],
        constants::PROJECTILE_COLOR[2],
        constants::PROJECTILE_COLOR[3],
    ));
    let spread = evolution.spread_radians();
    let base_damage = upgrades.bullet_damage() as f32 * evolution.bullet_damage_multiplier();
    let bullet_speed = upgrades.bullet_speed() * evolution.bullet_speed_multiplier();
    let lifetime = constants::PROJECTILE_LIFETIME * evolution.projectile_lifetime_multiplier();
    let knockback = evolution.bullet_knockback_multiplier();

    for spec in evolution.barrel_specs() {
        let jitter = if spread > 0.0 {
            let roll = rng.next(10_000) as f32 / 9_999.0;
            (roll * 2.0 - 1.0) * spread
        } else {
            0.0
        };
        let barrel_rotation = Quat::from_rotation_z(spec.angle_offset);
        let shot_rotation = Quat::from_rotation_z(spec.angle_offset + jitter);
        let forward = transform.rotation * barrel_rotation * Vec3::Y;
        let right = transform.rotation * barrel_rotation * Vec3::X;
        let direction = transform.rotation * shot_rotation * Vec3::Y;
        let spawn_pos = transform.translation
            + forward * player::muzzle_projectile_distance(spec.length)
            + right * spec.lateral_offset;
        let damage = (base_damage * spec.damage_multiplier).round().max(1.0) as u32;

        commands.spawn((
            Projectile,
            Lifetime(lifetime),
            ProjectileDamage(damage),
            ProjectilePenetration(upgrades.bullet_penetration()),
            ProjectileKnockback(knockback),
            Mesh2d(projectile_mesh.clone()),
            MeshMaterial2d(projectile_material.clone()),
            Transform::from_translation(spawn_pos),
            Velocity(direction.xy() * bullet_speed),
        ));
    }
}

pub fn projectile_update(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &Velocity, &mut Lifetime), With<Projectile>>,
) {
    for (entity, mut transform, velocity, mut lifetime) in query.iter_mut() {
        let dt = time.delta_secs();
        lifetime.0 -= dt;
        if lifetime.0 <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation += velocity.0.extend(0.0) * dt;
        let half = constants::arena_half_extent() + constants::PROJECTILE_RADIUS;
        if transform.translation.x.abs() > half || transform.translation.y.abs() > half {
            commands.entity(entity).despawn();
        }
    }
}
