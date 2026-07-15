use crate::{
    combat::LifeGeneration,
    constants,
    evolution::{EvolutionKind, EvolutionState, PassiveKind},
    hud::{UpgradeKind, UpgradeState},
    passive::PassiveRuntime,
    player::{self, MoveVelocity, Player, Velocity},
    rng::Rng,
    tank::SpawnProtection,
};
use bevy::prelude::*;

#[derive(Component)]
pub struct Projectile;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileOwner {
    Player,
    EnemyBot(Entity),
}

#[derive(Component)]
pub struct Lifetime(pub f32);

#[derive(Component)]
pub struct ShootCooldown(pub f32);

#[derive(Component)]
pub struct ProjectileDamage(pub f32);

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ProjectileRadius(pub f32);

#[derive(Component)]
pub struct ProjectilePenetration(pub u32);

#[derive(Component)]
pub struct ProjectileKnockback(pub f32);

#[derive(Component, Clone, Copy)]
pub struct ProjectileEvolution(pub EvolutionKind);

#[derive(Component, Clone, Copy, Default)]
pub struct ProjectileTravel(pub f32);

#[derive(Component, Clone, Copy, Default)]
pub struct ProjectileRear(pub bool);

#[derive(Component, Clone, Copy)]
pub struct ProjectileSplashReady(pub bool);

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectileGeneration(pub u32);

impl ProjectileGeneration {
    pub fn matches(self, generation: &LifeGeneration) -> bool {
        self.0 == generation.0
    }
}

pub const HIT_HISTORY_CAPACITY: usize = 16;

#[derive(Component, Clone, Debug, Default)]
pub struct ProjectileHitHistory {
    entities: [Option<Entity>; HIT_HISTORY_CAPACITY],
    len: usize,
}

impl ProjectileHitHistory {
    pub fn record(&mut self, entity: Entity) -> bool {
        if self.entities[..self.len].contains(&Some(entity)) {
            return false;
        }
        if self.len < HIT_HISTORY_CAPACITY {
            self.entities[self.len] = Some(entity);
            self.len += 1;
            true
        } else {
            false
        }
    }
}

#[derive(Resource, Clone)]
pub struct ProjectileAssets {
    pub mesh: Handle<Mesh>,
}

pub fn setup_projectile_assets(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(ProjectileAssets {
        mesh: meshes.add(Circle::new(constants::PROJECTILE_RADIUS)),
    });
}

pub fn projectile_radius(upgrades: &UpgradeState, evolution: &EvolutionState) -> f32 {
    let damage_upgrade_scale = 1.0 + upgrades.level_of(UpgradeKind::BulletDamage) as f32 * 0.02;
    constants::PROJECTILE_RADIUS * damage_upgrade_scale * evolution.projectile_size_multiplier()
}

pub fn projectile_lifetime(
    upgrades: &UpgradeState,
    evolution: &EvolutionState,
    lifetime_multiplier: f32,
) -> f32 {
    let speed_upgrade_multiplier = upgrades.bullet_speed_multiplier().max(f32::EPSILON);
    constants::PROJECTILE_LIFETIME
        * evolution.projectile_lifetime_multiplier()
        * lifetime_multiplier
        / speed_upgrade_multiplier
}

