use crate::{
    combat::TANK_KILL_XP,
    constants,
    enemy_bot::{
        EnemyBot, EnemyBotBrain, EnemyBotDamageCooldown, EnemyBotEvolution, EnemyBotHealProgress,
        EnemyBotHealth, EnemyBotLevel, EnemyBotMoveVelocity, EnemyBotPlaystyle,
        EnemyBotRespawnTimer, EnemyBotSpawnPosition, EnemyBotTargetKind, EnemyBotTurret,
        EnemyBotUpgrades, EnemyBotVelocity, enemy_bot_barrel_transform,
        random_enemy_bot_spawn_position,
    },
    player::{self, MoveVelocity, Player, PlayerHealth, Velocity},
    projectile::{
        Lifetime, Projectile, ProjectileDamage, ProjectileKnockback, ProjectileOwner,
        ProjectilePenetration, ShootCooldown,
    },
    rng::Rng,
    shape::{Health, Level, Shape, XpValue},
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
    decision_min: f32,
    decision_variance: f32,
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
                decision_min: 0.34,
                decision_variance: 0.30,
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
                decision_min: 0.52,
                decision_variance: 0.38,
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
                decision_min: 0.62,
                decision_variance: 0.38,
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
                decision_min: 0.45,
                decision_variance: 0.30,
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
                decision_min: 0.24,
                decision_variance: 0.24,
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
}

pub(crate) struct ProjectileAssets {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
}

