use crate::{
    constants,
    enemy_bot::{
        EnemyBot, EnemyBotDamageCooldown, EnemyBotEvolution, EnemyBotHealth, EnemyBotLevel,
        EnemyBotName, EnemyBotUpgrades, EnemyBotVelocity, EnemyBotXp, apply_enemy_bot_damage,
        award_enemy_bot_xp,
    },
    evolution::EvolutionState,
    hud::UpgradeState,
    menu::{DeathSummary, GamePhase, RunStats},
    player::{DamageCooldown, Player, PlayerHealth, Velocity},
    projectile::{
        Projectile, ProjectileDamage, ProjectileKnockback, ProjectileOwner, ProjectilePenetration,
    },
    rng::Rng,
    shape::{
        Health, Shape, ShapeContactCooldown, ShapeDamage, ShapeKind, ShapeVelocity, TotalXp, Xp,
        XpValue,
    },
};
use bevy::prelude::*;

pub fn check_collisions(
    mut commands: Commands,
    mut projectiles: Query<
        (
            Entity,
            &Transform,
            &ProjectileOwner,
            &ProjectileDamage,
            &ProjectileKnockback,
            &mut ProjectilePenetration,
        ),
        With<Projectile>,
    >,
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
    mut rng: ResMut<Rng>,
    mut bot_progress: Query<(
        &mut EnemyBotXp,
        &mut EnemyBotLevel,
        &mut EnemyBotUpgrades,
        &EnemyBotEvolution,
        &mut EnemyBotHealth,
    )>,
) {
    let collision_dist = constants::PROJECTILE_RADIUS + constants::SHAPE_RADIUS;
    let collision_dist_sq = collision_dist * collision_dist;

    for (
        proj_entity,
        proj_transform,
        projectile_owner,
        projectile_damage,
        projectile_knockback,
        mut penetration,
    ) in projectiles.iter_mut()
    {
        if penetration.0 == 0 {
            commands.entity(proj_entity).despawn();
            continue;
        }

        let proj_pos = proj_transform.translation.xy();
        for (shape_entity, shape_pos, mut health, xp_val, mut velocity) in shapes.iter_mut() {
            let dist_sq = proj_pos.distance_squared(shape_pos.translation.xy());
            if dist_sq < collision_dist_sq {
                if health.0 == 0 {
                    commands.entity(shape_entity).despawn();
                    continue;
                }

                let knockback_dir = (shape_pos.translation.xy() - proj_pos).normalize_or_zero();
                velocity.0 +=
                    knockback_dir * constants::SHAPE_KNOCKBACK_SPEED * projectile_knockback.0;
                if apply_shape_damage(&mut health, projectile_damage.0) {
                    commands.entity(shape_entity).despawn();
                    match *projectile_owner {
                        ProjectileOwner::Player => {
                            xp.0 += xp_val.0;
                            total_xp.0 += xp_val.0;
                        }
                        ProjectileOwner::EnemyBot(bot_entity) => {
                            if let Ok((
                                mut bot_xp,
                                mut bot_level,
                                mut bot_upgrades,
                                bot_evolution,
                                mut bot_health,
                            )) = bot_progress.get_mut(bot_entity)
                            {
                                award_enemy_bot_xp(
                                    xp_val.0,
                                    &mut bot_xp,
                                    &mut bot_level,
                                    &mut bot_upgrades,
                                    bot_evolution,
                                    &mut bot_health,
                                    &mut rng,
                                );
                            }
                        }
                    }
                }
                penetration.0 = penetration.0.saturating_sub(1);
                if penetration.0 == 0 {
                    commands.entity(proj_entity).despawn();
                    break;
                }
            }
        }
    }
}