pub fn shoot_projectile(
    mut commands: Commands,
    assets: Res<ProjectileAssets>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    upgrades: Res<UpgradeState>,
    evolution: Res<EvolutionState>,
    profile: Res<crate::profile::Profile>,
    palettes: Res<crate::palette::PaletteMaterials>,
    mut rng: ResMut<Rng>,
    mut player_query: Query<
        (
            &Transform,
            &mut ShootCooldown,
            &mut SpawnProtection,
            &mut MoveVelocity,
            &mut PassiveRuntime,
            &mut crate::ability::ActiveAbilityState,
            &LifeGeneration,
        ),
        With<Player>,
    >,
) {
    let Ok((
        transform,
        mut cooldown,
        mut protection,
        mut move_velocity,
        mut runtime,
        mut ability,
        generation,
    )) = player_query.single_mut()
    else {
        return;
    };

    cooldown.0 -= time.delta_secs();
    if ability.firing_disabled() {
        return;
    }
    if cooldown.0 > 0.0 {
        return;
    }

    if !mouse.pressed(MouseButton::Left) {
        return;
    }

    let mut adjustments = runtime.shot_adjustments(evolution.current_kind);
    if ability.braced() {
        adjustments.speed_multiplier = 1.30;
        adjustments.spread_multiplier = 0.25;
    }
    if ability.full_battery() {
        adjustments.bank = None;
    }
    let primed = ability.primed_shot();
    cooldown.0 = upgrades.reload_cooldown()
        * evolution.reload_multiplier()
        * adjustments.cooldown_multiplier
        * ability.reload_multiplier();
    protection.cancel();
    let spread = evolution.spread_radians() * adjustments.spread_multiplier;
    let base_damage =
        upgrades.bullet_damage() * evolution.bullet_damage_multiplier() * primed.damage;
    let bullet_speed = upgrades.bullet_speed()
        * evolution.bullet_speed_multiplier()
        * adjustments.speed_multiplier
        * primed.speed;
    let lifetime = projectile_lifetime(&upgrades, &evolution, primed.lifetime);
    let knockback = evolution.bullet_knockback_multiplier();
    let projectile_radius = projectile_radius(&upgrades, &evolution);
    let projectile_scale = projectile_radius / constants::PROJECTILE_RADIUS;

    let specs = evolution.barrel_specs();
    for (index, spec) in specs.iter().enumerate() {
        let bank_index = if evolution.current_kind == EvolutionKind::Fusillade {
            index / 4
        } else {
            index / 2
        };
        if adjustments.bank.is_some_and(|bank| bank_index != bank) {
            continue;
        }
        let jitter = if spread > 0.0 {
            let roll = rng.next(10_000) as f32 / 9_999.0;
            (roll * 2.0 - 1.0) * spread
        } else {
            0.0
        };
        let shot_offset = spec.angle_offset + adjustments.angle_offset;
        let barrel_rotation = Quat::from_rotation_z(shot_offset);
        let shot_rotation = Quat::from_rotation_z(shot_offset + jitter);
        let forward = transform.rotation * barrel_rotation * Vec3::Y;
        let right = transform.rotation * barrel_rotation * Vec3::X;
        let direction = transform.rotation * shot_rotation * Vec3::Y;
        let spawn_pos = transform.translation
            + forward
                * player::muzzle_projectile_distance(spec.length, &evolution, projectile_radius)
            + right * spec.lateral_offset;
        let damage = (base_damage * spec.damage_multiplier).max(0.1);

        commands
            .spawn((
                Projectile,
                ProjectileOwner::Player,
                Lifetime(lifetime),
                ProjectileDamage(damage),
                ProjectilePenetration(
                    upgrades
                        .bullet_penetration()
                        .saturating_add(evolution.penetration_bonus())
                        .saturating_add(primed.penetration),
                ),
                ProjectileKnockback(knockback),
                ProjectileEvolution(evolution.current_kind),
                ProjectileTravel::default(),
                ProjectileRear(spec.angle_offset.cos() < 0.0),
                ProjectileSplashReady(evolution.passive() == PassiveKind::Splash),
                ProjectileHitHistory::default(),
                Mesh2d(assets.mesh.clone()),
                MeshMaterial2d(
                    palettes
                        .player(profile.data.selected_palette)
                        .projectile
                        .clone(),
                ),
                Transform::from_translation(spawn_pos).with_scale(Vec3::new(
                    projectile_scale,
                    projectile_scale,
                    1.0,
                )),
                Velocity(direction.xy() * bullet_speed),
            ))
            .insert((
                ProjectileGeneration(generation.0),
                ProjectileRadius(projectile_radius),
                crate::ability::ProjectileAbility {
                    clears_projectiles: primed.clears_projectiles,
                    pinning: primed.pinning,
                    ..default()
                },
            ));
    }

    if evolution.passive() == PassiveKind::BoosterRecoil {
        let forward = (transform.rotation * Vec3::Y).xy();
        let recoil = if evolution.current_kind == EvolutionKind::Afterburner {
            0.12
        } else {
            0.08
        };
        move_velocity.0 += forward * upgrades.movement_speed() * recoil;
        move_velocity.0 = move_velocity
            .0
            .clamp_length_max(upgrades.movement_speed() * evolution.movement_multiplier() * 1.25);
    }
}

