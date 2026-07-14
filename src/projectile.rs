use crate::{
    constants,
    evolution::{EvolutionKind, EvolutionState, PassiveKind},
    hud::UpgradeState,
    passive::PassiveRuntime,
    player::{self, MoveVelocity, Player, Velocity},
    rng::Rng,
    tank::SpawnProtection,
};
use bevy::prelude::*;

#[derive(Component)]
pub struct Projectile;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
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
        ),
        With<Player>,
    >,
) {
    let Ok((transform, mut cooldown, mut protection, mut move_velocity, mut runtime)) =
        player_query.single_mut()
    else {
        return;
    };

    cooldown.0 -= time.delta_secs();
    if cooldown.0 > 0.0 {
        return;
    }

    if !mouse.pressed(MouseButton::Left) {
        return;
    }

    let adjustments = runtime.shot_adjustments(evolution.passive());
    cooldown.0 = upgrades.reload_cooldown()
        * evolution.reload_multiplier()
        * adjustments.cooldown_multiplier;
    protection.cancel();
    let spread = evolution.spread_radians() * adjustments.spread_multiplier;
    let base_damage = upgrades.bullet_damage() * evolution.bullet_damage_multiplier();
    let bullet_speed = upgrades.bullet_speed()
        * evolution.bullet_speed_multiplier()
        * adjustments.speed_multiplier;
    let lifetime = constants::PROJECTILE_LIFETIME * evolution.projectile_lifetime_multiplier();
    let knockback = evolution.bullet_knockback_multiplier();

    let specs = evolution.barrel_specs();
    for (index, spec) in specs.iter().enumerate() {
        if adjustments.bank.is_some_and(|bank| index / 2 != bank) {
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
            + forward * player::muzzle_projectile_distance(spec.length, &evolution)
            + right * spec.lateral_offset;
        let damage = (base_damage * spec.damage_multiplier).max(0.1);

        commands.spawn((
            Projectile,
            ProjectileOwner::Player,
            Lifetime(lifetime),
            ProjectileDamage(damage),
            ProjectilePenetration(
                upgrades
                    .bullet_penetration()
                    .saturating_add(evolution.penetration_bonus()),
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
            Transform::from_translation(spawn_pos),
            Velocity(direction.xy() * bullet_speed),
        ));
    }

    if evolution.passive() == PassiveKind::BoosterRecoil {
        let forward = (transform.rotation * Vec3::Y).xy();
        move_velocity.0 += forward * upgrades.movement_speed() * 0.08;
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
        ),
        With<Projectile>,
    >,
) {
    for (entity, mut transform, velocity, mut lifetime, mut travel) in query.iter_mut() {
        let dt = time.delta_secs();
        lifetime.0 -= dt;
        if lifetime.0 <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation += velocity.0.extend(0.0) * dt;
        travel.0 += velocity.0.length() * dt;
        let half = constants::arena_half_extent() + constants::PROJECTILE_RADIUS;
        if transform.translation.x.abs() > half || transform.translation.y.abs() > half {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hit_history_accepts_distinct_targets_once() {
        let mut history = ProjectileHitHistory::default();
        let a = Entity::from_bits(1);
        let b = Entity::from_bits(2);
        assert!(history.record(a));
        assert!(!history.record(a));
        assert!(history.record(b));
    }
}
