use crate::{
    enemy_bot::{
        EnemyBot, EnemyBotEvolution, EnemyBotHealth, EnemyBotMoveVelocity, EnemyBotUpgrades,
    },
    evolution::{EvolutionState, PassiveKind},
    hud::UpgradeState,
    player::{MoveVelocity, Player, PlayerHealth},
    projectile::ShootCooldown,
    tank::RecentDamage,
};
use bevy::prelude::*;

#[derive(Component, Clone, Debug)]
pub struct PassiveRuntime {
    pub sustained_fire: f32,
    pub stationary: f32,
    pub no_damage: f32,
    pub shield: f32,
    pub shield_max: f32,
    pub tracked_target: Option<Entity>,
    pub stacks: u8,
    pub stack_timer: f32,
    pub follow_up_hits: u8,
    pub speed_boost: f32,
    pub speed_boost_cooldown: f32,
    pub volley_phase: bool,
}

impl Default for PassiveRuntime {
    fn default() -> Self {
        Self {
            sustained_fire: 0.0,
            stationary: 0.0,
            no_damage: 0.0,
            shield: 0.0,
            shield_max: 0.0,
            tracked_target: None,
            stacks: 0,
            stack_timer: 0.0,
            follow_up_hits: 0,
            speed_boost: 0.0,
            speed_boost_cooldown: 0.0,
            volley_phase: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ShotAdjustments {
    pub cooldown_multiplier: f32,
    pub speed_multiplier: f32,
    pub spread_multiplier: f32,
    pub angle_offset: f32,
    pub bank: Option<usize>,
}

impl Default for ShotAdjustments {
    fn default() -> Self {
        Self {
            cooldown_multiplier: 1.0,
            speed_multiplier: 1.0,
            spread_multiplier: 1.0,
            angle_offset: 0.0,
            bank: None,
        }
    }
}

impl PassiveRuntime {
    pub fn reset_for_life(&mut self) {
        *self = Self::default();
    }

    pub fn update(
        &mut self,
        passive: PassiveKind,
        dt: f32,
        speed_fraction: f32,
        firing: bool,
        recently_damaged: bool,
        max_health: f32,
    ) {
        if firing {
            self.sustained_fire = (self.sustained_fire + dt).min(1.5);
        } else {
            self.sustained_fire = (self.sustained_fire - dt * 1.5).max(0.0);
        }
        if speed_fraction <= 0.3 {
            self.stationary += dt;
        } else {
            self.stationary = 0.0;
        }
        if recently_damaged {
            self.no_damage = 0.0;
        } else {
            self.no_damage += dt;
        }
        self.stack_timer = (self.stack_timer - dt).max(0.0);
        if self.stack_timer <= 0.0 {
            self.tracked_target = None;
            self.stacks = 0;
            self.follow_up_hits = 0;
        }
        self.speed_boost = (self.speed_boost - dt).max(0.0);
        self.speed_boost_cooldown = (self.speed_boost_cooldown - dt).max(0.0);

        if passive == PassiveKind::FrontalShield {
            let first_activation = self.shield_max <= 0.0;
            self.shield_max = max_health * 0.25;
            if first_activation {
                self.shield = self.shield_max;
            } else if self.shield <= 0.0 && self.no_damage < 4.0 {
                self.shield = 0.0;
            } else if self.no_damage >= 4.0 {
                self.shield = (self.shield + max_health * 0.10 * dt).min(self.shield_max);
            }
        } else {
            self.shield = 0.0;
            self.shield_max = 0.0;
        }
    }

    pub fn shot_adjustments(&mut self, passive: PassiveKind) -> ShotAdjustments {
        let mut adjustment = ShotAdjustments::default();
        match passive {
            PassiveKind::MinigunSpin => {
                let spin = (self.sustained_fire / 1.5).clamp(0.0, 1.0);
                adjustment.cooldown_multiplier = 1.0 - spin * 0.25;
                adjustment.spread_multiplier = 1.0 + spin * 0.40;
            }
            PassiveKind::Stabilized if self.stationary >= 0.75 => {
                adjustment.speed_multiplier = 1.20;
                adjustment.spread_multiplier = 0.40;
            }
            PassiveKind::AlternatingPairs => {
                adjustment.bank = Some(usize::from(self.volley_phase));
                self.volley_phase = !self.volley_phase;
            }
            PassiveKind::PhasedFan => {
                adjustment.angle_offset = if self.volley_phase { 0.04 } else { -0.04 };
                self.volley_phase = !self.volley_phase;
            }
            _ => {}
        }
        adjustment
    }

    pub fn projectile_hit_multiplier(
        &mut self,
        passive: PassiveKind,
        target: Entity,
        travel_distance: f32,
    ) -> f32 {
        match passive {
            PassiveKind::DistanceDamage => 1.0 + (travel_distance / 100.0 * 0.05).min(0.60),
            PassiveKind::HunterMark => {
                if self.tracked_target == Some(target)
                    && self.stack_timer > 0.0
                    && self.follow_up_hits > 0
                {
                    self.follow_up_hits -= 1;
                    self.stack_timer = 2.5;
                    1.25
                } else {
                    self.tracked_target = Some(target);
                    self.follow_up_hits = 2;
                    self.stack_timer = 2.5;
                    1.0
                }
            }
            PassiveKind::ConsecutiveHits => {
                if self.tracked_target == Some(target) && self.stack_timer > 0.0 {
                    self.stacks = self.stacks.saturating_add(1).min(5);
                } else {
                    self.tracked_target = Some(target);
                    self.stacks = 0;
                }
                self.stack_timer = 1.5;
                1.0 + self.stacks as f32 * 0.08
            }
            PassiveKind::HitSpeed => {
                if self.speed_boost_cooldown <= 0.0 {
                    self.speed_boost = 1.2;
                    self.speed_boost_cooldown = 2.5;
                }
                1.0
            }
            _ => 1.0,
        }
    }

    pub fn incoming_damage(
        &mut self,
        passive: PassiveKind,
        damage: f32,
        frontal: bool,
        _speed_fraction: f32,
    ) -> f32 {
        let mut damage = damage;
        if passive == PassiveKind::Entrenched && self.stationary >= 1.25 {
            damage *= 0.75;
        }
        if passive == PassiveKind::FrontalArmor && frontal {
            damage *= 0.65;
        }
        if passive == PassiveKind::FrontalShield && frontal && self.shield > 0.0 {
            let absorbed = damage.min(self.shield);
            self.shield -= absorbed;
            damage -= absorbed;
            self.no_damage = 0.0;
        }
        damage.max(0.0)
    }

    pub fn incoming_contact_damage(
        &mut self,
        passive: PassiveKind,
        damage: f32,
        speed_fraction: f32,
    ) -> f32 {
        let mut damage = damage;
        if passive == PassiveKind::Entrenched && self.stationary >= 1.25 {
            damage *= 0.75;
        }
        if passive == PassiveKind::MomentumArmor && speed_fraction >= 0.8 {
            damage *= 0.75;
        }
        damage
    }
}

pub fn movement_multiplier(runtime: &PassiveRuntime, passive: PassiveKind) -> f32 {
    if passive == PassiveKind::HitSpeed && runtime.speed_boost > 0.0 {
        1.18
    } else {
        1.0
    }
}

pub fn body_damage_multiplier(passive: PassiveKind, speed_fraction: f32) -> f32 {
    if passive == PassiveKind::MomentumArmor {
        1.0 + (speed_fraction / 0.8).clamp(0.0, 1.0) * 0.50
    } else {
        1.0
    }
}

pub fn is_frontal_hit(target: &Transform, projectile_velocity: Vec2, half_angle: f32) -> bool {
    let forward = (target.rotation * Vec3::Y).xy().normalize_or_zero();
    let toward_source = -projectile_velocity.normalize_or_zero();
    forward.dot(toward_source) >= half_angle.cos()
}

pub fn tick_passives(
    time: Res<Time<Fixed>>,
    mouse: Res<ButtonInput<MouseButton>>,
    player_upgrades: Res<UpgradeState>,
    player_evolution: Res<EvolutionState>,
    mut player: Query<
        (
            &MoveVelocity,
            &mut PlayerHealth,
            &mut PassiveRuntime,
            &RecentDamage,
        ),
        (With<Player>, Without<EnemyBot>),
    >,
    mut bots: Query<
        (
            &EnemyBotMoveVelocity,
            &mut EnemyBotHealth,
            &mut PassiveRuntime,
            &RecentDamage,
            &EnemyBotEvolution,
            &EnemyBotUpgrades,
            &ShootCooldown,
        ),
        (With<EnemyBot>, Without<Player>),
    >,
) {
    let dt = time.delta_secs();
    if let Ok((velocity, mut health, mut runtime, recent)) = player.single_mut() {
        let max_speed = player_upgrades.movement_speed() * player_evolution.movement_multiplier();
        runtime.update(
            player_evolution.passive(),
            dt,
            velocity.0.length() / max_speed.max(1.0),
            mouse.pressed(MouseButton::Left),
            recent.remaining > 0.0,
            health.max,
        );
        if player_evolution.passive() == PassiveKind::Entrenched
            && runtime.stationary >= 1.25
            && !mouse.pressed(MouseButton::Left)
        {
            health.current = (health.current + 4.0 * dt).min(health.max);
        }
    }
    for (velocity, mut health, mut runtime, recent, evolution, upgrades, cooldown) in &mut bots {
        let max_speed = upgrades.0.movement_speed() * evolution.0.movement_multiplier();
        runtime.update(
            evolution.0.passive(),
            dt,
            velocity.0.length() / max_speed.max(1.0),
            cooldown.0 > 0.0,
            recent.remaining > 0.0,
            health.max,
        );
        if evolution.0.passive() == PassiveKind::Entrenched
            && runtime.stationary >= 1.25
            && cooldown.0 <= 0.0
        {
            health.current = (health.current + 4.0 * dt).min(health.max);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minigun_spin_and_stabilizer_are_bounded() {
        let mut runtime = PassiveRuntime {
            sustained_fire: 1.5,
            ..default()
        };
        let spin = runtime.shot_adjustments(PassiveKind::MinigunSpin);
        assert_eq!(spin.cooldown_multiplier, 0.75);
        assert_eq!(spin.spread_multiplier, 1.4);
        runtime.stationary = 0.75;
        let stable = runtime.shot_adjustments(PassiveKind::Stabilized);
        assert_eq!(stable.speed_multiplier, 1.2);
        assert_eq!(stable.spread_multiplier, 0.4);
    }

    #[test]
    fn marks_and_needler_stacks_are_owner_target_specific() {
        let target = Entity::from_bits(7);
        let mut hunter = PassiveRuntime::default();
        assert_eq!(
            hunter.projectile_hit_multiplier(PassiveKind::HunterMark, target, 0.0),
            1.0
        );
        assert_eq!(
            hunter.projectile_hit_multiplier(PassiveKind::HunterMark, target, 0.0),
            1.25
        );
        let mut needler = PassiveRuntime::default();
        for expected in [1.0, 1.08, 1.16, 1.24, 1.32, 1.40, 1.40] {
            let actual =
                needler.projectile_hit_multiplier(PassiveKind::ConsecutiveHits, target, 0.0);
            assert!((actual - expected).abs() < 0.001);
        }
    }

    #[test]
    fn shield_absorbs_only_frontal_projectile_damage() {
        let mut runtime = PassiveRuntime {
            shield: 10.0,
            shield_max: 10.0,
            ..default()
        };
        assert_eq!(
            runtime.incoming_damage(PassiveKind::FrontalShield, 7.0, true, 0.0),
            0.0
        );
        assert_eq!(runtime.shield, 3.0);
        assert_eq!(
            runtime.incoming_damage(PassiveKind::FrontalShield, 7.0, false, 0.0),
            7.0
        );
    }

    #[test]
    fn bulwark_starts_full_and_recharges_after_four_seconds() {
        let mut runtime = PassiveRuntime::default();
        runtime.update(PassiveKind::FrontalShield, 0.1, 0.0, false, false, 100.0);
        assert_eq!(runtime.shield, 25.0);
        runtime.shield = 0.0;
        runtime.no_damage = 3.8;
        runtime.update(PassiveKind::FrontalShield, 0.1, 0.0, false, false, 100.0);
        assert_eq!(runtime.shield, 0.0);
        runtime.update(PassiveKind::FrontalShield, 0.2, 0.0, false, false, 100.0);
        assert!(runtime.shield > 0.0);
    }
}
