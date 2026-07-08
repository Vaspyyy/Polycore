use crate::{
    constants,
    player::{Player, Velocity},
};
use bevy::prelude::*;

#[derive(Component)]
pub struct Projectile;

#[derive(Component)]
pub struct Lifetime(pub f32);

#[derive(Component)]
pub struct ShootCooldown(pub f32);

pub fn shoot_projectile(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
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

    cooldown.0 = constants::SHOOT_COOLDOWN;

    let direction = transform.rotation * Vec3::Y;
    let spawn_pos = transform.translation
        + direction * (constants::PLAYER_RADIUS + constants::PROJECTILE_RADIUS + 4.0);

    commands.spawn((
        Projectile,
        Lifetime(constants::PROJECTILE_LIFETIME),
        Mesh2d(meshes.add(Circle::new(constants::PROJECTILE_RADIUS))),
        MeshMaterial2d(materials.add(Color::srgba(
            constants::PROJECTILE_COLOR[0],
            constants::PROJECTILE_COLOR[1],
            constants::PROJECTILE_COLOR[2],
            constants::PROJECTILE_COLOR[3],
        ))),
        Transform::from_translation(spawn_pos),
        Velocity(direction.xy() * constants::PROJECTILE_SPEED),
    ));
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
