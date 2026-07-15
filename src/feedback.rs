use crate::{
    dominance::DominanceState,
    enemy_bot::{EnemyBot, EnemyBotEvolution},
    evolution::{EvolutionKind, EvolutionState, PassiveKind},
    menu::GamePhase,
    passive::PassiveRuntime,
    player::{MoveVelocity, Player},
    tank::RecentDamage,
};
use bevy::prelude::*;
use std::collections::HashMap;

const EFFECT_POOL_SIZE: usize = 96;
const DAMAGE_INDICATOR_SIZE: f32 = 30.0;
const DAMAGE_INDICATOR_RADIUS: f32 = 112.0;

#[derive(Message, Clone, Copy)]
pub struct CombatFeedback {
    pub position: Vec2,
    pub direction: Vec2,
    pub intensity: f32,
    pub is_player: bool,
}

#[derive(Component)]
pub(crate) struct EffectParticle {
    pub(crate) velocity: Vec2,
    pub(crate) remaining: f32,
}

#[derive(Resource, Default)]
pub struct EffectPool {
    entities: Vec<Entity>,
    cursor: usize,
}

#[derive(Resource, Default)]
pub struct FeedbackTracker {
    damage_timers: HashMap<Entity, f32>,
    evolutions: HashMap<Entity, EvolutionKind>,
    last_leader: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct CameraShake {
    pub remaining: f32,
    phase: f32,
}

#[derive(Resource, Default)]
pub(crate) struct DamageIndicatorState {
    remaining: f32,
}

#[derive(Component)]
pub(crate) struct DamageIndicator;

#[derive(Component)]
pub(crate) struct ShieldVisual;

#[derive(Component)]
pub(crate) struct ArmorArcVisual;

#[derive(Component)]
pub(crate) struct PassiveStatusText;

#[derive(Resource, Clone)]
pub(crate) struct PassiveVisualAssets {
    shield_mesh: Handle<Mesh>,
    arc_mesh: Handle<Mesh>,
    shield_material: Handle<ColorMaterial>,
    arc_material: Handle<ColorMaterial>,
}

pub fn setup_feedback(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let particle_mesh = meshes.add(Circle::new(2.6));
    let particle_material = materials.add(Color::srgba(1.0, 0.72, 0.22, 0.92));
    let mut pool = EffectPool::default();
    for _ in 0..EFFECT_POOL_SIZE {
        let entity = commands
            .spawn((
                EffectParticle {
                    velocity: Vec2::ZERO,
                    remaining: 0.0,
                },
                Mesh2d(particle_mesh.clone()),
                MeshMaterial2d(particle_material.clone()),
                Transform::from_xyz(0.0, 0.0, 8.0),
                Visibility::Hidden,
            ))
            .id();
        pool.entities.push(entity);
    }
    commands.insert_resource(pool);
    commands.insert_resource(DamageIndicatorState::default());
    commands.insert_resource(PassiveVisualAssets {
        shield_mesh: meshes.add(Annulus::new(27.0, 29.5)),
        arc_mesh: meshes.add(CircularSector::new(31.0, 50.0_f32.to_radians())),
        shield_material: materials.add(Color::srgba(0.22, 0.78, 1.0, 0.72)),
        arc_material: materials.add(Color::srgba(0.72, 0.88, 1.0, 0.34)),
    });
    commands
        .spawn((
            DamageIndicator,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                width: Val::Px(DAMAGE_INDICATOR_SIZE),
                height: Val::Px(DAMAGE_INDICATOR_SIZE),
                ..default()
            },
            UiTransform::from_translation(Val2::px(
                -DAMAGE_INDICATOR_SIZE / 2.0,
                -DAMAGE_INDICATOR_RADIUS - DAMAGE_INDICATOR_SIZE / 2.0,
            )),
            GlobalZIndex(45),
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .with_children(|indicator| {
            for (index, width) in [20.0, 16.0, 12.0, 8.0, 4.0].into_iter().enumerate() {
                indicator.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px((DAMAGE_INDICATOR_SIZE - width) / 2.0),
                        top: Val::Px(6.0 + index as f32 * 3.0),
                        width: Val::Px(width),
                        height: Val::Px(4.0),
                        border_radius: BorderRadius::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 0.18, 0.16, 0.92)),
                    Pickable::IGNORE,
                ));
            }
        });
    commands.spawn((
        PassiveStatusText,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(18.0),
            bottom: Val::Px(18.0),
            ..default()
        },
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::srgba(0.66, 0.88, 1.0, 0.92)),
        GlobalZIndex(45),
        Visibility::Hidden,
    ));
}