pub fn respawn_enemy_bots(
    time: Res<Time>,
    mut rng: ResMut<Rng>,
    player: Query<&Transform, (With<Player>, Without<EnemyBot>)>,
    mut bots: ParamSet<(
        Query<(Entity, &Transform, &EnemyBotHealth), With<EnemyBot>>,
        Query<
            (
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
            ),
            With<EnemyBot>,
        >,
    )>,
) {
    let player_pos = player
        .single()
        .map(|transform| transform.translation.xy())
        .unwrap_or(Vec2::ZERO);
    let mut occupied_positions = bots
        .p0()
        .iter()
        .filter(|(_, _, health)| health.current > 0)
        .map(|(_, transform, _)| transform.translation.xy())
        .collect::<Vec<_>>();

    for (
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
    ) in bots.p1().iter_mut()
    {
        if health.current > 0 || respawn_timer.0 <= 0.0 {
            continue;
        }

        respawn_timer.0 -= time.delta_secs();
        if respawn_timer.0 > 0.0 {
            continue;
        }

        let position = random_enemy_bot_spawn_position(&mut rng, &occupied_positions, player_pos);
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
        brain.reset();
        *visibility = Visibility::Visible;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn enemy_bot_ai_update(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut projectile_assets: Local<Option<ProjectileAssets>>,
    time: Res<Time>,
    mut rng: ResMut<Rng>,
    player_level: Res<Level>,
    mut bots: ParamSet<(
        Query<
            (
                Entity,
                &Transform,
                &EnemyBotHealth,
                &EnemyBotLevel,
                &EnemyBotMoveVelocity,
                &EnemyBotVelocity,
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
                &EnemyBotEvolution,
                &mut ShootCooldown,
                &Children,
                &EnemyBotPlaystyle,
                &EnemyBotLevel,
                &mut EnemyBotBrain,
            ),
            (With<EnemyBot>, Without<EnemyBotTurret>),
        >,
    )>,
    player: Query<
        (Entity, &Transform, &PlayerHealth, &MoveVelocity, &Velocity),
        (With<Player>, Without<EnemyBot>, Without<EnemyBotTurret>),
    >,
    shapes: Query<
        (Entity, &Transform, &Health, &XpValue),
        (With<Shape>, Without<EnemyBot>, Without<EnemyBotTurret>),
    >,
    projectiles: Query<
        (&Transform, &Velocity, &ProjectileOwner),
        (With<Projectile>, Without<EnemyBot>, Without<EnemyBotTurret>),
    >,
    mut turrets: Query<(&mut Transform, &mut Visibility, &EnemyBotTurret), Without<EnemyBot>>,
) {
    let assets = projectile_assets.get_or_insert_with(|| ProjectileAssets {
        mesh: meshes.add(Circle::new(constants::PROJECTILE_RADIUS)),
        material: materials.add(Color::srgba(
            constants::PROJECTILE_COLOR[0],
            constants::PROJECTILE_COLOR[1],
            constants::PROJECTILE_COLOR[2],
            constants::PROJECTILE_COLOR[3],
        )),
    });
    let dt = time.delta_secs();
    let half = constants::arena_half_extent() - constants::PLAYER_RADIUS;
    let damping = (1.0 - constants::PLAYER_KNOCKBACK_DAMPING * dt).clamp(0.0, 1.0);

    let mut combat_targets = Vec::new();
    if let Ok((entity, transform, health, move_velocity, knockback_velocity)) = player.single()
        && health.current > 0
    {
        combat_targets.push(TargetSnapshot {
            entity,
            kind: EnemyBotTargetKind::Combatant,
            position: transform.translation.xy(),
            velocity: move_velocity.0 + knockback_velocity.0,
            health_fraction: health.current as f32 / health.max.max(1) as f32,
            current_health: health.current as f32,
            level: player_level.0,
            reward: TANK_KILL_XP,
        });
    }
    combat_targets.extend(
        bots.p0()
            .iter()
            .filter(|(_, _, health, _, _, _)| health.current > 0)
            .map(
                |(entity, transform, health, level, move_velocity, knockback_velocity)| {
                    TargetSnapshot {
                        entity,
                        kind: EnemyBotTargetKind::Combatant,
                        position: transform.translation.xy(),
                        velocity: move_velocity.0 + knockback_velocity.0,
                        health_fraction: health.current as f32 / health.max.max(1) as f32,
                        current_health: health.current as f32,
                        level: level.0,
                        reward: TANK_KILL_XP,
                    }
                },
            ),
    );
    let shape_targets = shapes
        .iter()
        .filter(|(_, _, health, _)| health.0 > 0)
        .map(|(entity, transform, health, xp)| TargetSnapshot {
            entity,
            kind: EnemyBotTargetKind::Shape,
            position: transform.translation.xy(),
            velocity: Vec2::ZERO,
            health_fraction: (health.0 as f32 / 16.0).clamp(0.0, 1.0),
            current_health: health.0 as f32,
            level: 0,
            reward: xp.0,
        })
        .collect::<Vec<_>>();

    for (
        bot_entity,
        mut transform,
        mut move_velocity,
        mut knockback_velocity,
        mut damage_cooldown,
        mut health,
        mut heal_progress,
        upgrades,
        evolution,
        mut shoot_cooldown,
        children,
        playstyle,
        bot_level,
        mut brain,
    ) in bots.p1().iter_mut()
    {
        if health.current == 0 {
            move_velocity.0 = Vec2::ZERO;
            continue;
        }

        let tuning = playstyle.tuning();
        regenerate_enemy_bot_health(&mut health, &mut heal_progress, upgrades, evolution, dt);
        shoot_cooldown.0 -= dt;
        damage_cooldown.0 = (damage_cooldown.0 - dt).max(0.0);
        tick_brain(&mut brain, dt);
        update_flee_state(
            &mut brain.fleeing,
            health.current as f32 / health.max.max(1) as f32,
            tuning.flee_threshold,
        );
        update_strafe(&mut brain, &mut rng);

        let bot_pos = transform.translation.xy();
        let visible_combat = combat_targets
            .iter()
            .copied()
            .filter(|target| {
                target.entity != bot_entity
                    && target.position.distance_squared(bot_pos)
                        <= tuning.view_range * tuning.view_range
            })
            .collect::<Vec<_>>();
        let movement_speed = upgrades.0.movement_speed() * evolution.0.movement_multiplier();
        let damage_per_second = bot_damage_per_second(upgrades, evolution);
        let low_health_threat_radius = (tuning.personal_space
            * LOW_HEALTH_THREAT_RADIUS_MULTIPLIER)
            .max(LOW_HEALTH_MIN_THREAT_RADIUS);
        let nearby_threats = visible_combat
            .iter()
            .copied()
            .filter(|target| bot_pos.distance(target.position) <= low_health_threat_radius)
            .collect::<Vec<_>>();
        let actively_fleeing = brain.fleeing && !nearby_threats.is_empty();
        let defensive_intruder = personal_space_intruder(bot_pos, &visible_combat, tuning);
        let selected_target = current_target(&brain, &visible_combat, &shape_targets);
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

        let selected_target = current_target(&brain, &visible_combat, &shape_targets);
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
                    &shape_targets,
                    &visible_combat,
                    damage_per_second,
                    movement_speed,
                    &mut rng,
                );
                set_brain_target(&mut brain, shape, EnemyBotTargetKind::Shape);
            }
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
                    &shape_targets,
                    &visible_combat,
                    damage_per_second,
                    movement_speed,
                    &mut rng,
                );
                set_brain_target(&mut brain, shape, EnemyBotTargetKind::Shape);
            }
        } else if brain.decision_timer <= 0.0 || selected_target.is_none() {
            let shape = select_farm_target(
                bot_pos,
                &shape_targets,
                &visible_combat,
                damage_per_second,
                movement_speed,
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
            brain.decision_timer =
                tuning.decision_min + random_unit(&mut rng) * tuning.decision_variance;
        }

        let target = current_target(&brain, &visible_combat, &shape_targets);
        let dodge = projectile_avoidance(bot_entity, bot_pos, &projectiles);
        let boundary = boundary_avoidance(bot_pos, half);
        let social = social_avoidance(bot_pos, &visible_combat);
        let desired_velocity = if actively_fleeing {
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
            brain.aim_angle =
                rotate_towards(brain.aim_angle, desired_angle, tuning.turn_speed * dt);
            update_enemy_bot_turrets(children, brain.aim_angle, evolution, &mut turrets);

            if target_distance <= tuning.view_range
                && aim_error <= tuning.aim_tolerance
                && shoot_cooldown.0 <= 0.0
            {
                shoot_cooldown.0 = upgrades.0.reload_cooldown() * evolution.0.reload_multiplier();
                shoot_enemy_bot_projectiles(
                    &mut commands,
                    assets,
                    bot_entity,
                    transform.translation,
                    brain.aim_angle,
                    upgrades,
                    evolution,
                    &mut rng,
                );
            }
        } else {
            brain.aim_angle = normalize_angle(brain.aim_angle + TURRET_IDLE_SPIN_SPEED * dt);
            update_enemy_bot_turrets(children, brain.aim_angle, evolution, &mut turrets);
        }

        transform.translation += (move_velocity.0 + knockback_velocity.0).extend(0.0) * dt;
        transform.translation.x = transform.translation.x.clamp(-half, half);
        transform.translation.y = transform.translation.y.clamp(-half, half);
        knockback_velocity.0 *= damping;
    }
}