pub fn check_projectile_enemy_bot_collisions(
    mut commands: Commands,
    mut projectiles: Query<
        (
            Entity,
            &Transform,
            &ProjectileOwner,
            &ProjectileDamage,
            &ProjectileKnockback,
            &mut ProjectilePenetration,
        ),
        With<Projectile>,
    >,
    mut bots: Query<
        (
            Entity,
            &Transform,
            &mut EnemyBotHealth,
            &mut EnemyBotVelocity,
            &mut Visibility,
        ),
        With<EnemyBot>,
    >,
) {
    let collision_dist = constants::PROJECTILE_RADIUS + constants::PLAYER_RADIUS;
    let collision_dist_sq = collision_dist * collision_dist;

    for (
        proj_entity,
        proj_transform,
        projectile_owner,
        projectile_damage,
        projectile_knockback,
        mut penetration,
    ) in projectiles.iter_mut()
    {
        if penetration.0 == 0 {
            commands.entity(proj_entity).despawn();
            continue;
        }

        let proj_pos = proj_transform.translation.xy();
        for (bot_entity, bot_transform, mut health, mut velocity, mut visibility) in bots.iter_mut()
        {
            if health.current == 0 {
                continue;
            }
            if let ProjectileOwner::EnemyBot(owner) = *projectile_owner {
                if owner == bot_entity {
                    continue;
                }
            }

            let dist_sq = proj_pos.distance_squared(bot_transform.translation.xy());
            if dist_sq < collision_dist_sq {
                let knockback_dir = (bot_transform.translation.xy() - proj_pos).normalize_or_zero();
                velocity.0 += knockback_dir
                    * constants::PLAYER_COLLISION_KNOCKBACK_SPEED
                    * projectile_knockback.0;
                if apply_enemy_bot_damage(&mut health, projectile_damage.0) {
                    *visibility = Visibility::Hidden;
                }
                penetration.0 = penetration.0.saturating_sub(1);
                if penetration.0 == 0 {
                    commands.entity(proj_entity).despawn();
                    break;
                }
            }
        }
    }
}

pub fn check_projectile_player_collisions(
    mut commands: Commands,
    mut phase: ResMut<GamePhase>,
    run_stats: Res<RunStats>,
    total_xp: Res<TotalXp>,
    level: Res<crate::shape::Level>,
    evolution: Res<EvolutionState>,
    mut death_summary: ResMut<DeathSummary>,
    mut projectiles: Query<
        (
            Entity,
            &Transform,
            &ProjectileOwner,
            &ProjectileDamage,
            &ProjectileKnockback,
            &mut ProjectilePenetration,
        ),
        With<Projectile>,
    >,
    mut player: Query<
        (&Transform, &mut Velocity, &mut PlayerHealth),
        (With<Player>, Without<EnemyBot>),
    >,
    bot_names: Query<&EnemyBotName, With<EnemyBot>>,
) {
    let Ok((player_transform, mut player_velocity, mut player_health)) = player.single_mut() else {
        return;
    };
    let collision_dist = constants::PROJECTILE_RADIUS + constants::PLAYER_RADIUS;
    let collision_dist_sq = collision_dist * collision_dist;
    let player_pos = player_transform.translation.xy();

    for (
        proj_entity,
        proj_transform,
        projectile_owner,
        projectile_damage,
        projectile_knockback,
        mut penetration,
    ) in projectiles.iter_mut()
    {
        let ProjectileOwner::EnemyBot(owner) = *projectile_owner else {
            continue;
        };
        if penetration.0 == 0 {
            commands.entity(proj_entity).despawn();
            continue;
        }

        let proj_pos = proj_transform.translation.xy();
        if proj_pos.distance_squared(player_pos) >= collision_dist_sq {
            continue;
        }

        let knockback_dir = (player_pos - proj_pos).normalize_or_zero();
        player_velocity.0 +=
            knockback_dir * constants::PLAYER_COLLISION_KNOCKBACK_SPEED * projectile_knockback.0;
        player_health.current = player_health.current.saturating_sub(projectile_damage.0);
        penetration.0 = penetration.0.saturating_sub(1);
        if penetration.0 == 0 {
            commands.entity(proj_entity).despawn();
        }

        if player_health.current == 0 {
            death_summary.killed_by = bot_names
                .get(owner)
                .map(|name| name.0.clone())
                .unwrap_or_else(|_| "Enemy Bot".to_string());
            death_summary.score = total_xp.0;
            death_summary.level = level.0;
            death_summary.time_alive = run_stats.time_alive;
            death_summary.tank_name = evolution.current_name.clone();
            *phase = GamePhase::Dead;
            break;
        }
    }
}

