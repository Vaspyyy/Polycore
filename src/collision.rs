use crate::{
    combat::{CombatDeathQueue, CombatStats, CombatantId},
    constants,
    enemy_bot::{
        EnemyBot, EnemyBotBrain, EnemyBotDamageCooldown, EnemyBotEvolution, EnemyBotHealth,
        EnemyBotLevel, EnemyBotMoveVelocity, EnemyBotName, EnemyBotPlaystyle, EnemyBotRespawnTimer,
        EnemyBotUpgrades, EnemyBotVelocity, EnemyBotXp, apply_enemy_bot_damage, award_enemy_bot_xp,
        finish_enemy_bot_death,
    },
    evolution::EvolutionState,
    hud::UpgradeState,
    menu::{DeathSummary, GamePhase, RunStats},
    passive::{PassiveRuntime, body_damage_multiplier, is_frontal_hit},
    player::{DamageCooldown, MoveVelocity, Player, PlayerHealth, Velocity},
    projectile::{
        Projectile, ProjectileDamage, ProjectileEvolution, ProjectileHitHistory,
        ProjectileKnockback, ProjectileOwner, ProjectilePenetration, ProjectileRear,
        ProjectileSplashReady, ProjectileTravel,
    },
    rng::Rng,
    shape::{
        Health, Shape, ShapeContactCooldown, ShapeDamage, ShapeKind, ShapeVelocity, TotalXp, Xp,
        XpValue,
    },
    tank::{RecentDamage, SpawnProtection},
};
use bevy::prelude::*;

const SPLASH_RADIUS: f32 = 90.0;

#[derive(Component, Clone, Copy)]
pub struct PendingSplash {
    position: Vec2,
    owner: ProjectileOwner,
    direct_target: Entity,
    damage: f32,
}

fn splash_damage(base_damage: f32, distance: f32) -> f32 {
    base_damage * 0.65 * (1.0 - distance / SPLASH_RADIUS).clamp(0.0, 1.0)
}