pub fn ensure_passive_visuals(
    mut commands: Commands,
    assets: Res<PassiveVisualAssets>,
    tanks: Query<Entity, Added<PassiveRuntime>>,
) {
    for entity in &tanks {
        commands.entity(entity).with_children(|tank| {
            tank.spawn((
                ShieldVisual,
                Mesh2d(assets.shield_mesh.clone()),
                MeshMaterial2d(assets.shield_material.clone()),
                Transform::from_xyz(0.0, 0.0, 0.35),
                Visibility::Hidden,
            ));
            tank.spawn((
                ArmorArcVisual,
                Mesh2d(assets.arc_mesh.clone()),
                MeshMaterial2d(assets.arc_material.clone()),
                Transform::from_xyz(0.0, 15.0, 0.3),
                Visibility::Hidden,
            ));
        });
    }
}

pub fn update_passive_visuals(
    player_evolution: Res<EvolutionState>,
    players: Query<(&Children, &PassiveRuntime), (With<Player>, Without<EnemyBot>)>,
    bots: Query<
        (&Children, &PassiveRuntime, &EnemyBotEvolution),
        (With<EnemyBot>, Without<Player>),
    >,
    mut visuals: Query<(
        &mut Visibility,
        Option<&ShieldVisual>,
        Option<&ArmorArcVisual>,
    )>,
) {
    if let Ok((children, runtime)) = players.single() {
        set_passive_children(children, runtime, player_evolution.passive(), &mut visuals);
    }
    for (children, runtime, evolution) in &bots {
        set_passive_children(children, runtime, evolution.0.passive(), &mut visuals);
    }
}

pub fn update_passive_status(
    phase: Res<GamePhase>,
    evolution: Res<EvolutionState>,
    player: Query<(&PassiveRuntime, &MoveVelocity), With<Player>>,
    mut status: Query<(&mut Text, &mut Visibility), With<PassiveStatusText>>,
) {
    let Ok((mut text, mut visibility)) = status.single_mut() else {
        return;
    };
    if *phase != GamePhase::Playing {
        *visibility = Visibility::Hidden;
        return;
    }
    let Ok((runtime, velocity)) = player.single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let label = match evolution.passive() {
        PassiveKind::None => String::new(),
        PassiveKind::MinigunSpin => {
            format!("SPIN {:>3}%", (runtime.sustained_fire / 1.5 * 100.0) as u32)
        }
        PassiveKind::Stabilized => format!(
            "STABILIZER {:>3}%",
            (runtime.stationary / 0.75 * 100.0).clamp(0.0, 100.0) as u32
        ),
        PassiveKind::HunterMark => format!("MARK FOLLOW-UPS {}", runtime.follow_up_hits),
        PassiveKind::ConsecutiveHits => format!("NEEDLER STACKS {}/5", runtime.stacks),
        PassiveKind::HitSpeed => format!("HIT BOOST {:.1}s", runtime.speed_boost),
        PassiveKind::Entrenched => format!(
            "ENTRENCH {:>3}%{}",
            (runtime.stationary / 1.25 * 100.0).clamp(0.0, 100.0) as u32,
            if runtime.firing { "  HOLD FIRE" } else { "" }
        ),
        PassiveKind::FrontalShield => {
            format!(
                "FRONTAL SHIELD {:.0}/{:.0}",
                runtime.shield, runtime.shield_max
            )
        }
        PassiveKind::MomentumArmor => format!("MOMENTUM {:.0}", velocity.0.length()),
        PassiveKind::AlternatingPairs => format!(
            "VOLLEY PAIR {}",
            if runtime.volley_phase { "B" } else { "A" }
        ),
        passive => passive_name(passive).to_string(),
    };
    let visible = !label.is_empty();
    if **text != label {
        **text = label;
    }
    *visibility = if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

fn passive_name(passive: PassiveKind) -> &'static str {
    match passive {
        PassiveKind::DistanceDamage => "DISTANCE DAMAGE",
        PassiveKind::Splash => "SPLASH",
        PassiveKind::RearKnockback => "REAR KNOCKBACK",
        PassiveKind::FrontalArmor => "FRONTAL ARMOR",
        PassiveKind::PhasedFan => "PHASED FAN",
        PassiveKind::BoosterRecoil => "BOOSTER RECOIL",
        PassiveKind::None
        | PassiveKind::MinigunSpin
        | PassiveKind::Stabilized
        | PassiveKind::HunterMark
        | PassiveKind::AlternatingPairs
        | PassiveKind::ConsecutiveHits
        | PassiveKind::Entrenched
        | PassiveKind::FrontalShield
        | PassiveKind::MomentumArmor
        | PassiveKind::HitSpeed => "",
    }
}

