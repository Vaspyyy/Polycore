use crate::{
    combat::LifeGeneration,
    constants,
    dominance::{DominanceState, LeaderChallenger},
    enemy_bot::{
        EnemyBot, EnemyBotBrain, EnemyBotDamageCooldown, EnemyBotEvolution, EnemyBotHealProgress,
        EnemyBotHealth, EnemyBotLevel, EnemyBotMoveVelocity, EnemyBotPlaystyle,
        EnemyBotRespawnTimer, EnemyBotSpawnPosition, EnemyBotTargetKind, EnemyBotTurret,
        EnemyBotUpgrades, EnemyBotVelocity, EnemyBotXp, enemy_bot_barrel_transform,
    },
    palette::{BotPalette, PaletteMaterials},
    passive::{PassiveRuntime, ShotAdjustments},
    player::{self, MoveVelocity, Player, PlayerHealth, Velocity},
    projectile::{
        Lifetime, Projectile, ProjectileAssets, ProjectileDamage, ProjectileEvolution,
        ProjectileGeneration, ProjectileHitHistory, ProjectileKnockback, ProjectileOwner,
        ProjectilePenetration, ProjectileRadius, ProjectileRear, ProjectileSplashReady,
        ProjectileTravel, ShootCooldown,
    },
    rng::Rng,
    shape::{Health, Level, MaxHealth, Shape, XpValue},
    spatial::{SpatialGrid, SpatialKind},
    tank::{ProjectileCorridor, RecentDamage, SpawnProtection},
};
use bevy::prelude::*;

const TURRET_IDLE_SPIN_SPEED: f32 = 0.45;
const FLEE_RECOVERY_MARGIN: f32 = 0.18;
const PROJECTILE_DODGE_TIME: f32 = 0.65;
const PROJECTILE_DODGE_RADIUS: f32 = 90.0;
const ARENA_AVOIDANCE_MARGIN: f32 = 150.0;
const LOW_HEALTH_THREAT_RADIUS_MULTIPLIER: f32 = 1.35;
const LOW_HEALTH_MIN_THREAT_RADIUS: f32 = 360.0;
const SHAPE_FARM_DISTANCE: f32 = 260.0;
const SHAPE_FARM_RETREAT_DISTANCE: f32 = 155.0;
const STRATEGIC_DECISION_INTERVAL: f32 = 0.125;

#[derive(Clone, Copy)]
struct PlaystyleTuning {
    preferred_range: f32,
    retreat_range: f32,
    view_range: f32,
    strafe_weight: f32,
    flee_threshold: f32,
    turn_speed: f32,
    aim_tolerance: f32,
    lead_factor: f32,
    combat_threshold: f32,
    pursuit_leash: f32,
    personal_space: f32,
    engagement_min: f32,
    engagement_variance: f32,
    truce_min: f32,
    truce_variance: f32,
}

impl EnemyBotPlaystyle {
    fn tuning(self) -> PlaystyleTuning {
        match self {
            Self::Brawler => PlaystyleTuning {
                preferred_range: 175.0,
                retreat_range: 85.0,
                view_range: 650.0,
                strafe_weight: 0.38,
                flee_threshold: 0.28,
                turn_speed: 5.8,
                aim_tolerance: 0.20,
                lead_factor: 0.45,
                combat_threshold: 1.10,
                pursuit_leash: 430.0,
                personal_space: 270.0,
                engagement_min: 2.0,
                engagement_variance: 1.6,
                truce_min: 4.0,
                truce_variance: 3.0,
            },
            Self::Sharpshooter => PlaystyleTuning {
                preferred_range: 485.0,
                retreat_range: 365.0,
                view_range: 900.0,
                strafe_weight: 0.65,
                flee_threshold: 0.42,
                turn_speed: 4.4,
                aim_tolerance: 0.065,
                lead_factor: 1.0,
                combat_threshold: 1.35,
                pursuit_leash: 610.0,
                personal_space: 420.0,
                engagement_min: 2.4,
                engagement_variance: 1.8,
                truce_min: 5.5,
                truce_variance: 3.5,
            },
            Self::Juggernaut => PlaystyleTuning {
                preferred_range: 35.0,
                retreat_range: 0.0,
                view_range: 680.0,
                strafe_weight: 0.08,
                flee_threshold: 0.20,
                turn_speed: 4.0,
                aim_tolerance: 0.24,
                lead_factor: 0.30,
                combat_threshold: 1.05,
                pursuit_leash: 390.0,
                personal_space: 235.0,
                engagement_min: 2.2,
                engagement_variance: 1.8,
                truce_min: 4.0,
                truce_variance: 2.5,
            },
            Self::Sentinel => PlaystyleTuning {
                preferred_range: 320.0,
                retreat_range: 225.0,
                view_range: 740.0,
                strafe_weight: 0.48,
                flee_threshold: 0.55,
                turn_speed: 4.8,
                aim_tolerance: 0.11,
                lead_factor: 0.72,
                combat_threshold: 1.60,
                pursuit_leash: 450.0,
                personal_space: 340.0,
                engagement_min: 1.5,
                engagement_variance: 1.2,
                truce_min: 7.0,
                truce_variance: 4.0,
            },
            Self::Skirmisher => PlaystyleTuning {
                preferred_range: 255.0,
                retreat_range: 145.0,
                view_range: 800.0,
                strafe_weight: 0.95,
                flee_threshold: 0.36,
                turn_speed: 6.8,
                aim_tolerance: 0.13,
                lead_factor: 0.82,
                combat_threshold: 1.25,
                pursuit_leash: 500.0,
                personal_space: 310.0,
                engagement_min: 1.8,
                engagement_variance: 1.5,
                truce_min: 5.0,
                truce_variance: 3.0,
            },
        }
    }
}

#[derive(Clone, Copy)]
struct TargetSnapshot {
    entity: Entity,
    kind: EnemyBotTargetKind,
    position: Vec2,
    velocity: Vec2,
    health_fraction: f32,
    current_health: f32,
    level: u32,
    reward: u32,
    dps: f32,
    effective_health: f32,
    evolution_power: f32,
    recent_damage: f32,
    is_leader: bool,
    is_hotspot: bool,
}

