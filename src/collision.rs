use crate::{
    constants,
    menu::{DeathSummary, GamePhase, RunStats},
    player::{DamageCooldown, Player, PlayerHealth, Velocity},
    projectile::Projectile,
    shape::{
        Health, Shape, ShapeContactCooldown, ShapeDamage, ShapeKind, ShapeVelocity, TotalXp, Xp,
        XpValue,
    },
};
use bevy::prelude::*;

pub fn check_collisions(
    mut commands: Commands,
    projectiles: Query<(Entity, &Transform), With<Projectile>>,
    mut shapes: Query<
        (
            Entity,
            &Transform,
            &mut Health,
            &XpValue,
            &mut ShapeVelocity,
        ),
        With<Shape>,
    >,
    mut xp: ResMut<Xp>,
    mut total_xp: ResMut<TotalXp>,
) {
    let proj_data: Vec<(Entity, Vec2)> = projectiles
        .iter()
        .map(|(e, t)| (e, t.translation.xy()))
        .collect();

    let collision_dist = constants::PROJECTILE_RADIUS + constants::SHAPE_RADIUS;
    let collision_dist_sq = collision_dist * collision_dist;

    for (proj_entity, proj_pos) in &proj_data {
        for (shape_entity, shape_pos, mut health, xp_val, mut velocity) in shapes.iter_mut() {
            let dist_sq = proj_pos.distance_squared(shape_pos.translation.xy());
            if dist_sq < collision_dist_sq {
                commands.entity(*proj_entity).despawn();
                let knockback_dir = (shape_pos.translation.xy() - *proj_pos).normalize_or_zero();
                if health.0 == 0 {
                    commands.entity(shape_entity).despawn();
                    break;
                }

                velocity.0 += knockback_dir * constants::SHAPE_KNOCKBACK_SPEED;
                if apply_shape_damage(&mut health, 1) {
                    commands.entity(shape_entity).despawn();
                    xp.0 += xp_val.0;
                    total_xp.0 += xp_val.0;
                }
                break;
            }
        }
    }
}

pub fn check_player_shape_collisions(
    mut phase: ResMut<GamePhase>,
    run_stats: Res<RunStats>,
    total_xp: Res<TotalXp>,
    level: Res<crate::shape::Level>,
    mut death_summary: ResMut<DeathSummary>,
    mut player: Query<
        (
            &mut Transform,
            &mut Velocity,
            &mut PlayerHealth,
            &mut DamageCooldown,
        ),
        (With<Player>, Without<Shape>),
    >,
    mut shapes: Query<
        (&mut Transform, &mut ShapeVelocity, &ShapeDamage, &ShapeKind),
        (With<Shape>, Without<Player>),
    >,
) {
    let Ok((mut player_transform, mut player_velocity, mut player_health, mut damage_cooldown)) =
        player.single_mut()
    else {
        return;
    };
    let collision_distance = constants::PLAYER_RADIUS + constants::SHAPE_RADIUS;
    let collision_distance_sq = collision_distance * collision_distance;
    let player_half = constants::arena_half_extent() - constants::PLAYER_RADIUS;
    let shape_half = constants::arena_half_extent() - constants::SHAPE_RADIUS;

    for (mut shape_transform, mut shape_velocity, shape_damage, shape_kind) in shapes.iter_mut() {
        let player_pos = player_transform.translation.xy();
        let shape_pos = shape_transform.translation.xy();
        let delta = player_pos - shape_pos;
        let distance_sq = delta.length_squared();

        if distance_sq >= collision_distance_sq {
            continue;
        }

        let distance = distance_sq.sqrt();
        let normal = if distance > 0.001 {
            delta / distance
        } else {
            Vec2::X
        };
        let penetration = collision_distance - distance;

        player_transform.translation += (normal * penetration * 0.55).extend(0.0);
        shape_transform.translation -= (normal * penetration * 0.45).extend(0.0);

        player_transform.translation.x = player_transform
            .translation
            .x
            .clamp(-player_half, player_half);
        player_transform.translation.y = player_transform
            .translation
            .y
            .clamp(-player_half, player_half);
        shape_transform.translation.x =
            shape_transform.translation.x.clamp(-shape_half, shape_half);
        shape_transform.translation.y =
            shape_transform.translation.y.clamp(-shape_half, shape_half);

        player_velocity.0 += normal * constants::PLAYER_COLLISION_KNOCKBACK_SPEED;
        shape_velocity.0 -= normal * constants::SHAPE_COLLISION_KNOCKBACK_SPEED;

        if damage_cooldown.0 <= 0.0 {
            player_health.current = player_health.current.saturating_sub(shape_damage.0);
            damage_cooldown.0 = constants::PLAYER_DAMAGE_COOLDOWN;
            if player_health.current == 0 {
                death_summary.killed_by = shape_kind.name().to_string();
                death_summary.score = total_xp.0;
                death_summary.level = level.0;
                death_summary.time_alive = run_stats.time_alive;
                death_summary.tank_name = "Tank".to_string();
                *phase = GamePhase::Dead;
                break;
            }
        }
    }
}