pub fn check_player_shape_collisions(
    mut commands: Commands,
    mut phase: ResMut<GamePhase>,
    run_stats: Res<RunStats>,
    upgrades: Res<UpgradeState>,
    evolution: Res<EvolutionState>,
    mut xp: ResMut<Xp>,
    mut total_xp: ResMut<TotalXp>,
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
        (
            Entity,
            &mut Transform,
            &mut ShapeVelocity,
            &ShapeDamage,
            &ShapeKind,
            &mut Health,
            &mut ShapeContactCooldown,
            &XpValue,
        ),
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
    let body_damage = upgrades.body_damage() + evolution.body_damage_bonus();

    for (
        shape_entity,
        mut shape_transform,
        mut shape_velocity,
        shape_damage,
        shape_kind,
        mut shape_health,
        mut shape_contact_cooldown,
        xp_value,
    ) in shapes.iter_mut()
    {
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

        if body_damage > 0 && shape_contact_cooldown.0 <= 0.0 {
            if shape_health.0 == 0 {
                commands.entity(shape_entity).despawn();
                continue;
            }

            if apply_shape_damage(&mut shape_health, body_damage) {
                commands.entity(shape_entity).despawn();
                xp.0 += xp_value.0;
                total_xp.0 += xp_value.0;
            }
            shape_contact_cooldown.0 = constants::SHAPE_SHAPE_DAMAGE_COOLDOWN;
        }

        if damage_cooldown.0 <= 0.0 {
            player_health.current = player_health.current.saturating_sub(shape_damage.0);
            damage_cooldown.0 = constants::PLAYER_DAMAGE_COOLDOWN;
            if player_health.current == 0 {
                death_summary.killed_by = shape_kind.name().to_string();
                death_summary.score = total_xp.0;
                death_summary.level = level.0;
                death_summary.time_alive = run_stats.time_alive;
                death_summary.tank_name = evolution.current_name.clone();
                *phase = GamePhase::Dead;
                break;
            }
        }
    }
}