#[allow(clippy::type_complexity)]
pub fn respawn_enemy_bots(
    time: Res<Time>,
    mut rng: ResMut<Rng>,
    player: Query<(&Transform, &PlayerHealth), (With<Player>, Without<EnemyBot>)>,
    shapes: Query<&Transform, (With<Shape>, Without<EnemyBot>, Without<Player>)>,
    projectiles: Query<
        (&Transform, &Velocity),
        (With<Projectile>, Without<EnemyBot>, Without<Player>),
    >,
    mut bots: ParamSet<(
        Query<(Entity, &Transform, &EnemyBotHealth), With<EnemyBot>>,
        Query<
            (
                Entity,
                &mut Transform,
                &mut Visibility,
                &mut EnemyBotHealth,
                &mut EnemyBotRespawnTimer,
                &mut EnemyBotVelocity,
                &mut EnemyBotMoveVelocity,
                &mut EnemyBotDamageCooldown,
                &mut EnemyBotHealProgress,
                &mut ShootCooldown,
                &mut EnemyBotBrain,
                &mut EnemyBotSpawnPosition,
                &mut LifeGeneration,
            ),
            With<EnemyBot>,
        >,
        Query<
            (
                &mut EnemyBotHealth,
                &mut EnemyBotXp,
                &mut EnemyBotLevel,
                &mut EnemyBotUpgrades,
                &mut EnemyBotEvolution,
                &mut crate::combat::CombatStats,
                &mut SpawnProtection,
                &mut PassiveRuntime,
                &mut crate::ability::ActiveAbilityState,
            ),
            With<EnemyBot>,
        >,
    )>,
) {
    let needs_respawn = bots
        .p1()
        .iter()
        .any(|(_, _, _, health, timer, ..)| health.current <= 0.0 && timer.0 > 0.0);
    if !needs_respawn {
        return;
    }

    let player_pos = player
        .single()
        .map_or(Vec2::ZERO, |(transform, _)| transform.translation.xy());
    let mut occupied_positions = bots
        .p0()
        .iter()
        .filter(|(_, _, health)| health.current > 0.0)
        .map(|(_, transform, _)| transform.translation.xy())
        .collect::<Vec<_>>();
    if player
        .single()
        .is_ok_and(|(_, health)| health.current > 0.0)
    {
        occupied_positions.push(player_pos);
    }
    let shape_positions = shapes
        .iter()
        .map(|transform| transform.translation.xy())
        .collect::<Vec<_>>();
    let corridors = projectiles
        .iter()
        .map(|(transform, velocity)| ProjectileCorridor {
            start: transform.translation.xy(),
            end: transform.translation.xy() + velocity.0,
        })
        .collect::<Vec<_>>();

    let mut respawned = Vec::new();
    for (
        bot_entity,
        mut transform,
        mut visibility,
        mut health,
        mut respawn_timer,
        mut velocity,
        mut move_velocity,
        mut damage_cooldown,
        mut heal_progress,
        mut shoot_cooldown,
        mut brain,
        mut spawn_position,
        mut generation,
    ) in bots.p1().iter_mut()
    {
        if health.current > 0.0 || respawn_timer.0 <= 0.0 {
            continue;
        }

        respawn_timer.0 -= time.delta_secs();
        if respawn_timer.0 > 0.0 {
            continue;
        }

        let position =
            crate::tank::safe_spawn(&mut rng, &occupied_positions, &shape_positions, &corridors);
        occupied_positions.push(position);
        spawn_position.0 = position;
        transform.translation = position.extend(0.0);
        transform.rotation = Quat::IDENTITY;
        health.current = health.max;
        velocity.0 = Vec2::ZERO;
        move_velocity.0 = Vec2::ZERO;
        damage_cooldown.0 = 0.0;
        heal_progress.0 = 0.0;
        shoot_cooldown.0 = 0.35;
        brain.reset_for_respawn();
        brain.decision_timer = rng.range_f32(0.0, 0.125);
        brain.truce_timer = 2.5;
        generation.0 = generation.0.wrapping_add(1);
        *visibility = Visibility::Visible;
        respawned.push(bot_entity);
    }

    let mut progression_query = bots.p2();
    for entity in respawned {
        let Ok((
            mut health,
            mut xp,
            mut level,
            mut upgrades,
            mut evolution,
            mut stats,
            mut protection,
            mut passive_runtime,
            mut ability_state,
        )) = progression_query.get_mut(entity)
        else {
            continue;
        };
        xp.0 = 0;
        level.0 = 1;
        upgrades.0.reset();
        evolution.0.reset();
        health.max = crate::enemy_bot::enemy_bot_max_health(&upgrades.0, &evolution.0);
        health.current = health.max;
        crate::combat::reset_life_stats(&mut stats);
        protection.remaining = constants::SPAWN_PROTECTION_SECS;
        passive_runtime.reset_for_life();
        ability_state.reset_for_life();
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn enemy_bot_ai_update(
    mut commands: Commands,
    projectile_assets: Res<ProjectileAssets>,
    palette_materials: Res<PaletteMaterials>,
    time: Res<Time>,
    mut rng: ResMut<Rng>,
    player_level: Res<Level>,
    player_upgrades: Res<crate::hud::UpgradeState>,
    player_evolution: Res<crate::evolution::EvolutionState>,
    grid: Res<SpatialGrid>,
    dominance: Res<DominanceState>,
    mut bots: ParamSet<(
        Query<
            (
                Entity,
                &Transform,
                &EnemyBotHealth,
                &EnemyBotLevel,
                &EnemyBotMoveVelocity,
                &EnemyBotVelocity,
                &EnemyBotUpgrades,
                &mut EnemyBotEvolution,
                &crate::combat::CombatStats,
                &RecentDamage,
            ),
            (With<EnemyBot>, Without<EnemyBotTurret>),
        >,
        Query<
            (
                Entity,
                &mut Transform,
                &mut EnemyBotMoveVelocity,
                &mut EnemyBotVelocity,
                &mut EnemyBotDamageCooldown,
                &mut EnemyBotHealth,
                &mut EnemyBotHealProgress,
                &EnemyBotUpgrades,
                &mut EnemyBotEvolution,
                &mut ShootCooldown,
                &EnemyBotPlaystyle,
                &EnemyBotLevel,
                &mut EnemyBotBrain,
                &mut SpawnProtection,
                &LifeGeneration,
            ),
            (With<EnemyBot>, Without<EnemyBotTurret>),
        >,
    )>,
    player: Query<
        (
            Entity,
            &Transform,
            &PlayerHealth,
            &MoveVelocity,
            &Velocity,
            &crate::combat::CombatStats,
            &RecentDamage,
        ),
        (With<Player>, Without<EnemyBot>, Without<EnemyBotTurret>),
    >,
    shapes: Query<
        (
            Entity,
            &Transform,
            &Health,
            &MaxHealth,
            &XpValue,
            Option<&crate::hotspot::HotspotShape>,
        ),
        (With<Shape>, Without<EnemyBot>, Without<EnemyBotTurret>),
    >,
    projectiles: Query<
        (Entity, &Transform, &Velocity, &ProjectileOwner),
        (With<Projectile>, Without<EnemyBot>, Without<EnemyBotTurret>),
    >,
    mut bot_context: Query<
        (
            &BotPalette,
            Option<&LeaderChallenger>,
            &mut PassiveRuntime,
            &mut crate::ability::ActiveAbilityState,
            &crate::ability::Slowed,
        ),
        With<EnemyBot>,
    >,
    mut turrets: Query<(&mut Transform, &mut Visibility, &EnemyBotTurret), Without<EnemyBot>>,
) {
    let dt = time.delta_secs();
    let damping = (1.0 - constants::PLAYER_KNOCKBACK_DAMPING * dt).clamp(0.0, 1.0);

    let mut combat_targets = Vec::new();
    if let Ok((entity, transform, health, move_velocity, knockback_velocity, stats, recent)) =
        player.single()
        && health.current > 0.0
    {
        combat_targets.push(TargetSnapshot {
            entity,
            kind: EnemyBotTargetKind::Combatant,
            position: transform.translation.xy(),
            velocity: move_velocity.0 + knockback_velocity.0,
            health_fraction: health.current / health.max.max(1.0),
            current_health: health.current,
            level: player_level.0,
            reward: crate::combat::kill_xp(stats.life_score),
            dps: tank_damage_per_second(&player_upgrades, &player_evolution),
            effective_health: health.current,
            evolution_power: evolution_power(&player_evolution),
            recent_damage: recent.amount,
            is_leader: dominance.leader == Some(entity),
            is_hotspot: false,
        });
    }
    combat_targets.extend(
        bots.p0()
            .iter()
            .filter(|(_, _, health, ..)| health.current > 0.0)
            .map(
                |(
                    entity,
                    transform,
                    health,
                    level,
                    move_velocity,
                    knockback_velocity,
                    upgrades,
                    evolution,
                    stats,
                    recent,
                )| {
                    TargetSnapshot {
                        entity,
                        kind: EnemyBotTargetKind::Combatant,
                        position: transform.translation.xy(),
                        velocity: move_velocity.0 + knockback_velocity.0,
                        health_fraction: health.current / health.max.max(1.0),
                        current_health: health.current,
                        level: level.0,
                        reward: crate::combat::kill_xp(stats.life_score),
                        dps: bot_damage_per_second(upgrades, evolution),
                        effective_health: health.current,
                        evolution_power: evolution_power(&evolution.0),
                        recent_damage: recent.amount,
                        is_leader: dominance.leader == Some(entity),
                        is_hotspot: false,
                    }
                },
            ),
    );
    let shape_targets = shapes
        .iter()
        .filter(|(_, _, health, _, _, _)| health.0 > 0.0)
        .map(
            |(entity, transform, health, max_health, xp, hotspot)| TargetSnapshot {
                entity,
                kind: EnemyBotTargetKind::Shape,
                position: transform.translation.xy(),
                velocity: Vec2::ZERO,
                health_fraction: (health.0 / max_health.0.max(1.0)).clamp(0.0, 1.0),
                current_health: health.0,
                level: 0,
                reward: xp.0,
                dps: 0.0,
                effective_health: health.0,
                evolution_power: 0.0,
                recent_damage: 0.0,
                is_leader: false,
                is_hotspot: hotspot.is_some_and(|marker| !marker.expired),
            },
        )
        .collect::<Vec<_>>();

    let mut nearby_entries = Vec::new();

    for (
        bot_entity,
        mut transform,
        mut move_velocity,
        mut knockback_velocity,
        mut damage_cooldown,
        mut health,
        mut heal_progress,
        upgrades,
        mut evolution,
        mut shoot_cooldown,
        playstyle,
        bot_level,
        mut brain,
        mut protection,
        generation,
    ) in bots.p1().iter_mut()
    {
        if health.current <= 0.0 {
            move_velocity.0 = Vec2::ZERO;
            continue;
        }

        let tuning = playstyle.tuning();
        regenerate_enemy_bot_health(&mut health, &mut heal_progress, upgrades, &evolution, dt);
        shoot_cooldown.0 -= dt;
        damage_cooldown.0 = (damage_cooldown.0 - dt).max(0.0);
        tick_brain(&mut brain, dt);
        if bot_level.0 >= 30 && evolution.0.current_kind.is_advanced() {
            if !brain.capstone_pending {
                brain.capstone_pending = true;
                brain.capstone_confirmation = capstone_confirmation_delay(&mut rng);
            } else if brain.capstone_confirmation <= 0.0
                && let Some(capstone) = crate::evolution::capstone_kind(evolution.0.current_kind)
            {
                evolution.0.choose_kind_for_level(bot_level.0, capstone);
                brain.capstone_pending = false;
            }
        }
        update_flee_state(
            &mut brain.fleeing,
            health.current / health.max.max(1.0),
            tuning.flee_threshold,
        );
        update_strafe(&mut brain, &mut rng);

        let bot_pos = transform.translation.xy();
        grid.nearby_into(bot_pos, tuning.view_range.max(1_000.0), &mut nearby_entries);
        let mut visible_combat = combat_targets
            .iter()
            .copied()
            .filter(|target| {
                target.entity != bot_entity
                    && nearby_entries.iter().any(|entry| {
                        entry.entity == target.entity && entry.kind == SpatialKind::Tank
                    })
                    && target.position.distance_squared(bot_pos)
                        <= tuning.view_range * tuning.view_range
            })
            .collect::<Vec<_>>();
        let challenge_target = bot_context
            .get(bot_entity)
            .ok()
            .and_then(|(_, challenge, _, _, _)| challenge)
            .and_then(|challenge| {
                combat_targets
                    .iter()
                    .copied()
                    .find(|target| target.entity == challenge.target)
            });
        if let Some(target) = challenge_target
            && !visible_combat
                .iter()
                .any(|visible| visible.entity == target.entity)
        {
            visible_combat.push(target);
        }
        let visible_shapes = shape_targets
            .iter()
            .copied()
            .filter(|target| {
                nearby_entries
                    .iter()
                    .any(|entry| entry.entity == target.entity && entry.kind == SpatialKind::Shape)
            })
            .collect::<Vec<_>>();
        let (passive_movement, ability_movement, forced_forward, limited_turning) =
            bot_context.get(bot_entity).map_or(
                (1.0, 1.0, false, false),
                |(_, _, runtime, ability, slowed)| {
                    (
                        crate::passive::movement_multiplier(runtime, evolution.0.current_kind),
                        ability.movement_multiplier() * slowed.movement_multiplier(),
                        ability.forces_forward_movement(),
                        ability.limited_turning(),
                    )
                },
            );
        let movement_speed = upgrades.0.movement_speed()
            * evolution.0.movement_multiplier()
            * passive_movement
            * ability_movement;
        let damage_per_second = bot_damage_per_second(upgrades, &evolution);
        let low_health_threat_radius = (tuning.personal_space
            * LOW_HEALTH_THREAT_RADIUS_MULTIPLIER)
            .max(LOW_HEALTH_MIN_THREAT_RADIUS);
        let nearby_threats = visible_combat
            .iter()
            .copied()
            .filter(|target| bot_pos.distance(target.position) <= low_health_threat_radius)
            .collect::<Vec<_>>();
        let health_fraction = health.current / health.max.max(1.0);
        let hotspot_interest = playstyle.hotspot_interest()
            * if health_fraction < 0.45 { 0.65 } else { 1.0 }
            * if bot_level.0 < 30 { 1.25 } else { 0.92 }
            * if nearby_threats.is_empty() { 1.0 } else { 0.72 }
            * if dominance.leader == Some(bot_entity) {
                0.82
            } else {
                1.0
            };
        let own_power =
            damage_per_second + health.current * 0.25 + evolution_power(&evolution.0) * 10.0;
        let assessed_flee = nearby_threats.iter().any(|target| {
            should_flee(
                health.current / health.max.max(1.0),
                tuning.flee_threshold,
                own_power,
                *target,
            )
        });
        let actively_fleeing = (brain.fleeing || assessed_flee) && !nearby_threats.is_empty();
        let is_ram_build =
            evolution.0.current_kind.base() == crate::evolution::EvolutionKind::RamCore;
        let defensive_intruder = if is_ram_build {
            None
        } else {
            personal_space_intruder(bot_pos, &visible_combat, tuning)
        };
        let selected_target = current_target(&brain, &visible_combat, &visible_shapes);
        let engagement_expired = brain.target_kind == EnemyBotTargetKind::Combatant
            && !actively_fleeing
            && defensive_intruder.is_none()
            && (brain.fleeing
                || brain.engagement_timer <= 0.0
                || selected_target.is_some_and(|target| {
                    bot_pos.distance(target.position) > tuning.pursuit_leash
                }));
        if engagement_expired {
            begin_truce(&mut brain, tuning, &mut rng);
            set_brain_target(&mut brain, None, EnemyBotTargetKind::Shape);
        }

        let selected_target = current_target(&brain, &visible_combat, &visible_shapes);
        if actively_fleeing {
            let target = most_dangerous_target(bot_pos, &nearby_threats, &brain);
            set_brain_target(&mut brain, target, EnemyBotTargetKind::Combatant);
        } else if brain.fleeing {
            if brain.target_kind == EnemyBotTargetKind::Combatant
                || selected_target.is_none()
                || brain.decision_timer <= 0.0
            {
                let shape = select_farm_target(
                    bot_pos,
                    &visible_shapes,
                    &visible_combat,
                    damage_per_second,
                    movement_speed,
                    hotspot_interest,
                    &mut rng,
                );
                set_brain_target(&mut brain, shape, EnemyBotTargetKind::Shape);
            }
        } else if let Some(challenge) = challenge_target {
            brain.truce_timer = 0.0;
            brain.engagement_timer = brain.engagement_timer.max(0.5);
            set_brain_target(&mut brain, Some(challenge), EnemyBotTargetKind::Combatant);
        } else if let Some(intruder) = defensive_intruder {
            brain.engagement_timer = 0.0;
            set_brain_target(&mut brain, Some(intruder), EnemyBotTargetKind::Combatant);
        } else if brain.truce_timer > 0.0 {
            if brain.target_kind == EnemyBotTargetKind::Combatant
                || selected_target.is_none()
                || brain.decision_timer <= 0.0
            {
                let shape = select_farm_target(
                    bot_pos,
                    &visible_shapes,
                    &visible_combat,
                    damage_per_second,
                    movement_speed,
                    hotspot_interest,
                    &mut rng,
                );
                set_brain_target(&mut brain, shape, EnemyBotTargetKind::Shape);
            }
        } else if brain.decision_timer <= 0.0 || selected_target.is_none() {
            let shape = select_farm_target(
                bot_pos,
                &visible_shapes,
                &visible_combat,
                damage_per_second,
                movement_speed,
                hotspot_interest,
                &mut rng,
            );
            let combat = select_worthwhile_combat_target(
                *playstyle,
                bot_pos,
                bot_level.0,
                &visible_combat,
                shape,
                &brain,
                damage_per_second,
                movement_speed,
                &mut rng,
            );
            if let Some(combat) = combat {
                set_brain_target(&mut brain, Some(combat), EnemyBotTargetKind::Combatant);
                if brain.engagement_timer <= 0.0 {
                    brain.engagement_timer =
                        tuning.engagement_min + random_unit(&mut rng) * tuning.engagement_variance;
                }
            } else {
                set_brain_target(&mut brain, shape, EnemyBotTargetKind::Shape);
                begin_truce(&mut brain, tuning, &mut rng);
            }
        }
        if brain.decision_timer <= 0.0 {
            brain.decision_timer = STRATEGIC_DECISION_INTERVAL;
        }

        let target = current_target(&brain, &visible_combat, &visible_shapes);
        let dodge = projectile_avoidance(bot_entity, bot_pos, &nearby_entries, &projectiles);
        let half = constants::arena_half_extent() - crate::tank::radius(&evolution.0);
        let boundary = boundary_avoidance(bot_pos, half);
        let social = social_avoidance(bot_pos, &visible_combat);
        let desired_velocity = if forced_forward {
            Vec2::from_angle(brain.aim_angle) * movement_speed
        } else if actively_fleeing {
            flee_velocity(
                bot_pos,
                &nearby_threats,
                &brain,
                dodge,
                boundary,
                movement_speed,
            )
        } else if let Some(intruder) = defensive_intruder {
            defensive_velocity(
                bot_pos,
                intruder.position,
                dodge,
                boundary,
                social,
                movement_speed,
            )
        } else if let Some(target) = target {
            engage_velocity(
                bot_pos,
                target,
                tuning,
                evolution.0.passive(),
                is_ram_build,
                brain.strafe_direction,
                dodge,
                boundary,
                social
                    * if brain.truce_timer > 0.0 || target.kind == EnemyBotTargetKind::Shape {
                        2.2
                    } else {
                        0.2
                    },
                movement_speed,
            )
        } else {
            (dodge * 1.4 + boundary + social * 2.4).normalize_or_zero() * movement_speed * 0.7
        };
        let acceleration = movement_speed / constants::PLAYER_ACCEL_TIME;
        move_velocity.0 =
            approach_velocity(move_velocity.0, desired_velocity, acceleration * dt * 1.15);

        if let Some(target) = target {
            let target_distance = bot_pos.distance(target.position);
            let bullet_speed = upgrades.0.bullet_speed() * evolution.0.bullet_speed_multiplier();
            let lead_time = (target_distance / bullet_speed.max(1.0)).min(0.9);
            let aim_point = target.position + target.velocity * lead_time * tuning.lead_factor;
            let desired_angle = (aim_point - bot_pos).to_angle();
            let aim_error = angle_delta(brain.aim_angle, desired_angle).abs();
            let turn_speed = if limited_turning {
                1.2
            } else {
                tuning.turn_speed
            };
            brain.aim_angle = rotate_towards(brain.aim_angle, desired_angle, turn_speed * dt);

            let fortifying = evolution.0.passive() == crate::evolution::PassiveKind::Entrenched
                && health.current / health.max.max(1.0) < 0.8
                && bot_context
                    .get(bot_entity)
                    .is_ok_and(|(_, _, runtime, _, _)| runtime.stationary >= 0.75);

            let firing_disabled = bot_context
                .get(bot_entity)
                .is_ok_and(|(_, _, _, ability, _)| ability.firing_disabled());
            if target_distance <= tuning.view_range
                && aim_error <= tuning.aim_tolerance
                && shoot_cooldown.0 <= 0.0
                && !fortifying
                && !firing_disabled
            {
                let mut adjustments = bot_context.get_mut(bot_entity).map_or_else(
                    |_| ShotAdjustments::default(),
                    |(_, _, mut runtime, _, _)| runtime.shot_adjustments(evolution.0.current_kind),
                );
                let primed = bot_context.get_mut(bot_entity).map_or_else(
                    |_| crate::ability::PrimedShot::default(),
                    |(_, _, _, mut ability, _)| {
                        if ability.full_battery() {
                            adjustments.bank = None;
                        }
                        ability.primed_shot()
                    },
                );
                if bot_context
                    .get(bot_entity)
                    .is_ok_and(|(_, _, _, ability, _)| ability.braced())
                {
                    adjustments.speed_multiplier = 1.30;
                    adjustments.spread_multiplier = 0.25;
                }
                shoot_cooldown.0 = upgrades.0.reload_cooldown()
                    * evolution.0.reload_multiplier()
                    * adjustments.cooldown_multiplier
                    * bot_context
                        .get(bot_entity)
                        .map_or(1.0, |(_, _, _, ability, _)| ability.reload_multiplier());
                protection.cancel();
                let palette_index = bot_context
                    .get(bot_entity)
                    .map_or(0, |(palette, _, _, _, _)| palette.0);
                shoot_enemy_bot_projectiles(
                    &mut commands,
                    &projectile_assets,
                    bot_entity,
                    generation.0,
                    transform.translation,
                    brain.aim_angle,
                    upgrades,
                    &evolution,
                    palette_materials.bot(palette_index).projectile.clone(),
                    adjustments,
                    primed,
                    &mut rng,
                );
                if evolution.0.passive() == crate::evolution::PassiveKind::BoosterRecoil {
                    let forward = Vec2::from_angle(brain.aim_angle);
                    let recoil = if evolution.0.current_kind
                        == crate::evolution::EvolutionKind::Afterburner
                    {
                        0.12
                    } else {
                        0.08
                    };
                    move_velocity.0 += forward * upgrades.0.movement_speed() * recoil;
                    move_velocity.0 = move_velocity.0.clamp_length_max(movement_speed * 1.25);
                }
            }
        } else {
            brain.aim_angle = normalize_angle(brain.aim_angle + TURRET_IDLE_SPIN_SPEED * dt);
        }

        transform.rotation = Quat::from_rotation_z(brain.aim_angle - std::f32::consts::FRAC_PI_2);

        transform.translation += (move_velocity.0 + knockback_velocity.0).extend(0.0) * dt;
        transform.translation.x = transform.translation.x.clamp(-half, half);
        transform.translation.y = transform.translation.y.clamp(-half, half);
        update_enemy_bot_turrets(bot_entity, &transform, &evolution, &mut turrets);
        knockback_velocity.0 *= damping;
    }
}

fn tick_brain(brain: &mut EnemyBotBrain, dt: f32) {
    brain.decision_timer -= dt;
    brain.strafe_timer -= dt;
    brain.retaliation_timer = (brain.retaliation_timer - dt).max(0.0);
    brain.engagement_timer = (brain.engagement_timer - dt).max(0.0);
    brain.truce_timer = (brain.truce_timer - dt).max(0.0);
    brain.capstone_confirmation = (brain.capstone_confirmation - dt).max(0.0);
    if brain.retaliation_timer <= 0.0 {
        brain.last_attacker = None;
    }
}

fn update_strafe(brain: &mut EnemyBotBrain, rng: &mut Rng) {
    if brain.strafe_timer > 0.0 {
        return;
    }
    if rng.next(100) < 68 {
        brain.strafe_direction *= -1.0;
    }
    brain.strafe_timer = 0.65 + random_unit(rng) * 1.15;
}

fn update_flee_state(fleeing: &mut bool, health_fraction: f32, threshold: f32) {
    if *fleeing {
        if health_fraction >= (threshold + FLEE_RECOVERY_MARGIN).min(0.95) {
            *fleeing = false;
        }
    } else if health_fraction <= threshold {
        *fleeing = true;
    }
}

fn current_target(
    brain: &EnemyBotBrain,
    combat_targets: &[TargetSnapshot],
    shape_targets: &[TargetSnapshot],
) -> Option<TargetSnapshot> {
    let entity = brain.target?;
    let targets = match brain.target_kind {
        EnemyBotTargetKind::Combatant => combat_targets,
        EnemyBotTargetKind::Shape => shape_targets,
    };
    targets
        .iter()
        .find(|target| target.entity == entity)
        .copied()
}

fn set_brain_target(
    brain: &mut EnemyBotBrain,
    target: Option<TargetSnapshot>,
    fallback_kind: EnemyBotTargetKind,
) {
    brain.target = target.map(|target| target.entity);
    brain.target_kind = target.map(|target| target.kind).unwrap_or(fallback_kind);
}

fn begin_truce(brain: &mut EnemyBotBrain, tuning: PlaystyleTuning, rng: &mut Rng) {
    brain.engagement_timer = 0.0;
    brain.truce_timer = tuning.truce_min + random_unit(rng) * tuning.truce_variance;
    brain.decision_timer = 0.0;
}

fn bot_damage_per_second(upgrades: &EnemyBotUpgrades, evolution: &EnemyBotEvolution) -> f32 {
    tank_damage_per_second(&upgrades.0, &evolution.0)
}

fn tank_damage_per_second(
    upgrades: &crate::hud::UpgradeState,
    evolution: &crate::evolution::EvolutionState,
) -> f32 {
    let base_damage = upgrades.bullet_damage() * evolution.bullet_damage_multiplier();
    let mut volley_damage = evolution
        .barrel_specs()
        .iter()
        .map(|spec| base_damage * spec.damage_multiplier)
        .sum::<f32>()
        .max(1.0);
    if evolution.passive() == crate::evolution::PassiveKind::AlternatingPairs {
        volley_damage *= 0.5;
    }
    let cooldown = upgrades.reload_cooldown() * evolution.reload_multiplier();
    volley_damage / cooldown.max(0.05)
}

fn evolution_power(evolution: &crate::evolution::EvolutionState) -> f32 {
    if evolution.current_kind == crate::evolution::EvolutionKind::Tank {
        1.0
    } else {
        1.35
    }
}

fn combat_power(target: TargetSnapshot) -> f32 {
    target.dps + target.effective_health * 0.25 + target.evolution_power * 10.0
}

fn should_flee(
    health_fraction: f32,
    threshold: f32,
    own_power: f32,
    attacker: TargetSnapshot,
) -> bool {
    health_fraction <= threshold || combat_power(attacker) > own_power * 1.25
}

fn select_farm_target(
    bot_pos: Vec2,
    shapes: &[TargetSnapshot],
    combatants: &[TargetSnapshot],
    damage_per_second: f32,
    movement_speed: f32,
    hotspot_interest: f32,
    rng: &mut Rng,
) -> Option<TargetSnapshot> {
    shapes
        .iter()
        .copied()
        .map(|shape| {
            let bot_distance = bot_pos.distance(shape.position);
            let competitors = combatants
                .iter()
                .filter(|combatant| {
                    combatant.position.distance(shape.position) + 80.0 < bot_distance
                })
                .count() as f32;
            let competition_penalty = 1.0 + competitors * 0.5;
            let variation = 0.90 + random_unit(rng) * 0.20;
            let efficiency = farming_efficiency(bot_pos, shape, damage_per_second, movement_speed)
                * variation
                * if shape.is_hotspot {
                    hotspot_interest
                } else {
                    1.0
                }
                / competition_penalty;
            (shape, efficiency)
        })
        .max_by(|(_, value_a), (_, value_b)| value_a.total_cmp(value_b))
        .map(|(shape, _)| shape)
}

fn farming_efficiency(
    bot_pos: Vec2,
    shape: TargetSnapshot,
    damage_per_second: f32,
    movement_speed: f32,
) -> f32 {
    let travel_time = bot_pos.distance(shape.position) / movement_speed.max(1.0);
    let destroy_time = shape.current_health / damage_per_second.max(0.1);
    shape.reward as f32 / (travel_time + destroy_time).max(0.25)
}

#[allow(clippy::too_many_arguments)]
fn select_worthwhile_combat_target(
    playstyle: EnemyBotPlaystyle,
    bot_pos: Vec2,
    bot_level: u32,
    targets: &[TargetSnapshot],
    farm_target: Option<TargetSnapshot>,
    brain: &EnemyBotBrain,
    damage_per_second: f32,
    movement_speed: f32,
    rng: &mut Rng,
) -> Option<TargetSnapshot> {
    let tuning = playstyle.tuning();
    let farm_value = farm_target
        .map(|shape| farming_efficiency(bot_pos, shape, damage_per_second, movement_speed))
        .unwrap_or(2.0)
        .max(2.0);
    let required_value = farm_value * tuning.combat_threshold;

    targets
        .iter()
        .copied()
        .filter(|target| bot_pos.distance(target.position) <= tuning.pursuit_leash)
        .map(|target| {
            let distance = bot_pos.distance(target.position);
            let travel_distance = (distance - tuning.preferred_range).max(0.0);
            let travel_time = travel_distance / movement_speed.max(1.0);
            let destroy_time = target.current_health / damage_per_second.max(0.1);
            let level_risk = target.level.saturating_sub(bot_level) as f32 * 0.16;
            let health_risk = target.health_fraction * 0.40;
            let retaliation_bonus = if brain.last_attacker == Some(target.entity) {
                1.35
            } else {
                1.0
            };
            let variation = 0.90 + random_unit(rng) * 0.20;
            let value = target.reward as f32 / (travel_time + destroy_time).max(0.4)
                * retaliation_bonus
                * if target.is_leader { 1.25 } else { 1.0 }
                * variation
                / (1.0 + level_risk + health_risk);
            (target, value)
        })
        .filter(|(_, value)| *value >= required_value)
        .max_by(|(_, value_a), (_, value_b)| value_a.total_cmp(value_b))
        .map(|(target, _)| target)
}

fn personal_space_intruder(
    bot_pos: Vec2,
    targets: &[TargetSnapshot],
    tuning: PlaystyleTuning,
) -> Option<TargetSnapshot> {
    targets
        .iter()
        .copied()
        .filter(|target| bot_pos.distance(target.position) <= tuning.personal_space)
        .min_by(|target_a, target_b| {
            bot_pos
                .distance_squared(target_a.position)
                .total_cmp(&bot_pos.distance_squared(target_b.position))
        })
}

fn most_dangerous_target(
    bot_pos: Vec2,
    targets: &[TargetSnapshot],
    brain: &EnemyBotBrain,
) -> Option<TargetSnapshot> {
    targets.iter().copied().max_by(|a, b| {
        threat_score(bot_pos, *a, brain).total_cmp(&threat_score(bot_pos, *b, brain))
    })
}

fn threat_score(bot_pos: Vec2, target: TargetSnapshot, brain: &EnemyBotBrain) -> f32 {
    let distance = bot_pos.distance(target.position);
    let proximity = (1.0 - distance / 900.0).clamp(0.0, 1.0);
    let power = combat_power(target) / 25.0;
    let bounty = target.reward as f32 / crate::combat::MAX_KILL_XP as f32;
    let vulnerability = 1.0 - target.health_fraction;
    let recently_hurt = (target.recent_damage / target.effective_health.max(1.0)).min(1.0);
    let retaliation = if brain.last_attacker == Some(target.entity) {
        1.6
    } else {
        0.0
    };
    proximity * proximity * power + retaliation + bounty * 0.25
        - vulnerability * 0.15
        - recently_hurt * 0.1
}

fn defensive_velocity(
    bot_pos: Vec2,
    intruder_pos: Vec2,
    dodge: Vec2,
    boundary: Vec2,
    social: Vec2,
    movement_speed: f32,
) -> Vec2 {
    let retreat = (bot_pos - intruder_pos).normalize_or_zero();
    let steering = retreat * 1.6 + dodge * 1.8 + boundary * 1.4 + social * 1.5;
    steering.normalize_or_zero() * movement_speed
}

fn engage_velocity(
    bot_pos: Vec2,
    target: TargetSnapshot,
    tuning: PlaystyleTuning,
    passive: crate::evolution::PassiveKind,
    charge_target: bool,
    strafe_direction: f32,
    dodge: Vec2,
    boundary: Vec2,
    social: Vec2,
    movement_speed: f32,
) -> Vec2 {
    let offset = target.position - bot_pos;
    let distance = offset.length();
    let direction = offset.normalize_or_zero();
    let perpendicular = Vec2::new(-direction.y, direction.x) * strafe_direction;
    let range_steering = if charge_target && target.kind == EnemyBotTargetKind::Combatant {
        direction
    } else if target.kind == EnemyBotTargetKind::Shape {
        if distance > SHAPE_FARM_DISTANCE {
            direction
        } else if distance < SHAPE_FARM_RETREAT_DISTANCE {
            -direction
        } else {
            Vec2::ZERO
        }
    } else if distance > tuning.preferred_range + 35.0 {
        direction
    } else if distance < tuning.retreat_range {
        -direction
    } else {
        Vec2::ZERO
    };
    let strafe = if target.kind == EnemyBotTargetKind::Combatant {
        perpendicular * tuning.strafe_weight
    } else {
        perpendicular * tuning.strafe_weight * 0.18
    };
    let can_hold = matches!(
        passive,
        crate::evolution::PassiveKind::Stabilized | crate::evolution::PassiveKind::Entrenched
    ) && target.kind == EnemyBotTargetKind::Combatant
        && (distance - tuning.preferred_range).abs() <= 55.0
        && dodge.length_squared() <= 0.04
        && boundary.length_squared() <= 0.04;
    if can_hold {
        return Vec2::ZERO;
    }
    let steering = range_steering + strafe + dodge * 1.35 + boundary * 1.2 + social;
    steering.normalize_or_zero() * movement_speed
}

fn social_avoidance(bot_pos: Vec2, combatants: &[TargetSnapshot]) -> Vec2 {
    const SOCIAL_DISTANCE: f32 = 380.0;

    let mut avoidance = Vec2::ZERO;
    for combatant in combatants {
        let offset = bot_pos - combatant.position;
        let distance = offset.length();
        if distance <= 0.001 || distance >= SOCIAL_DISTANCE {
            continue;
        }
        let strength = 1.0 - distance / SOCIAL_DISTANCE;
        avoidance += offset / distance * strength * strength;
    }
    avoidance
}

fn flee_velocity(
    bot_pos: Vec2,
    targets: &[TargetSnapshot],
    brain: &EnemyBotBrain,
    dodge: Vec2,
    boundary: Vec2,
    movement_speed: f32,
) -> Vec2 {
    let mut escape = Vec2::ZERO;
    for target in targets {
        let away = (bot_pos - target.position).normalize_or_zero();
        escape += away * threat_score(bot_pos, *target, brain).max(0.1);
    }
    if escape.length_squared() <= 0.001
        && let Some(target) = targets.first()
    {
        let away = (bot_pos - target.position).normalize_or_zero();
        escape = Vec2::new(-away.y, away.x) * brain.strafe_direction;
    }
    let steering = escape + dodge * 2.4 + boundary * 1.8;
    steering.normalize_or_zero() * movement_speed * 1.08
}

fn projectile_avoidance(
    bot_entity: Entity,
    bot_pos: Vec2,
    nearby: &[crate::spatial::SpatialEntry],
    projectiles: &Query<
        (Entity, &Transform, &Velocity, &ProjectileOwner),
        (With<Projectile>, Without<EnemyBot>, Without<EnemyBotTurret>),
    >,
) -> Vec2 {
    let mut avoidance = Vec2::ZERO;
    for entry in nearby
        .iter()
        .filter(|entry| entry.kind == SpatialKind::Projectile)
    {
        let Ok((_, transform, velocity, owner)) = projectiles.get(entry.entity) else {
            continue;
        };
        if matches!(*owner, ProjectileOwner::EnemyBot(owner) if owner == bot_entity) {
            continue;
        }
        let speed_sq = velocity.0.length_squared();
        if speed_sq <= 1.0 {
            continue;
        }

        let projectile_pos = transform.translation.xy();
        let to_bot = bot_pos - projectile_pos;
        let time_to_closest = velocity.0.dot(to_bot) / speed_sq;
        if !(0.0..=PROJECTILE_DODGE_TIME).contains(&time_to_closest) {
            continue;
        }
        let closest = projectile_pos + velocity.0 * time_to_closest;
        let miss_distance = bot_pos.distance(closest);
        if miss_distance >= PROJECTILE_DODGE_RADIUS {
            continue;
        }

        let away = (bot_pos - closest)
            .try_normalize()
            .unwrap_or_else(|| Vec2::new(-velocity.0.y, velocity.0.x).normalize_or_zero());
        let urgency = (1.0 - miss_distance / PROJECTILE_DODGE_RADIUS)
            * (1.0 - time_to_closest / PROJECTILE_DODGE_TIME);
        avoidance += away * urgency;
    }
    avoidance
}

fn boundary_avoidance(position: Vec2, half: f32) -> Vec2 {
    let mut steering = Vec2::ZERO;
    let edge_start = half - ARENA_AVOIDANCE_MARGIN;
    if position.x.abs() > edge_start {
        steering.x = -position.x.signum()
            * ((position.x.abs() - edge_start) / ARENA_AVOIDANCE_MARGIN).clamp(0.0, 1.0);
    }
    if position.y.abs() > edge_start {
        steering.y = -position.y.signum()
            * ((position.y.abs() - edge_start) / ARENA_AVOIDANCE_MARGIN).clamp(0.0, 1.0);
    }
    steering
}

fn regenerate_enemy_bot_health(
    health: &mut EnemyBotHealth,
    heal_progress: &mut EnemyBotHealProgress,
    upgrades: &EnemyBotUpgrades,
    evolution: &EnemyBotEvolution,
    dt: f32,
) {
    let regen_per_second = upgrades.0.health_regen_per_second() + evolution.0.health_regen_bonus();
    if regen_per_second <= 0.0 || health.current >= health.max {
        heal_progress.0 = 0.0;
        return;
    }

    heal_progress.0 += regen_per_second * dt;
    let heal_amount = heal_progress.0.floor();
    if heal_amount <= 0.0 {
        return;
    }
    health.current = (health.current + heal_amount).min(health.max);
    heal_progress.0 -= heal_amount;
}

fn update_enemy_bot_turrets(
    owner: Entity,
    owner_transform: &Transform,
    evolution: &EnemyBotEvolution,
    turrets: &mut Query<(&mut Transform, &mut Visibility, &EnemyBotTurret), Without<EnemyBot>>,
) {
    let specs = evolution.0.barrel_specs();
    for (mut transform, mut visibility, turret) in turrets.iter_mut() {
        if turret.owner != owner {
            continue;
        }
        let Some(spec) = specs.get(turret.slot).copied() else {
            *visibility = Visibility::Hidden;
            continue;
        };
        *visibility = Visibility::Visible;
        *transform = owner_transform.mul_transform(enemy_bot_barrel_transform(
            spec,
            turret.outline,
            std::f32::consts::FRAC_PI_2,
            &evolution.0,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn shoot_enemy_bot_projectiles(
    commands: &mut Commands,
    assets: &ProjectileAssets,
    bot_entity: Entity,
    generation: u32,
    bot_translation: Vec3,
    aim_angle: f32,
    upgrades: &EnemyBotUpgrades,
    evolution: &EnemyBotEvolution,
    projectile_material: Handle<ColorMaterial>,
    adjustments: ShotAdjustments,
    primed: crate::ability::PrimedShot,
    rng: &mut Rng,
) {
    let spread = evolution.0.spread_radians() * adjustments.spread_multiplier;
    let base_damage =
        upgrades.0.bullet_damage() * evolution.0.bullet_damage_multiplier() * primed.damage;
    let bullet_speed = upgrades.0.bullet_speed()
        * evolution.0.bullet_speed_multiplier()
        * adjustments.speed_multiplier
        * primed.speed;
    let lifetime = constants::PROJECTILE_LIFETIME
        * evolution.0.projectile_lifetime_multiplier()
        * primed.lifetime;
    let knockback = evolution.0.bullet_knockback_multiplier();
    let projectile_radius = crate::projectile::projectile_radius(&upgrades.0, &evolution.0);
    let projectile_scale = projectile_radius / constants::PROJECTILE_RADIUS;

    for (index, spec) in evolution.0.barrel_specs().iter().enumerate() {
        let bank_index = if evolution.0.current_kind == crate::evolution::EvolutionKind::Fusillade {
            index / 4
        } else {
            index / 2
        };
        if adjustments.bank.is_some_and(|bank| bank_index != bank) {
            continue;
        }
        let jitter = if spread > 0.0 {
            (random_unit(rng) * 2.0 - 1.0) * spread
        } else {
            0.0
        };
        let shot_angle = aim_angle + spec.angle_offset + adjustments.angle_offset + jitter;
        let direction = Vec2::from_angle(shot_angle);
        let right = Vec2::new(direction.y, -direction.x);
        let spawn_pos = bot_translation
            + (direction
                * player::muzzle_projectile_distance(spec.length, &evolution.0, projectile_radius)
                + right * spec.lateral_offset)
                .extend(1.0);
        let damage = (base_damage * spec.damage_multiplier).max(0.1);

        commands
            .spawn((
                Projectile,
                ProjectileOwner::EnemyBot(bot_entity),
                Lifetime(lifetime),
                ProjectileDamage(damage),
                ProjectilePenetration(
                    upgrades
                        .0
                        .bullet_penetration()
                        .saturating_add(evolution.0.penetration_bonus())
                        .saturating_add(primed.penetration),
                ),
                ProjectileKnockback(knockback),
                ProjectileEvolution(evolution.0.current_kind),
                ProjectileTravel::default(),
                ProjectileRear(spec.angle_offset.cos() < 0.0),
                ProjectileSplashReady(
                    evolution.0.passive() == crate::evolution::PassiveKind::Splash,
                ),
                ProjectileHitHistory::default(),
                Mesh2d(assets.mesh.clone()),
                MeshMaterial2d(projectile_material.clone()),
                Transform::from_translation(spawn_pos).with_scale(Vec3::new(
                    projectile_scale,
                    projectile_scale,
                    1.0,
                )),
                Velocity(direction * bullet_speed),
            ))
            .insert((
                ProjectileGeneration(generation),
                ProjectileRadius(projectile_radius),
                crate::ability::ProjectileAbility {
                    clears_projectiles: primed.clears_projectiles,
                    pinning: primed.pinning,
                    ..default()
                },
            ));
    }
}

fn approach_velocity(current: Vec2, target: Vec2, max_delta: f32) -> Vec2 {
    let delta = target - current;
    if delta.length_squared() <= max_delta * max_delta {
        target
    } else {
        current + delta.normalize_or_zero() * max_delta
    }
}

fn rotate_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    normalize_angle(current + angle_delta(current, target).clamp(-max_delta, max_delta))
}

fn angle_delta(current: f32, target: f32) -> f32 {
    normalize_angle(target - current)
}

fn normalize_angle(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

fn random_unit(rng: &mut Rng) -> f32 {
    rng.next(10_000) as f32 / 9_999.0
}

fn capstone_confirmation_delay(rng: &mut Rng) -> f32 {
    0.8 + random_unit(rng) * 0.7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fleeing_uses_hysteresis() {
        let mut fleeing = false;
        update_flee_state(&mut fleeing, 0.25, 0.30);
        assert!(fleeing);

        update_flee_state(&mut fleeing, 0.40, 0.30);
        assert!(fleeing);

        update_flee_state(&mut fleeing, 0.49, 0.30);
        assert!(!fleeing);
    }

    #[test]
    fn capstone_confirmation_is_visible_and_imperfect() {
        let mut rng = Rng::new(31);
        let delays = (0..64)
            .map(|_| capstone_confirmation_delay(&mut rng))
            .collect::<Vec<_>>();
        assert!(delays.iter().all(|delay| (0.8..=1.5).contains(delay)));
        assert!(delays.iter().any(|delay| (*delay - delays[0]).abs() > 0.01));
    }

    #[test]
    fn full_health_combat_is_rejected_when_farming_is_more_efficient() {
        let combatant = TargetSnapshot {
            entity: Entity::from_raw_u32(1).unwrap(),
            kind: EnemyBotTargetKind::Combatant,
            position: Vec2::new(200.0, 0.0),
            velocity: Vec2::ZERO,
            health_fraction: 1.0,
            current_health: 50.0,
            level: 1,
            reward: 100,
            dps: 7.0,
            effective_health: 50.0,
            evolution_power: 1.0,
            recent_damage: 0.0,
            is_leader: false,
            is_hotspot: false,
        };
        let shape = TargetSnapshot {
            entity: Entity::from_raw_u32(2).unwrap(),
            kind: EnemyBotTargetKind::Shape,
            position: Vec2::new(100.0, 0.0),
            velocity: Vec2::ZERO,
            health_fraction: 1.0,
            current_health: 4.0,
            level: 0,
            reward: 20,
            dps: 0.0,
            effective_health: 4.0,
            evolution_power: 0.0,
            recent_damage: 0.0,
            is_leader: false,
            is_hotspot: false,
        };
        let mut rng = Rng::new(7);

        assert!(
            select_worthwhile_combat_target(
                EnemyBotPlaystyle::Brawler,
                Vec2::ZERO,
                1,
                &[combatant],
                Some(shape),
                &EnemyBotBrain::default(),
                1.67,
                300.0,
                &mut rng,
            )
            .is_none()
        );
    }

    #[test]
    fn incoming_damage_interrupts_truce_for_retaliation_window() {
        let mut brain = EnemyBotBrain {
            truce_timer: 8.0,
            ..default()
        };
        let attacker = Entity::from_bits(42);
        brain.note_attacker(attacker);
        assert_eq!(brain.last_attacker, Some(attacker));
        assert_eq!(brain.truce_timer, 0.0);
        assert_eq!(brain.retaliation_timer, 2.5);
    }

    #[test]
    fn flee_assessment_reacts_to_low_health_or_superior_power() {
        let attacker = TargetSnapshot {
            entity: Entity::from_bits(99),
            kind: EnemyBotTargetKind::Combatant,
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            health_fraction: 1.0,
            current_health: 100.0,
            level: 5,
            reward: 200,
            dps: 20.0,
            effective_health: 100.0,
            evolution_power: 1.35,
            recent_damage: 0.0,
            is_leader: false,
            is_hotspot: false,
        };
        assert!(should_flee(0.2, 0.3, 100.0, attacker));
        assert!(should_flee(1.0, 0.3, 20.0, attacker));
        assert!(!should_flee(1.0, 0.3, 100.0, attacker));
        assert_eq!(STRATEGIC_DECISION_INTERVAL, 1.0 / 8.0);
    }

    fn combat_target(entity: u64, position: Vec2) -> TargetSnapshot {
        TargetSnapshot {
            entity: Entity::from_bits(entity),
            kind: EnemyBotTargetKind::Combatant,
            position,
            velocity: Vec2::ZERO,
            health_fraction: 1.0,
            current_health: 50.0,
            level: 1,
            reward: 100,
            dps: 5.0,
            effective_health: 50.0,
            evolution_power: 1.0,
            recent_damage: 0.0,
            is_leader: false,
            is_hotspot: false,
        }
    }

    #[test]
    fn quad_barrel_is_an_upgrade_and_ai_counts_only_the_active_bank() {
        let upgrades = crate::hud::UpgradeState::default();
        let twin = crate::evolution::EvolutionState {
            current_kind: crate::evolution::EvolutionKind::TwinBarrel,
            ..default()
        };
        let quad = crate::evolution::EvolutionState {
            current_kind: crate::evolution::EvolutionKind::QuadBarrel,
            ..default()
        };

        assert!(
            tank_damage_per_second(&upgrades, &quad) > tank_damage_per_second(&upgrades, &twin)
        );
        let base_damage = upgrades.bullet_damage() * quad.bullet_damage_multiplier();
        let active_bank_damage = quad
            .barrel_specs()
            .iter()
            .take(2)
            .map(|barrel| base_damage * barrel.damage_multiplier)
            .sum::<f32>();
        let expected = active_bank_damage / (upgrades.reload_cooldown() * quad.reload_multiplier());
        assert!((tank_damage_per_second(&upgrades, &quad) - expected).abs() < 0.001);
    }

    #[test]
    fn ram_build_charges_and_stationary_build_holds() {
        let charge = engage_velocity(
            Vec2::ZERO,
            combat_target(1, Vec2::new(20.0, 0.0)),
            EnemyBotPlaystyle::Juggernaut.tuning(),
            crate::evolution::PassiveKind::MomentumArmor,
            true,
            1.0,
            Vec2::ZERO,
            Vec2::ZERO,
            Vec2::ZERO,
            300.0,
        );
        assert!(charge.x > 0.0);

        let tuning = EnemyBotPlaystyle::Sentinel.tuning();
        let hold = engage_velocity(
            Vec2::ZERO,
            combat_target(2, Vec2::new(tuning.preferred_range, 0.0)),
            tuning,
            crate::evolution::PassiveKind::Stabilized,
            false,
            1.0,
            Vec2::ZERO,
            Vec2::ZERO,
            Vec2::ZERO,
            300.0,
        );
        assert_eq!(hold, Vec2::ZERO);
    }

    #[test]
    fn symmetric_threats_still_produce_an_escape_direction() {
        let targets = [
            combat_target(1, Vec2::new(-100.0, 0.0)),
            combat_target(2, Vec2::new(100.0, 0.0)),
        ];
        let velocity = flee_velocity(
            Vec2::ZERO,
            &targets,
            &EnemyBotBrain::default(),
            Vec2::ZERO,
            Vec2::ZERO,
            300.0,
        );
        assert!(velocity.length() > 0.0);
    }
}