pub fn check_shape_shape_collisions(
    mut commands: Commands,
    mut shapes: Query<
        (
            Entity,
            &mut Transform,
            &mut Health,
            &mut ShapeVelocity,
            &mut ShapeContactCooldown,
        ),
        With<Shape>,
    >,
) {
    let collision_distance = constants::SHAPE_RADIUS * 2.0;
    let collision_distance_sq = collision_distance * collision_distance;
    let shape_half = constants::arena_half_extent() - constants::SHAPE_RADIUS;
    let mut dead_shapes = Vec::new();

    let mut combinations = shapes.iter_combinations_mut::<2>();
    while let Some(
        [
            (entity_a, mut transform_a, mut health_a, mut velocity_a, mut cooldown_a),
            (entity_b, mut transform_b, mut health_b, mut velocity_b, mut cooldown_b),
        ],
    ) = combinations.fetch_next()
    {
        if health_a.0 == 0 || health_b.0 == 0 {
            if health_a.0 == 0 {
                push_dead_shape(&mut dead_shapes, entity_a);
            }
            if health_b.0 == 0 {
                push_dead_shape(&mut dead_shapes, entity_b);
            }
            continue;
        }

        let pos_a = transform_a.translation.xy();
        let pos_b = transform_b.translation.xy();
        let delta = pos_a - pos_b;
        let distance_sq = delta.length_squared();

        if distance_sq >= collision_distance_sq {
            continue;
        }

        let distance = distance_sq.sqrt();
        let normal = if distance > 0.001 {
            delta / distance
        } else {
            Vec2::X
        };
        let penetration = collision_distance - distance;

        transform_a.translation += (normal * penetration * 0.5).extend(0.0);
        transform_b.translation -= (normal * penetration * 0.5).extend(0.0);

        transform_a.translation.x = transform_a.translation.x.clamp(-shape_half, shape_half);
        transform_a.translation.y = transform_a.translation.y.clamp(-shape_half, shape_half);
        transform_b.translation.x = transform_b.translation.x.clamp(-shape_half, shape_half);
        transform_b.translation.y = transform_b.translation.y.clamp(-shape_half, shape_half);

        velocity_a.0 += normal * constants::SHAPE_SHAPE_KNOCKBACK_SPEED;
        velocity_b.0 -= normal * constants::SHAPE_SHAPE_KNOCKBACK_SPEED;

        if cooldown_a.0 <= 0.0 {
            if apply_shape_damage(&mut health_a, constants::SHAPE_SHAPE_DAMAGE) {
                push_dead_shape(&mut dead_shapes, entity_a);
            }
            cooldown_a.0 = constants::SHAPE_SHAPE_DAMAGE_COOLDOWN;
        }
        if cooldown_b.0 <= 0.0 {
            if apply_shape_damage(&mut health_b, constants::SHAPE_SHAPE_DAMAGE) {
                push_dead_shape(&mut dead_shapes, entity_b);
            }
            cooldown_b.0 = constants::SHAPE_SHAPE_DAMAGE_COOLDOWN;
        }
    }

    for entity in dead_shapes {
        commands.entity(entity).despawn();
    }
}

fn apply_shape_damage(health: &mut Health, damage: u32) -> bool {
    let was_alive = health.0 > 0;
    health.0 = health.0.saturating_sub(damage);
    was_alive && health.0 == 0
}

fn push_dead_shape(dead_shapes: &mut Vec<Entity>, entity: Entity) {
    if !dead_shapes.contains(&entity) {
        dead_shapes.push(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_damage_saturates_without_double_kill() {
        let mut health = Health(1);

        assert!(apply_shape_damage(&mut health, 1));
        assert_eq!(health.0, 0);
        assert!(!apply_shape_damage(&mut health, 1));
        assert_eq!(health.0, 0);
    }
}