pub fn check_player_enemy_bot_collisions(
    mut phase: ResMut<GamePhase>,
    run_stats: Res<RunStats>,
    upgrades: Res<UpgradeState>,
    evolution: Res<EvolutionState>,
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
        (With<Player>, Without<EnemyBot>),
    >,
    mut bots: Query<
        (
            &mut Transform,
            &mut EnemyBotVelocity,
            &mut EnemyBotHealth,
            &mut EnemyBotDamageCooldown,
            &EnemyBotName,
            &EnemyBotUpgrades,
            &EnemyBotEvolution,
            &mut Visibility,
        ),
        (With<EnemyBot>, Without<Player>),
    >,
) {
    let Ok((mut player_transform, mut player_velocity, mut player_health, mut damage_cooldown)) =
        player.single_mut()
    else {
        return;
    };
    let collision_distance = constants::PLAYER_RADIUS * 2.0;
    let collision_distance_sq = collision_distance * collision_distance;
    let half = constants::arena_half_extent() - constants::PLAYER_RADIUS;
    let body_damage = upgrades.body_damage() + evolution.body_damage_bonus();

    for (
        mut bot_transform,
        mut bot_velocity,
        mut bot_health,
        mut bot_damage_cooldown,
        bot_name,
        bot_upgrades,
        bot_evolution,
        mut bot_visibility,
    ) in bots.iter_mut()
    {
        if bot_health.current == 0 {
            continue;
        }

        let player_pos = player_transform.translation.xy();
        let bot_pos = bot_transform.translation.xy();
        let delta = player_pos - bot_pos;
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

        player_transform.translation += (normal * penetration * 0.5).extend(0.0);
        bot_transform.translation -= (normal * penetration * 0.5).extend(0.0);

        player_transform.translation.x = player_transform.translation.x.clamp(-half, half);
        player_transform.translation.y = player_transform.translation.y.clamp(-half, half);
        bot_transform.translation.x = bot_transform.translation.x.clamp(-half, half);
        bot_transform.translation.y = bot_transform.translation.y.clamp(-half, half);

        player_velocity.0 += normal * constants::PLAYER_COLLISION_KNOCKBACK_SPEED;
        bot_velocity.0 -= normal * constants::PLAYER_COLLISION_KNOCKBACK_SPEED;

        if body_damage > 0 && bot_damage_cooldown.0 <= 0.0 {
            if apply_enemy_bot_damage(&mut bot_health, body_damage) {
                *bot_visibility = Visibility::Hidden;
            }
            bot_damage_cooldown.0 = constants::PLAYER_DAMAGE_COOLDOWN;
        }

        if damage_cooldown.0 <= 0.0 {
            let bot_body_damage = constants::shape_damage(4)
                + bot_upgrades.0.body_damage()
                + bot_evolution.0.body_damage_bonus();
            player_health.current = player_health.current.saturating_sub(bot_body_damage);
            damage_cooldown.0 = constants::PLAYER_DAMAGE_COOLDOWN;
            if player_health.current == 0 {
                death_summary.killed_by = bot_name.0.clone();
                death_summary.score = total_xp.0;
                death_summary.level = level.0;
                death_summary.time_alive = run_stats.time_alive;
                death_summary.tank_name = evolution.current_name.clone();
                *phase = GamePhase::Dead;
                break;
            }
        }
    }
}

pub fn check_enemy_bot_shape_collisions(
    mut commands: Commands,
    mut rng: ResMut<Rng>,
    mut bots: Query<
        (
            &mut Transform,
            &mut EnemyBotVelocity,
            &mut EnemyBotHealth,
            &mut EnemyBotDamageCooldown,
            &mut EnemyBotUpgrades,
            &EnemyBotEvolution,
            &mut EnemyBotXp,
            &mut EnemyBotLevel,
            &mut Visibility,
        ),
        (With<EnemyBot>, Without<Shape>),
    >,
    mut shapes: Query<
        (
            Entity,
            &mut Transform,
            &mut ShapeVelocity,
            &ShapeDamage,
            &mut Health,
            &mut ShapeContactCooldown,
            &XpValue,
        ),
        (With<Shape>, Without<EnemyBot>),
    >,
) {
    let collision_distance = constants::PLAYER_RADIUS + constants::SHAPE_RADIUS;
    let collision_distance_sq = collision_distance * collision_distance;
    let bot_half = constants::arena_half_extent() - constants::PLAYER_RADIUS;
    let shape_half = constants::arena_half_extent() - constants::SHAPE_RADIUS;

    for (
        mut bot_transform,
        mut bot_velocity,
        mut bot_health,
        mut damage_cooldown,
        mut bot_upgrades,
        bot_evolution,
        mut bot_xp,
        mut bot_level,
        mut visibility,
    ) in bots.iter_mut()
    {
        if bot_health.current == 0 {
            continue;
        }

        for (
            shape_entity,
            mut shape_transform,
            mut shape_velocity,
            shape_damage,
            mut shape_health,
            mut shape_contact_cooldown,
            xp_value,
        ) in shapes.iter_mut()
        {
            let bot_pos = bot_transform.translation.xy();
            let shape_pos = shape_transform.translation.xy();
            let delta = bot_pos - shape_pos;
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

            bot_transform.translation += (normal * penetration * 0.55).extend(0.0);
            shape_transform.translation -= (normal * penetration * 0.45).extend(0.0);

            bot_transform.translation.x = bot_transform.translation.x.clamp(-bot_half, bot_half);
            bot_transform.translation.y = bot_transform.translation.y.clamp(-bot_half, bot_half);
            shape_transform.translation.x =
                shape_transform.translation.x.clamp(-shape_half, shape_half);
            shape_transform.translation.y =
                shape_transform.translation.y.clamp(-shape_half, shape_half);

            bot_velocity.0 += normal * constants::PLAYER_COLLISION_KNOCKBACK_SPEED;
            shape_velocity.0 -= normal * constants::SHAPE_COLLISION_KNOCKBACK_SPEED;

            let body_damage = bot_upgrades.0.body_damage() + bot_evolution.0.body_damage_bonus();
            if body_damage > 0 && shape_contact_cooldown.0 <= 0.0 {
                if shape_health.0 == 0 {
                    commands.entity(shape_entity).despawn();
                    continue;
                }

                if apply_shape_damage(&mut shape_health, body_damage) {
                    commands.entity(shape_entity).despawn();
                    award_enemy_bot_xp(
                        xp_value.0,
                        &mut bot_xp,
                        &mut bot_level,
                        &mut bot_upgrades,
                        bot_evolution,
                        &mut bot_health,
                        &mut rng,
                    );
                }
                shape_contact_cooldown.0 = constants::SHAPE_SHAPE_DAMAGE_COOLDOWN;
            }

            if damage_cooldown.0 <= 0.0 {
                if apply_enemy_bot_damage(&mut bot_health, shape_damage.0) {
                    *visibility = Visibility::Hidden;
                    break;
                }
                damage_cooldown.0 = constants::PLAYER_DAMAGE_COOLDOWN;
            }
        }
    }
}