pub fn check_collisions(
    mut commands: Commands,
    grid: Res<crate::spatial::SpatialGrid>,
    mut projectiles: Query<
        (
            Entity,
            &Transform,
            &ProjectileOwner,
            &ProjectileDamage,
            &ProjectileKnockback,
            &mut ProjectilePenetration,
            &mut ProjectileHitHistory,
        ),
        (With<Projectile>, Without<Player>),
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
    mut player_shape_kills: ResMut<crate::dominance::PlayerShapeKills>,
    mut player_stats: Query<(&mut CombatStats, &PlayerHealth), (With<Player>, Without<EnemyBot>)>,
    mut rng: ResMut<Rng>,
    mut bot_progress: Query<
        (
            &mut EnemyBotXp,
            &mut EnemyBotLevel,
            &mut EnemyBotUpgrades,
            &mut EnemyBotEvolution,
            &mut EnemyBotHealth,
            &EnemyBotPlaystyle,
            &mut CombatStats,
        ),
        (With<EnemyBot>, Without<Player>),
    >,
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
        mut hit_history,
    ) in projectiles.iter_mut()
    {
        if penetration.0 == 0 {
            commands.entity(proj_entity).despawn();
            continue;
        }

        let proj_pos = proj_transform.translation.xy();
        for candidate in grid.nearby(proj_pos, collision_dist) {
            if candidate.kind != crate::spatial::SpatialKind::Shape {
                continue;
            }
            let Ok((shape_entity, shape_pos, mut health, xp_val, mut velocity)) =
                shapes.get_mut(candidate.entity)
            else {
                continue;
            };
            let dist_sq = proj_pos.distance_squared(shape_pos.translation.xy());
            if dist_sq < collision_dist_sq {
                if health.0 <= 0.0 {
                    commands.entity(shape_entity).despawn();
                    continue;
                }

                if !hit_history.record(shape_entity) {
                    continue;
                }
                let knockback_dir = (shape_pos.translation.xy() - proj_pos).normalize_or_zero();
                velocity.0 +=
                    knockback_dir * constants::SHAPE_KNOCKBACK_SPEED * projectile_knockback.0;
                if apply_shape_damage(&mut health, projectile_damage.0) {
                    commands.entity(shape_entity).despawn();
                    match *projectile_owner {
                        ProjectileOwner::Player => {
                            if let Ok((mut stats, health)) = player_stats.single_mut()
                                && health.current > 0.0
                            {
                                xp.0 += xp_val.0;
                                total_xp.0 += xp_val.0;
                                stats.life_score = stats.life_score.saturating_add(xp_val.0);
                                player_shape_kills.0 = player_shape_kills.0.saturating_add(1);
                            }
                        }
                        ProjectileOwner::EnemyBot(bot_entity) => {
                            if let Ok((
                                mut bot_xp,
                                mut bot_level,
                                mut bot_upgrades,
                                mut bot_evolution,
                                mut bot_health,
                                playstyle,
                                mut stats,
                            )) = bot_progress.get_mut(bot_entity)
                            {
                                award_enemy_bot_xp(
                                    xp_val.0,
                                    &mut bot_xp,
                                    &mut bot_level,
                                    &mut bot_upgrades,
                                    &mut bot_evolution,
                                    &mut bot_health,
                                    playstyle,
                                    &mut stats,
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

#[allow(clippy::type_complexity)]
pub fn check_projectile_enemy_bot_collisions(
    mut commands: Commands,
    grid: Res<crate::spatial::SpatialGrid>,
    mut deaths: ResMut<CombatDeathQueue>,
    player_entity: Query<Entity, With<Player>>,
    mut projectiles: Query<
        (
            Entity,
            &Transform,
            &ProjectileOwner,
            &ProjectileDamage,
            &ProjectileKnockback,
            &Velocity,
            &ProjectileEvolution,
            &ProjectileTravel,
            &ProjectileRear,
            &mut ProjectileSplashReady,
            &mut ProjectilePenetration,
            &mut ProjectileHitHistory,
        ),
        With<Projectile>,
    >,
    mut player_runtime: Query<&mut PassiveRuntime, (With<Player>, Without<EnemyBot>)>,
    mut bots: ParamSet<(
        Query<
            (
                Entity,
                &Transform,
                &mut EnemyBotHealth,
                &mut EnemyBotVelocity,
                &EnemyBotMoveVelocity,
                &mut Visibility,
                &mut EnemyBotRespawnTimer,
                &mut EnemyBotBrain,
                &SpawnProtection,
                &mut RecentDamage,
                &EnemyBotEvolution,
                &EnemyBotUpgrades,
                &mut PassiveRuntime,
            ),
            With<EnemyBot>,
        >,
        Query<&mut PassiveRuntime, With<EnemyBot>>,
    )>,
) {
    for (
        proj_entity,
        proj_transform,
        projectile_owner,
        projectile_damage,
        projectile_knockback,
        projectile_velocity,
        projectile_evolution,
        travel,
        rear,
        mut splash_ready,
        mut penetration,
        mut hit_history,
    ) in projectiles.iter_mut()
    {
        if penetration.0 == 0 {
            commands.entity(proj_entity).despawn();
            continue;
        }

        let proj_pos = proj_transform.translation.xy();
        for candidate in grid.nearby(proj_pos, constants::PROJECTILE_RADIUS + 25.0) {
            if candidate.kind != crate::spatial::SpatialKind::Tank {
                continue;
            }
            let source_multiplier = match *projectile_owner {
                ProjectileOwner::Player => {
                    player_runtime.single_mut().map_or(1.0, |mut runtime| {
                        runtime.projectile_hit_multiplier(
                            crate::evolution::definition(projectile_evolution.0).passive,
                            candidate.entity,
                            travel.0,
                        )
                    })
                }
                ProjectileOwner::EnemyBot(owner) => {
                    bots.p1().get_mut(owner).map_or(1.0, |mut runtime| {
                        runtime.projectile_hit_multiplier(
                            crate::evolution::definition(projectile_evolution.0).passive,
                            candidate.entity,
                            travel.0,
                        )
                    })
                }
            };
            let mut target_query = bots.p0();
            let Ok((
                bot_entity,
                bot_transform,
                mut health,
                mut velocity,
                move_velocity,
                mut visibility,
                mut respawn_timer,
                mut brain,
                protection,
                mut recent_damage,
                bot_evolution,
                bot_upgrades,
                mut passive_runtime,
            )) = target_query.get_mut(candidate.entity)
            else {
                continue;
            };
            if health.current <= 0.0 {
                continue;
            }
            if let ProjectileOwner::EnemyBot(owner) = *projectile_owner
                && owner == bot_entity
            {
                continue;
            }
            if protection.active() {
                continue;
            }

            let collision_dist =
                constants::PROJECTILE_RADIUS + crate::tank::radius(&bot_evolution.0);
            let collision_dist_sq = collision_dist * collision_dist;
            let dist_sq = proj_pos.distance_squared(bot_transform.translation.xy());
            if dist_sq < collision_dist_sq {
                if !hit_history.record(bot_entity) {
                    continue;
                }
                let (attacker, killer) = match *projectile_owner {
                    ProjectileOwner::Player => {
                        (player_entity.single().ok(), Some(CombatantId::Player))
                    }
                    ProjectileOwner::EnemyBot(owner) => {
                        (Some(owner), Some(CombatantId::EnemyBot(owner)))
                    }
                };
                if let Some(attacker) = attacker {
                    brain.note_attacker(attacker);
                }
                let speed_fraction = move_velocity.0.length()
                    / (bot_upgrades.0.movement_speed() * bot_evolution.0.movement_multiplier())
                        .max(1.0);
                let frontal =
                    is_frontal_hit(bot_transform, projectile_velocity.0, 50.0_f32.to_radians());
                let applied_damage = passive_runtime.incoming_damage(
                    bot_evolution.0.passive(),
                    projectile_damage.0 * source_multiplier,
                    frontal,
                    speed_fraction,
                );
                recent_damage.record(applied_damage);
                let knockback_dir = (bot_transform.translation.xy() - proj_pos).normalize_or_zero();
                velocity.0 += knockback_dir
                    * constants::PLAYER_COLLISION_KNOCKBACK_SPEED
                    * projectile_knockback.0
                    * if rear.0 { 1.8 } else { 1.0 };
                if splash_ready.0 {
                    commands.spawn(PendingSplash {
                        position: proj_pos,
                        owner: *projectile_owner,
                        direct_target: bot_entity,
                        damage: projectile_damage.0 * source_multiplier,
                    });
                    splash_ready.0 = false;
                }
                if apply_enemy_bot_damage(&mut health, applied_damage) {
                    finish_enemy_bot_death(
                        bot_entity,
                        &mut visibility,
                        &mut respawn_timer,
                        &mut deaths,
                        killer,
                    );
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
    mut deaths: ResMut<CombatDeathQueue>,
    run_stats: Res<RunStats>,
    total_xp: Res<TotalXp>,
    level: Res<crate::shape::Level>,
    evolution: Res<EvolutionState>,
    upgrades: Res<UpgradeState>,
    mut death_summary: ResMut<DeathSummary>,
    mut projectiles: Query<
        (
            Entity,
            &Transform,
            &ProjectileOwner,
            &ProjectileDamage,
            &ProjectileKnockback,
            &Velocity,
            &ProjectileEvolution,
            &ProjectileTravel,
            &ProjectileRear,
            &mut ProjectileSplashReady,
            &mut ProjectilePenetration,
            &mut ProjectileHitHistory,
        ),
        (With<Projectile>, Without<Player>),
    >,
    mut player: Query<
        (
            Entity,
            &Transform,
            &mut Velocity,
            &MoveVelocity,
            &mut PlayerHealth,
            &SpawnProtection,
            &mut RecentDamage,
            &mut PassiveRuntime,
        ),
        (With<Player>, Without<EnemyBot>),
    >,
    bot_names: Query<&EnemyBotName, With<EnemyBot>>,
    mut bot_runtimes: Query<&mut PassiveRuntime, (With<EnemyBot>, Without<Player>)>,
) {
    let Ok((
        player_entity,
        player_transform,
        mut player_velocity,
        player_move_velocity,
        mut player_health,
        protection,
        mut recent_damage,
        mut player_runtime,
    )) = player.single_mut()
    else {
        return;
    };
    if player_health.current <= 0.0 {
        return;
    }
    if protection.active() {
        return;
    }
    let collision_dist = constants::PROJECTILE_RADIUS + crate::tank::radius(&evolution);
    let collision_dist_sq = collision_dist * collision_dist;
    let player_pos = player_transform.translation.xy();

    for (
        proj_entity,
        proj_transform,
        projectile_owner,
        projectile_damage,
        projectile_knockback,
        projectile_velocity,
        projectile_evolution,
        travel,
        rear,
        mut splash_ready,
        mut penetration,
        mut hit_history,
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
        if !hit_history.record(player_entity) {
            continue;
        }

        let source_multiplier = bot_runtimes.get_mut(owner).map_or(1.0, |mut runtime| {
            runtime.projectile_hit_multiplier(
                crate::evolution::definition(projectile_evolution.0).passive,
                player_entity,
                travel.0,
            )
        });
        let speed_fraction = player_move_velocity.0.length()
            / (upgrades.movement_speed() * evolution.movement_multiplier()).max(1.0);
        let frontal = is_frontal_hit(
            player_transform,
            projectile_velocity.0,
            50.0_f32.to_radians(),
        );
        let applied_damage = player_runtime.incoming_damage(
            evolution.passive(),
            projectile_damage.0 * source_multiplier,
            frontal,
            speed_fraction,
        );
        let knockback_dir = (player_pos - proj_pos).normalize_or_zero();
        player_velocity.0 += knockback_dir
            * constants::PLAYER_COLLISION_KNOCKBACK_SPEED
            * projectile_knockback.0
            * if rear.0 { 1.8 } else { 1.0 };
        if splash_ready.0 {
            commands.spawn(PendingSplash {
                position: proj_pos,
                owner: *projectile_owner,
                direct_target: player_entity,
                damage: projectile_damage.0 * source_multiplier,
            });
            splash_ready.0 = false;
        }
        let was_killed = apply_player_damage(&mut player_health, applied_damage);
        recent_damage.record(applied_damage);
        penetration.0 = penetration.0.saturating_sub(1);
        if penetration.0 == 0 {
            commands.entity(proj_entity).despawn();
        }

        if was_killed {
            death_summary.killed_by = bot_names
                .get(owner)
                .map(|name| name.0.clone())
                .unwrap_or_else(|_| "Enemy Bot".to_string());
            death_summary.score = total_xp.0;
            death_summary.level = level.0;
            death_summary.time_alive = run_stats.time_alive;
            death_summary.tank_name = evolution.current_name.clone();
            deaths.record(CombatantId::Player, Some(CombatantId::EnemyBot(owner)));
            *phase = GamePhase::Dead;
            break;
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn resolve_pending_splashes(
    mut commands: Commands,
    splashes: Query<(Entity, &PendingSplash)>,
    mut phase: ResMut<GamePhase>,
    mut deaths: ResMut<CombatDeathQueue>,
    run_stats: Res<RunStats>,
    total_xp: Res<TotalXp>,
    level: Res<crate::shape::Level>,
    player_evolution: Res<EvolutionState>,
    player_upgrades: Res<UpgradeState>,
    mut death_summary: ResMut<DeathSummary>,
    bot_names: Query<&EnemyBotName, With<EnemyBot>>,
    mut player: Query<
        (
            Entity,
            &Transform,
            &MoveVelocity,
            &mut PlayerHealth,
            &SpawnProtection,
            &mut RecentDamage,
            &mut PassiveRuntime,
        ),
        (With<Player>, Without<EnemyBot>),
    >,
    mut bots: Query<
        (
            Entity,
            &Transform,
            &EnemyBotMoveVelocity,
            &mut EnemyBotHealth,
            &mut Visibility,
            &mut EnemyBotRespawnTimer,
            &mut EnemyBotBrain,
            &SpawnProtection,
            &mut RecentDamage,
            &EnemyBotEvolution,
            &EnemyBotUpgrades,
            &mut PassiveRuntime,
        ),
        (With<EnemyBot>, Without<Player>),
    >,
) {
    for (splash_entity, splash) in &splashes {
        let killer = match splash.owner {
            ProjectileOwner::Player => Some(CombatantId::Player),
            ProjectileOwner::EnemyBot(owner) => Some(CombatantId::EnemyBot(owner)),
        };
        let attacker = match splash.owner {
            ProjectileOwner::Player => player.single().ok().map(|player| player.0),
            ProjectileOwner::EnemyBot(owner) => Some(owner),
        };

        if let Ok((
            player_entity,
            transform,
            move_velocity,
            mut health,
            protection,
            mut recent,
            mut runtime,
        )) = player.single_mut()
            && splash.owner != ProjectileOwner::Player
            && splash.direct_target != player_entity
            && health.current > 0.0
            && !protection.active()
        {
            let distance = transform.translation.xy().distance(splash.position);
            if distance < SPLASH_RADIUS {
                let speed_fraction = move_velocity.0.length()
                    / (player_upgrades.movement_speed() * player_evolution.movement_multiplier())
                        .max(1.0);
                let damage = runtime.incoming_damage(
                    player_evolution.passive(),
                    splash_damage(splash.damage, distance),
                    false,
                    speed_fraction,
                );
                recent.record(damage);
                if apply_player_damage(&mut health, damage) {
                    let owner = match splash.owner {
                        ProjectileOwner::EnemyBot(owner) => owner,
                        ProjectileOwner::Player => unreachable!(),
                    };
                    death_summary.killed_by = bot_names
                        .get(owner)
                        .map(|name| name.0.clone())
                        .unwrap_or_else(|_| "Enemy Bot".to_string());
                    death_summary.score = total_xp.0;
                    death_summary.level = level.0;
                    death_summary.time_alive = run_stats.time_alive;
                    death_summary.tank_name = player_evolution.current_name.clone();
                    deaths.record(CombatantId::Player, killer);
                    *phase = GamePhase::Dead;
                }
            }
        }

        for (
            entity,
            transform,
            move_velocity,
            mut health,
            mut visibility,
            mut respawn_timer,
            mut brain,
            protection,
            mut recent,
            evolution,
            upgrades,
            mut runtime,
        ) in &mut bots
        {
            if entity == splash.direct_target
                || matches!(splash.owner, ProjectileOwner::EnemyBot(owner) if owner == entity)
                || health.current <= 0.0
                || protection.active()
            {
                continue;
            }
            let distance = transform.translation.xy().distance(splash.position);
            if distance >= SPLASH_RADIUS {
                continue;
            }
            let speed_fraction = move_velocity.0.length()
                / (upgrades.0.movement_speed() * evolution.0.movement_multiplier()).max(1.0);
            let damage = runtime.incoming_damage(
                evolution.0.passive(),
                splash_damage(splash.damage, distance),
                false,
                speed_fraction,
            );
            if let Some(attacker) = attacker {
                brain.note_attacker(attacker);
            }
            recent.record(damage);
            if apply_enemy_bot_damage(&mut health, damage) {
                finish_enemy_bot_death(
                    entity,
                    &mut visibility,
                    &mut respawn_timer,
                    &mut deaths,
                    killer,
                );
            }
        }
        commands.entity(splash_entity).despawn();
    }
}

pub fn check_player_shape_collisions(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut phase: ResMut<GamePhase>,
    mut deaths: ResMut<CombatDeathQueue>,
    run_stats: Res<RunStats>,
    upgrades: Res<UpgradeState>,
    evolution: Res<EvolutionState>,
    mut xp: ResMut<Xp>,
    mut total_xp: ResMut<TotalXp>,
    mut player_shape_kills: ResMut<crate::dominance::PlayerShapeKills>,
    level: Res<crate::shape::Level>,
    mut death_summary: ResMut<DeathSummary>,
    mut player: Query<
        (
            &mut Transform,
            &mut Velocity,
            &mut PlayerHealth,
            &mut DamageCooldown,
            &mut CombatStats,
            &SpawnProtection,
            &mut RecentDamage,
            &MoveVelocity,
            &mut PassiveRuntime,
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
    let Ok((
        mut player_transform,
        mut player_velocity,
        mut player_health,
        mut damage_cooldown,
        mut stats,
        protection,
        mut recent_damage,
        move_velocity,
        mut passive_runtime,
    )) = player.single_mut()
    else {
        return;
    };
    if player_health.current <= 0.0 {
        return;
    }
    let player_radius = crate::tank::radius(&evolution);
    let collision_distance = player_radius + constants::SHAPE_RADIUS;
    let collision_distance_sq = collision_distance * collision_distance;
    let player_half = constants::arena_half_extent() - player_radius;
    let shape_half = constants::arena_half_extent() - constants::SHAPE_RADIUS;
    let speed_fraction = move_velocity.0.length()
        / (upgrades.movement_speed() * evolution.movement_multiplier()).max(1.0);
    let body_damage = crate::tank::body_damage(upgrades.body_damage(), &evolution)
        * body_damage_multiplier(evolution.passive(), speed_fraction);
    let dt = time.delta_secs();

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

        if protection.active() {
            continue;
        }

        if shape_health.0 <= 0.0 {
            commands.entity(shape_entity).despawn();
            continue;
        }
        let damage_to_shape = crate::tank::contact_damage_for_step(body_damage, dt);
        let damage_to_player = passive_runtime.incoming_contact_damage(
            evolution.passive(),
            crate::tank::contact_damage_for_step(shape_damage.0, dt),
            speed_fraction,
        );
        let shape_killed = apply_shape_damage(&mut shape_health, damage_to_shape);
        let player_killed = apply_player_damage(&mut player_health, damage_to_player);
        recent_damage.record(damage_to_player);
        shape_contact_cooldown.0 = 0.0;
        damage_cooldown.0 = 0.0;
        if shape_killed {
            commands.entity(shape_entity).despawn();
            xp.0 += xp_value.0;
            total_xp.0 += xp_value.0;
            stats.life_score = stats.life_score.saturating_add(xp_value.0);
            player_shape_kills.0 = player_shape_kills.0.saturating_add(1);
        }
        if player_killed {
            death_summary.killed_by = shape_kind.name().to_string();
            death_summary.score = total_xp.0;
            death_summary.level = level.0;
            death_summary.time_alive = run_stats.time_alive;
            death_summary.tank_name = evolution.current_name.clone();
            deaths.record(CombatantId::Player, None);
            *phase = GamePhase::Dead;
            break;
        }
    }
}

pub fn check_player_enemy_bot_collisions(
    time: Res<Time<Fixed>>,
    mut phase: ResMut<GamePhase>,
    mut deaths: ResMut<CombatDeathQueue>,
    run_stats: Res<RunStats>,
    upgrades: Res<UpgradeState>,
    evolution: Res<EvolutionState>,
    total_xp: Res<TotalXp>,
    level: Res<crate::shape::Level>,
    mut death_summary: ResMut<DeathSummary>,
    mut player: Query<
        (
            Entity,
            &mut Transform,
            &mut Velocity,
            &mut PlayerHealth,
            &mut DamageCooldown,
            &SpawnProtection,
            &mut RecentDamage,
            &MoveVelocity,
            &mut PassiveRuntime,
        ),
        (With<Player>, Without<EnemyBot>),
    >,
    mut bots: Query<
        (
            Entity,
            &mut Transform,
            &mut EnemyBotVelocity,
            &mut EnemyBotHealth,
            &mut EnemyBotDamageCooldown,
            &EnemyBotName,
            &EnemyBotUpgrades,
            &EnemyBotEvolution,
            &mut Visibility,
            &mut EnemyBotRespawnTimer,
            &mut EnemyBotBrain,
            &SpawnProtection,
            &mut RecentDamage,
            &EnemyBotMoveVelocity,
            &mut PassiveRuntime,
        ),
        (With<EnemyBot>, Without<Player>),
    >,
) {
    let Ok((
        player_entity,
        mut player_transform,
        mut player_velocity,
        mut player_health,
        mut damage_cooldown,
        player_protection,
        mut player_recent_damage,
        player_move_velocity,
        mut player_runtime,
    )) = player.single_mut()
    else {
        return;
    };
    if player_health.current <= 0.0 {
        return;
    }
    let player_radius = crate::tank::radius(&evolution);
    let half = constants::arena_half_extent() - player_radius;
    let body_damage = crate::tank::body_damage(upgrades.body_damage(), &evolution);
    let dt = time.delta_secs();

    for (
        bot_entity,
        mut bot_transform,
        mut bot_velocity,
        mut bot_health,
        mut bot_damage_cooldown,
        bot_name,
        bot_upgrades,
        bot_evolution,
        mut bot_visibility,
        mut bot_respawn_timer,
        mut bot_brain,
        bot_protection,
        mut bot_recent_damage,
        bot_move_velocity,
        mut bot_runtime,
    ) in bots.iter_mut()
    {
        if bot_health.current <= 0.0 {
            continue;
        }

        let bot_radius = crate::tank::radius(&bot_evolution.0);
        let collision_distance = player_radius + bot_radius;
        let collision_distance_sq = collision_distance * collision_distance;
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

        if !player_protection.active() && !bot_protection.active() {
            let player_speed_fraction = player_move_velocity.0.length()
                / (upgrades.movement_speed() * evolution.movement_multiplier()).max(1.0);
            let bot_speed_fraction = bot_move_velocity.0.length()
                / (bot_upgrades.0.movement_speed() * bot_evolution.0.movement_multiplier())
                    .max(1.0);
            let player_body_damage =
                body_damage * body_damage_multiplier(evolution.passive(), player_speed_fraction);
            let damage_to_bot = bot_runtime.incoming_contact_damage(
                bot_evolution.0.passive(),
                crate::tank::contact_damage_for_step(player_body_damage, dt),
                bot_speed_fraction,
            );
            let bot_body_damage =
                crate::tank::body_damage(bot_upgrades.0.body_damage(), &bot_evolution.0)
                    * body_damage_multiplier(bot_evolution.0.passive(), bot_speed_fraction);
            let damage_to_player = player_runtime.incoming_contact_damage(
                evolution.passive(),
                crate::tank::contact_damage_for_step(bot_body_damage, dt),
                player_speed_fraction,
            );
            bot_brain.note_attacker(player_entity);
            bot_recent_damage.record(damage_to_bot);
            player_recent_damage.record(damage_to_player);
            let bot_killed = apply_enemy_bot_damage(&mut bot_health, damage_to_bot);
            let player_killed = apply_player_damage(&mut player_health, damage_to_player);
            if bot_killed {
                finish_enemy_bot_death(
                    bot_entity,
                    &mut bot_visibility,
                    &mut bot_respawn_timer,
                    &mut deaths,
                    Some(CombatantId::Player),
                );
            }
            if player_killed {
                death_summary.killed_by = bot_name.0.clone();
                death_summary.score = total_xp.0;
                death_summary.level = level.0;
                death_summary.time_alive = run_stats.time_alive;
                death_summary.tank_name = evolution.current_name.clone();
                deaths.record(CombatantId::Player, Some(CombatantId::EnemyBot(bot_entity)));
                *phase = GamePhase::Dead;
                break;
            }
        }
        damage_cooldown.0 = 0.0;
        bot_damage_cooldown.0 = 0.0;
    }
}

pub fn check_enemy_bot_shape_collisions(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    grid: Res<crate::spatial::SpatialGrid>,
    mut rng: ResMut<Rng>,
    mut deaths: ResMut<CombatDeathQueue>,
    mut bot_passives: Query<(&EnemyBotMoveVelocity, &mut PassiveRuntime), With<EnemyBot>>,
    mut bots: Query<
        (
            Entity,
            &mut Transform,
            &mut EnemyBotVelocity,
            &mut EnemyBotHealth,
            &mut EnemyBotDamageCooldown,
            &mut EnemyBotUpgrades,
            &mut EnemyBotEvolution,
            &mut EnemyBotXp,
            &mut EnemyBotLevel,
            &mut Visibility,
            &EnemyBotPlaystyle,
            &mut CombatStats,
            &mut EnemyBotRespawnTimer,
            &SpawnProtection,
            &mut RecentDamage,
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
    let shape_half = constants::arena_half_extent() - constants::SHAPE_RADIUS;

    for (
        bot_entity,
        mut bot_transform,
        mut bot_velocity,
        mut bot_health,
        mut damage_cooldown,
        mut bot_upgrades,
        mut bot_evolution,
        mut bot_xp,
        mut bot_level,
        mut visibility,
        playstyle,
        mut stats,
        mut respawn_timer,
        protection,
        mut recent_damage,
    ) in bots.iter_mut()
    {
        if bot_health.current <= 0.0 {
            continue;
        }

        let Ok((move_velocity, mut passive_runtime)) = bot_passives.get_mut(bot_entity) else {
            continue;
        };
        let speed_fraction = move_velocity.0.length()
            / (bot_upgrades.0.movement_speed() * bot_evolution.0.movement_multiplier()).max(1.0);

        let bot_radius = crate::tank::radius(&bot_evolution.0);
        let collision_distance = bot_radius + constants::SHAPE_RADIUS;
        let collision_distance_sq = collision_distance * collision_distance;
        let bot_half = constants::arena_half_extent() - bot_radius;
        for candidate in grid.nearby(bot_transform.translation.xy(), collision_distance) {
            if candidate.kind != crate::spatial::SpatialKind::Shape {
                continue;
            }
            let Ok((
                shape_entity,
                mut shape_transform,
                mut shape_velocity,
                shape_damage,
                mut shape_health,
                mut shape_contact_cooldown,
                xp_value,
            )) = shapes.get_mut(candidate.entity)
            else {
                continue;
            };
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

            if protection.active() {
                continue;
            }

            if shape_health.0 <= 0.0 {
                commands.entity(shape_entity).despawn();
                continue;
            }
            let body_damage =
                crate::tank::body_damage(bot_upgrades.0.body_damage(), &bot_evolution.0)
                    * body_damage_multiplier(bot_evolution.0.passive(), speed_fraction);
            let damage_to_shape =
                crate::tank::contact_damage_for_step(body_damage, time.delta_secs());
            let damage_to_bot = passive_runtime.incoming_contact_damage(
                bot_evolution.0.passive(),
                crate::tank::contact_damage_for_step(shape_damage.0, time.delta_secs()),
                speed_fraction,
            );
            let shape_killed = apply_shape_damage(&mut shape_health, damage_to_shape);
            let bot_killed = apply_enemy_bot_damage(&mut bot_health, damage_to_bot);
            recent_damage.record(damage_to_bot);
            shape_contact_cooldown.0 = 0.0;
            damage_cooldown.0 = 0.0;
            if shape_killed {
                commands.entity(shape_entity).despawn();
                award_enemy_bot_xp(
                    xp_value.0,
                    &mut bot_xp,
                    &mut bot_level,
                    &mut bot_upgrades,
                    &mut bot_evolution,
                    &mut bot_health,
                    playstyle,
                    &mut stats,
                    &mut rng,
                );
            }
            if bot_killed {
                finish_enemy_bot_death(
                    bot_entity,
                    &mut visibility,
                    &mut respawn_timer,
                    &mut deaths,
                    None,
                );
                break;
            }
        }
    }
}

pub fn check_enemy_bot_enemy_bot_collisions(
    time: Res<Time<Fixed>>,
    grid: Res<crate::spatial::SpatialGrid>,
    mut deaths: ResMut<CombatDeathQueue>,
    mut bots: Query<
        (
            Entity,
            &mut Transform,
            &mut EnemyBotVelocity,
            &mut EnemyBotHealth,
            &mut EnemyBotDamageCooldown,
            &EnemyBotUpgrades,
            &EnemyBotEvolution,
            &mut Visibility,
            &mut EnemyBotRespawnTimer,
            &mut EnemyBotBrain,
            &SpawnProtection,
            &mut RecentDamage,
            &EnemyBotMoveVelocity,
            &mut PassiveRuntime,
        ),
        With<EnemyBot>,
    >,
) {
    let dt = time.delta_secs();
    for (candidate_a, candidate_b) in grid.unique_pairs(150.0) {
        let Ok(
            [
                (
                    entity_a,
                    mut transform_a,
                    mut velocity_a,
                    mut health_a,
                    mut cooldown_a,
                    upgrades_a,
                    evolution_a,
                    mut visibility_a,
                    mut respawn_a,
                    mut brain_a,
                    protection_a,
                    mut recent_a,
                    move_velocity_a,
                    mut runtime_a,
                ),
                (
                    entity_b,
                    mut transform_b,
                    mut velocity_b,
                    mut health_b,
                    mut cooldown_b,
                    upgrades_b,
                    evolution_b,
                    mut visibility_b,
                    mut respawn_b,
                    mut brain_b,
                    protection_b,
                    mut recent_b,
                    move_velocity_b,
                    mut runtime_b,
                ),
            ],
        ) = bots.get_many_mut([candidate_a, candidate_b])
        else {
            continue;
        };
        if health_a.current <= 0.0 || health_b.current <= 0.0 {
            continue;
        }
        let radius_a = crate::tank::radius(&evolution_a.0);
        let radius_b = crate::tank::radius(&evolution_b.0);
        let collision_distance = radius_a + radius_b;
        let repulsion_distance = collision_distance * 3.0;
        let delta = transform_a.translation.xy() - transform_b.translation.xy();
        let distance_sq = delta.length_squared();
        if distance_sq >= repulsion_distance * repulsion_distance {
            continue;
        }
        let distance = distance_sq.sqrt();
        let normal = if distance > 0.001 {
            delta / distance
        } else {
            Vec2::X
        };
        if distance < collision_distance {
            let penetration = collision_distance - distance;
            transform_a.translation += (normal * penetration * 0.5).extend(0.0);
            transform_b.translation -= (normal * penetration * 0.5).extend(0.0);
            if !protection_a.active() && !protection_b.active() {
                let speed_a = move_velocity_a.0.length()
                    / (upgrades_a.0.movement_speed() * evolution_a.0.movement_multiplier())
                        .max(1.0);
                let speed_b = move_velocity_b.0.length()
                    / (upgrades_b.0.movement_speed() * evolution_b.0.movement_multiplier())
                        .max(1.0);
                let body_a = crate::tank::body_damage(upgrades_a.0.body_damage(), &evolution_a.0)
                    * body_damage_multiplier(evolution_a.0.passive(), speed_a);
                let body_b = crate::tank::body_damage(upgrades_b.0.body_damage(), &evolution_b.0)
                    * body_damage_multiplier(evolution_b.0.passive(), speed_b);
                let damage_a = runtime_a.incoming_contact_damage(
                    evolution_a.0.passive(),
                    crate::tank::contact_damage_for_step(body_b, dt),
                    speed_a,
                );
                let damage_b = runtime_b.incoming_contact_damage(
                    evolution_b.0.passive(),
                    crate::tank::contact_damage_for_step(body_a, dt),
                    speed_b,
                );
                brain_a.note_attacker(entity_b);
                brain_b.note_attacker(entity_a);
                recent_a.record(damage_a);
                recent_b.record(damage_b);
                let killed_a = apply_enemy_bot_damage(&mut health_a, damage_a);
                let killed_b = apply_enemy_bot_damage(&mut health_b, damage_b);
                if killed_a {
                    finish_enemy_bot_death(
                        entity_a,
                        &mut visibility_a,
                        &mut respawn_a,
                        &mut deaths,
                        Some(CombatantId::EnemyBot(entity_b)),
                    );
                }
                if killed_b {
                    finish_enemy_bot_death(
                        entity_b,
                        &mut visibility_b,
                        &mut respawn_b,
                        &mut deaths,
                        Some(CombatantId::EnemyBot(entity_a)),
                    );
                }
            }
            cooldown_a.0 = 0.0;
            cooldown_b.0 = 0.0;
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
    grid: Res<crate::spatial::SpatialGrid>,
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

    for (candidate_a, candidate_b) in grid.unique_pairs(collision_distance) {
        let Ok(
            [
                (entity_a, mut transform_a, mut health_a, mut velocity_a, mut cooldown_a),
                (entity_b, mut transform_b, mut health_b, mut velocity_b, mut cooldown_b),
            ],
        ) = shapes.get_many_mut([candidate_a, candidate_b])
        else {
            continue;
        };
        if health_a.0 <= 0.0 || health_b.0 <= 0.0 {
            if health_a.0 <= 0.0 {
                push_dead_shape(&mut dead_shapes, entity_a);
            }
            if health_b.0 <= 0.0 {
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

fn apply_shape_damage(health: &mut Health, damage: f32) -> bool {
    let was_alive = health.0 > 0.0;
    health.0 = (health.0 - damage).max(0.0);
    was_alive && health.0 <= 0.0
}

fn apply_player_damage(health: &mut PlayerHealth, damage: f32) -> bool {
    let was_alive = health.current > 0.0;
    health.current = (health.current - damage).max(0.0);
    was_alive && health.current <= 0.0
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
        let mut health = Health(1.0);

        assert!(apply_shape_damage(&mut health, 1.0));
        assert_eq!(health.0, 0.0);
        assert!(!apply_shape_damage(&mut health, 1.0));
        assert_eq!(health.0, 0.0);
    }

    #[test]
    fn splash_falls_off_and_never_exceeds_direct_damage() {
        let center = splash_damage(20.0, 0.0);
        let edge = splash_damage(20.0, SPLASH_RADIUS);
        assert_eq!(center, 13.0);
        assert_eq!(edge, 0.0);
        assert!(splash_damage(20.0, 45.0) < center);
    }
}