fn set_passive_children(
    children: &Children,
    runtime: &PassiveRuntime,
    passive: PassiveKind,
    visuals: &mut Query<(
        &mut Visibility,
        Option<&ShieldVisual>,
        Option<&ArmorArcVisual>,
    )>,
) {
    for child in children.iter() {
        let Ok((mut visibility, shield, arc)) = visuals.get_mut(child) else {
            continue;
        };
        let next =
            if (shield.is_some() && passive == PassiveKind::FrontalShield && runtime.shield > 0.0)
                || (arc.is_some() && passive == PassiveKind::FrontalArmor)
            {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        if *visibility != next {
            *visibility = next;
        }
    }
}

pub fn detect_feedback(
    mut tracker: ResMut<FeedbackTracker>,
    player_evolution: Res<EvolutionState>,
    dominance: Res<DominanceState>,
    player: Query<(Entity, &Transform, &RecentDamage), (With<Player>, Without<EnemyBot>)>,
    bots: Query<
        (Entity, &Transform, &RecentDamage, &EnemyBotEvolution),
        (With<EnemyBot>, Without<Player>),
    >,
    transforms: Query<&Transform>,
    mut messages: MessageWriter<CombatFeedback>,
) {
    if let Ok((entity, transform, recent)) = player.single() {
        detect_tank_feedback(
            entity,
            transform,
            recent,
            player_evolution.current_kind,
            true,
            &mut tracker,
            &mut messages,
        );
    }
    for (entity, transform, recent, evolution) in &bots {
        detect_tank_feedback(
            entity,
            transform,
            recent,
            evolution.0.current_kind,
            false,
            &mut tracker,
            &mut messages,
        );
    }
    if tracker.last_leader != dominance.leader {
        if let Some(leader) = dominance.leader
            && let Ok(transform) = transforms.get(leader)
        {
            messages.write(CombatFeedback {
                position: transform.translation.xy(),
                direction: Vec2::Y,
                intensity: 1.8,
                is_player: false,
            });
        }
        tracker.last_leader = dominance.leader;
    }
}

#[allow(clippy::too_many_arguments)]
fn detect_tank_feedback(
    entity: Entity,
    transform: &Transform,
    recent: &RecentDamage,
    evolution: EvolutionKind,
    is_player: bool,
    tracker: &mut FeedbackTracker,
    messages: &mut MessageWriter<CombatFeedback>,
) {
    let previous = tracker
        .damage_timers
        .insert(entity, recent.remaining)
        .unwrap_or(0.0);
    if recent.remaining > previous + 0.01 {
        messages.write(CombatFeedback {
            position: transform.translation.xy(),
            direction: recent.direction,
            intensity: (recent.amount / 12.0).clamp(0.45, 1.5),
            is_player,
        });
    }
    if tracker.evolutions.insert(entity, evolution) != Some(evolution) {
        messages.write(CombatFeedback {
            position: transform.translation.xy(),
            direction: Vec2::Y,
            intensity: if evolution == EvolutionKind::Tank {
                0.0
            } else {
                2.0
            },
            is_player: false,
        });
    }
}

pub fn consume_feedback(
    mut messages: MessageReader<CombatFeedback>,
    mut pool: ResMut<EffectPool>,
    mut effects: Query<
        (&mut Transform, &mut Visibility, &mut EffectParticle),
        Without<DamageIndicator>,
    >,
    mut shake: ResMut<CameraShake>,
    mut indicator: ResMut<DamageIndicatorState>,
    profile: Res<crate::profile::Profile>,
    mut indicator_query: Query<
        (&mut Visibility, &mut UiTransform),
        (With<DamageIndicator>, Without<EffectParticle>),
    >,
) {
    for message in messages.read() {
        if message.intensity <= 0.0 {
            continue;
        }
        let count = combat_particle_count(message.intensity, profile.data.settings.low_power_mode);
        for index in 0..count {
            let entity = pool.entities[pool.cursor % pool.entities.len()];
            pool.cursor = pool.cursor.wrapping_add(1);
            let Ok((mut transform, mut visibility, mut particle)) = effects.get_mut(entity) else {
                continue;
            };
            let angle =
                index as f32 / count as f32 * std::f32::consts::TAU + pool.cursor as f32 * 0.17;
            let direction = Vec2::from_angle(angle);
            transform.translation = message.position.extend(8.0);
            transform.scale = Vec3::splat(0.8 + message.intensity * 0.22);
            particle.velocity = direction * (42.0 + message.intensity * 28.0);
            particle.remaining = 0.22 + message.intensity * 0.08;
            *visibility = Visibility::Visible;
        }
        if message.is_player {
            shake.remaining = shake.remaining.max(message.intensity * 0.16);
            if profile.data.settings.damage_indicators {
                indicator.remaining = 0.65;
                let (translation, rotation) = damage_indicator_pose(message.direction);
                for (mut visibility, mut transform) in &mut indicator_query {
                    *visibility = Visibility::Visible;
                    *transform =
                        UiTransform::from_translation(Val2::px(translation.x, translation.y));
                    transform.rotation = Rot2::radians(rotation);
                }
            }
        }
    }
}

fn combat_particle_count(intensity: f32, low_power: bool) -> usize {
    let normal_count = (4.0 + intensity * 5.0).round() as usize;
    if low_power {
        normal_count.min(2)
    } else {
        normal_count
    }
}

fn damage_indicator_pose(source_direction: Vec2) -> (Vec2, f32) {
    let world_direction = source_direction.normalize_or(Vec2::Y);
    let screen_direction = Vec2::new(world_direction.x, -world_direction.y);
    let translation =
        screen_direction * DAMAGE_INDICATOR_RADIUS - Vec2::splat(DAMAGE_INDICATOR_SIZE / 2.0);
    let inward = -screen_direction;
    let rotation = inward.to_angle() - std::f32::consts::FRAC_PI_2;
    (translation, rotation)
}

pub fn update_feedback_effects(
    time: Res<Time>,
    mut effects: Query<
        (&mut Transform, &mut Visibility, &mut EffectParticle),
        Without<DamageIndicator>,
    >,
    mut indicator: ResMut<DamageIndicatorState>,
    mut indicator_query: Query<&mut Visibility, (With<DamageIndicator>, Without<EffectParticle>)>,
) {
    let dt = time.delta_secs();
    for (mut transform, mut visibility, mut particle) in &mut effects {
        if particle.remaining <= 0.0 {
            continue;
        }
        particle.remaining -= dt;
        transform.translation += (particle.velocity * dt).extend(0.0);
        particle.velocity *= (1.0 - 7.0 * dt).max(0.0);
        transform.scale *= (1.0 - 2.8 * dt).max(0.0);
        if particle.remaining <= 0.0 {
            *visibility = Visibility::Hidden;
        }
    }
    indicator.remaining = (indicator.remaining - dt).max(0.0);
    if indicator.remaining <= 0.0 {
        for mut visibility in &mut indicator_query {
            *visibility = Visibility::Hidden;
        }
    }
}

pub fn camera_shake_offset(shake: &mut CameraShake, dt: f32, configured_strength: f32) -> Vec2 {
    if shake.remaining <= 0.0 || configured_strength <= 0.0 {
        return Vec2::ZERO;
    }
    shake.remaining = (shake.remaining - dt).max(0.0);
    shake.phase += dt * 74.0;
    let envelope = (shake.remaining / 0.24).clamp(0.0, 1.0);
    Vec2::new(shake.phase.sin(), (shake.phase * 1.73).cos()) * configured_strength * envelope * 7.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_shake_is_bounded_and_decays() {
        let mut shake = CameraShake {
            remaining: 0.2,
            phase: 0.0,
        };
        let first = camera_shake_offset(&mut shake, 1.0 / 60.0, 1.0);
        assert!(first.length() <= 10.0);
        for _ in 0..20 {
            camera_shake_offset(&mut shake, 1.0 / 60.0, 1.0);
        }
        assert_eq!(shake.remaining, 0.0);
    }

    #[test]
    fn damage_indicator_sits_toward_source_and_points_inward() {
        for source_direction in [Vec2::X, Vec2::Y, Vec2::NEG_X, Vec2::NEG_Y] {
            let (translation, rotation) = damage_indicator_pose(source_direction);
            let center = translation + Vec2::splat(DAMAGE_INDICATOR_SIZE / 2.0);
            let screen_direction = Vec2::new(source_direction.x, -source_direction.y).normalize();
            let arrow_direction = Vec2::from_angle(rotation + std::f32::consts::FRAC_PI_2);

            assert!(center.normalize().dot(screen_direction) > 0.999);
            assert!(arrow_direction.dot(-screen_direction) > 0.999);
        }
    }

    #[test]
    fn low_power_caps_only_presentation_particles() {
        assert_eq!(combat_particle_count(1.0, false), 9);
        assert_eq!(combat_particle_count(1.0, true), 2);
        assert_eq!(combat_particle_count(2.0, true), 2);
    }
}