pub fn check_enemy_bot_enemy_bot_collisions(
    mut bots: Query<(&mut Transform, &mut EnemyBotVelocity, &EnemyBotHealth), With<EnemyBot>>,
) {
    let collision_distance = constants::PLAYER_RADIUS * 2.0;
    let collision_distance_sq = collision_distance * collision_distance;
    let repulsion_distance = collision_distance * 3.0;
    let repulsion_distance_sq = repulsion_distance * repulsion_distance;
    let half = constants::arena_half_extent() - constants::PLAYER_RADIUS;

    let mut combinations = bots.iter_combinations_mut::<2>();
    while let Some(
        [
            (mut transform_a, mut velocity_a, health_a),
            (mut transform_b, mut velocity_b, health_b),
        ],
    ) = combinations.fetch_next()
    {
        if health_a.current == 0 || health_b.current == 0 {
            continue;
        }

        let pos_a = transform_a.translation.xy();
        let pos_b = transform_b.translation.xy();
        let delta = pos_a - pos_b;
        let distance_sq = delta.length_squared();

        if distance_sq >= repulsion_distance_sq {
            continue;
        }

        let distance = distance_sq.sqrt();
        let normal = if distance > 0.001 {
            delta / distance
        } else {
            Vec2::X
        };
        if distance_sq < collision_distance_sq {
            let penetration = collision_distance - distance;

            transform_a.translation += (normal * penetration * 0.75).extend(0.0);
            transform_b.translation -= (normal * penetration * 0.75).extend(0.0);

            transform_a.translation.x = transform_a.translation.x.clamp(-half, half);
            transform_a.translation.y = transform_a.translation.y.clamp(-half, half);
            transform_b.translation.x = transform_b.translation.x.clamp(-half, half);
            transform_b.translation.y = transform_b.translation.y.clamp(-half, half);
        }

        let repulsion_strength = (1.0 - distance / repulsion_distance).clamp(0.0, 1.0);
        let impulse =
            normal * constants::PLAYER_COLLISION_KNOCKBACK_SPEED * (1.5 + repulsion_strength * 2.5);
        velocity_a.0 += impulse;
        velocity_b.0 -= impulse;
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