pub fn projectile_update(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut Transform,
            &Velocity,
            &mut Lifetime,
            &mut ProjectileTravel,
            &ProjectileRadius,
        ),
        With<Projectile>,
    >,
) {
    for (entity, mut transform, velocity, mut lifetime, mut travel, radius) in query.iter_mut() {
        let dt = time.delta_secs();
        lifetime.0 -= dt;
        if lifetime.0 <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation += velocity.0.extend(0.0) * dt;
        travel.0 += velocity.0.length() * dt;
        let half = constants::arena_half_extent() + radius.0;
        if transform.translation.x.abs() > half || transform.translation.y.abs() > half {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simulate_projectile_with_power_mode(low_power: bool) -> (Vec3, f32, f32) {
        let mut world = World::new();
        let mut profile = crate::profile::Profile::test_with_path(None);
        profile.data.settings.low_power_mode = low_power;
        world.insert_resource(profile);
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_micros(15_625));
        world.insert_resource(time);
        let projectile = world
            .spawn((
                Projectile,
                Transform::default(),
                Velocity(Vec2::new(80.0, -20.0)),
                Lifetime(2.0),
                ProjectileTravel::default(),
                ProjectileRadius(constants::PROJECTILE_RADIUS),
            ))
            .id();
        let mut schedule = Schedule::default();
        schedule.add_systems(projectile_update);
        for _ in 0..16 {
            schedule.run(&mut world);
        }
        let transform = world.get::<Transform>(projectile).unwrap().translation;
        let lifetime = world.get::<Lifetime>(projectile).unwrap().0;
        let travel = world.get::<ProjectileTravel>(projectile).unwrap().0;
        (transform, lifetime, travel)
    }

    #[test]
    fn projectile_size_tracks_damage_upgrades_and_heavy_evolutions() {
        let base_upgrades = UpgradeState::default();
        let tank = EvolutionState::default();
        let base_radius = projectile_radius(&base_upgrades, &tank);
        assert!((base_radius - 4.8).abs() < f32::EPSILON);

        let mut damage_upgrades = base_upgrades.clone();
        damage_upgrades.levels[UpgradeKind::BulletDamage.index()] = 1;
        let upgraded_radius = projectile_radius(&damage_upgrades, &tank);
        assert!((upgraded_radius / base_radius - 1.02).abs() < 0.000_1);

        let cannon = EvolutionState {
            current_kind: EvolutionKind::Cannon,
            ..default()
        };
        let annihilator = EvolutionState {
            current_kind: EvolutionKind::Annihilator,
            ..default()
        };
        let rail_cannon = EvolutionState {
            current_kind: EvolutionKind::RailCannon,
            ..default()
        };
        assert!(projectile_radius(&base_upgrades, &cannon) > base_radius);
        assert!(
            projectile_radius(&base_upgrades, &annihilator)
                > projectile_radius(&base_upgrades, &cannon)
        );
        assert_eq!(projectile_radius(&base_upgrades, &rail_cannon), base_radius);
    }

    #[test]
    fn low_power_mode_does_not_change_projectile_gameplay_state() {
        assert_eq!(
            simulate_projectile_with_power_mode(false),
            simulate_projectile_with_power_mode(true)
        );
    }

    #[test]
    fn bullet_speed_upgrades_do_not_increase_projectile_range() {
        let base_upgrades = UpgradeState::default();
        let mut speed_upgrades = base_upgrades.clone();
        speed_upgrades.levels[UpgradeKind::BulletSpeed.index()] = 8;

        for (kind, expected_range) in [
            (EvolutionKind::Tank, 800.0),
            (EvolutionKind::Sniper, 2_012.8),
            (EvolutionKind::Marksman, 2_717.28),
            (EvolutionKind::Deadeye, 3_169.4),
        ] {
            let evolution = EvolutionState {
                current_kind: kind,
                ..default()
            };
            let base_range = base_upgrades.bullet_speed()
                * evolution.bullet_speed_multiplier()
                * projectile_lifetime(&base_upgrades, &evolution, 1.0);
            let upgraded_range = speed_upgrades.bullet_speed()
                * evolution.bullet_speed_multiplier()
                * projectile_lifetime(&speed_upgrades, &evolution, 1.0);

            assert!((base_range - upgraded_range).abs() < 0.001, "{kind:?}");
            assert!((base_range - expected_range).abs() < 0.1, "{kind:?}");
        }
    }

    #[test]
    fn primed_lifetime_multiplier_still_extends_projectile_range() {
        let upgrades = UpgradeState::default();
        let deadeye = EvolutionState {
            current_kind: EvolutionKind::Deadeye,
            ..default()
        };
        let normal = projectile_lifetime(&upgrades, &deadeye, 1.0);
        let primed = projectile_lifetime(&upgrades, &deadeye, 1.5);

        assert!((primed / normal - 1.5).abs() < 0.000_1);
    }

    #[test]
    fn hit_history_accepts_distinct_targets_once() {
        let mut history = ProjectileHitHistory::default();
        let a = Entity::from_bits(1);
        let b = Entity::from_bits(2);
        assert!(history.record(a));
        assert!(!history.record(a));
        assert!(history.record(b));
    }

    #[test]
    fn projectile_generation_expires_when_owner_starts_a_new_life() {
        let projectile = ProjectileGeneration(3);
        assert!(projectile.matches(&LifeGeneration(3)));
        assert!(!projectile.matches(&LifeGeneration(4)));
    }
}