fn tick_brain(brain: &mut EnemyBotBrain, dt: f32) {
    brain.decision_timer -= dt;
    brain.strafe_timer -= dt;
    brain.retaliation_timer = (brain.retaliation_timer - dt).max(0.0);
    brain.engagement_timer = (brain.engagement_timer - dt).max(0.0);
    brain.truce_timer = (brain.truce_timer - dt).max(0.0);
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
    let base_damage = upgrades.0.bullet_damage() as f32 * evolution.0.bullet_damage_multiplier();
    let volley_damage = evolution
        .0
        .barrel_specs()
        .iter()
        .map(|spec| base_damage * spec.damage_multiplier)
        .sum::<f32>()
        .max(1.0);
    let cooldown = upgrades.0.reload_cooldown() * evolution.0.reload_multiplier();
    volley_damage / cooldown.max(0.05)
}

fn select_farm_target(
    bot_pos: Vec2,
    shapes: &[TargetSnapshot],
    combatants: &[TargetSnapshot],
    damage_per_second: f32,
    movement_speed: f32,
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
    let power = 0.8 + (target.level as f32 * 0.07).min(1.2);
    let retaliation = if brain.last_attacker == Some(target.entity) {
        1.6
    } else {
        0.0
    };
    proximity * proximity * power + retaliation
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
    let range_steering = if target.kind == EnemyBotTargetKind::Shape {
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
    let steering = escape + dodge * 2.4 + boundary * 1.8;
    steering.normalize_or_zero() * movement_speed * 1.08
}

fn projectile_avoidance(
    bot_entity: Entity,
    bot_pos: Vec2,
    projectiles: &Query<
        (&Transform, &Velocity, &ProjectileOwner),
        (With<Projectile>, Without<EnemyBot>, Without<EnemyBotTurret>),
    >,
) -> Vec2 {
    let mut avoidance = Vec2::ZERO;
    for (transform, velocity, owner) in projectiles.iter() {
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
    let heal_amount = heal_progress.0.floor() as u32;
    if heal_amount == 0 {
        return;
    }
    health.current = (health.current + heal_amount).min(health.max);
    heal_progress.0 -= heal_amount as f32;
}

fn update_enemy_bot_turrets(
    children: &Children,
    aim_angle: f32,
    evolution: &EnemyBotEvolution,
    turrets: &mut Query<(&mut Transform, &mut Visibility, &EnemyBotTurret), Without<EnemyBot>>,
) {
    let specs = evolution.0.barrel_specs();
    for child in children.iter() {
        let Ok((mut transform, mut visibility, turret)) = turrets.get_mut(child) else {
            continue;
        };
        let Some(spec) = specs.get(turret.slot).copied() else {
            *visibility = Visibility::Hidden;
            continue;
        };
        *visibility = Visibility::Visible;
        *transform = enemy_bot_barrel_transform(spec, turret.outline, aim_angle);
    }
}

#[allow(clippy::too_many_arguments)]
fn shoot_enemy_bot_projectiles(
    commands: &mut Commands,
    assets: &ProjectileAssets,
    bot_entity: Entity,
    bot_translation: Vec3,
    aim_angle: f32,
    upgrades: &EnemyBotUpgrades,
    evolution: &EnemyBotEvolution,
    rng: &mut Rng,
) {
    let spread = evolution.0.spread_radians();
    let base_damage = upgrades.0.bullet_damage() as f32 * evolution.0.bullet_damage_multiplier();
    let bullet_speed = upgrades.0.bullet_speed() * evolution.0.bullet_speed_multiplier();
    let lifetime = constants::PROJECTILE_LIFETIME * evolution.0.projectile_lifetime_multiplier();
    let knockback = evolution.0.bullet_knockback_multiplier();

    for spec in evolution.0.barrel_specs() {
        let jitter = if spread > 0.0 {
            (random_unit(rng) * 2.0 - 1.0) * spread
        } else {
            0.0
        };
        let shot_angle = aim_angle + spec.angle_offset + jitter;
        let direction = Vec2::from_angle(shot_angle);
        let right = Vec2::new(direction.y, -direction.x);
        let spawn_pos = bot_translation
            + (direction * player::muzzle_projectile_distance(spec.length)
                + right * spec.lateral_offset)
                .extend(1.0);
        let damage = (base_damage * spec.damage_multiplier).round().max(1.0) as u32;

        commands.spawn((
            Projectile,
            ProjectileOwner::EnemyBot(bot_entity),
            Lifetime(lifetime),
            ProjectileDamage(damage),
            ProjectilePenetration(upgrades.0.bullet_penetration()),
            ProjectileKnockback(knockback),
            Mesh2d(assets.mesh.clone()),
            MeshMaterial2d(assets.material.clone()),
            Transform::from_translation(spawn_pos),
            Velocity(direction * bullet_speed),
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
    fn full_health_combat_is_rejected_when_farming_is_more_efficient() {
        let combatant = TargetSnapshot {
            entity: Entity::from_raw_u32(1).unwrap(),
            kind: EnemyBotTargetKind::Combatant,
            position: Vec2::new(200.0, 0.0),
            velocity: Vec2::ZERO,
            health_fraction: 1.0,
            current_health: 50.0,
            level: 1,
            reward: TANK_KILL_XP,
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
}
