use crate::{
    enemy_bot::{
        EnemyBot, EnemyBotEvolution, EnemyBotHealth, EnemyBotMoveVelocity, EnemyBotUpgrades,
    },
    evolution::{EvolutionKind, EvolutionState, PassiveKind},
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
    pub firing: bool,
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
            firing: false,
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
        evolution: EvolutionKind,
        dt: f32,
        speed_fraction: f32,
        firing: bool,
        recently_damaged: bool,
        max_health: f32,
    ) {
        let passive = crate::evolution::definition(evolution).passive;
        self.firing = firing;
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
            let capstone = evolution == EvolutionKind::Guardian;
            self.shield_max = max_health * if capstone { 0.35 } else { 0.25 };
            if first_activation {
                self.shield = self.shield_max;
            } else if self.shield <= 0.0 && self.no_damage < 4.0 {
                self.shield = 0.0;
            } else if self.no_damage >= 4.0 {
                let recharge = if capstone { 0.15 } else { 0.10 };
                self.shield = (self.shield + max_health * recharge * dt).min(self.shield_max);
            }
        } else {
            self.shield = 0.0;
            self.shield_max = 0.0;
        }
    }

    pub fn shot_adjustments(&mut self, evolution: EvolutionKind) -> ShotAdjustments {
        let passive = crate::evolution::definition(evolution).passive;
        let mut adjustment = ShotAdjustments::default();
        match passive {
            PassiveKind::MinigunSpin => {
                let spin = (self.sustained_fire / 1.5).clamp(0.0, 1.0);
                adjustment.cooldown_multiplier = 1.0
                    - spin
                        * if evolution == EvolutionKind::Sentry {
                            0.35
                        } else {
                            0.25
                        };
                adjustment.spread_multiplier = 1.0
                    + spin
                        * if evolution == EvolutionKind::Sentry {
                            0.30
                        } else {
                            0.40
                        };
            }
            PassiveKind::Stabilized
                if self.stationary
                    >= if evolution == EvolutionKind::Emplacement {
                        0.60
                    } else {
                        0.75
                    } =>
            {
                adjustment.speed_multiplier = if evolution == EvolutionKind::Emplacement {
                    1.30
                } else {
                    1.20
                };
                adjustment.spread_multiplier = if evolution == EvolutionKind::Emplacement {
                    0.25
                } else {
                    0.40
                };
            }
            PassiveKind::AlternatingPairs => {
                adjustment.bank = Some(usize::from(self.volley_phase));
                self.volley_phase = !self.volley_phase;
            }
            PassiveKind::PhasedFan => {
                let offset = if evolution == EvolutionKind::Bombardier {
                    0.07
                } else {
                    0.04
                };
                adjustment.angle_offset = if self.volley_phase { offset } else { -offset };
                self.volley_phase = !self.volley_phase;
            }
            _ => {}
        }
        adjustment
    }

    pub fn projectile_hit_multiplier(
        &mut self,
        evolution: EvolutionKind,
        target: Entity,
        travel_distance: f32,
    ) -> f32 {
        let passive = crate::evolution::definition(evolution).passive;
        match passive {
            PassiveKind::DistanceDamage => {
                let cap = match evolution {
                    EvolutionKind::Lancer => 0.80,
                    EvolutionKind::Deadeye => 0.90,
                    _ => 0.60,
                };
                1.0 + (travel_distance / 100.0 * 0.05).min(cap)
            }
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
                    self.follow_up_hits = if evolution == EvolutionKind::Pursuer {
                        3
                    } else {
                        2
                    };
                    self.stack_timer = 2.5;
                    1.0
                }
            }
            PassiveKind::ConsecutiveHits => {
                if self.tracked_target == Some(target) && self.stack_timer > 0.0 {
                    self.stacks =
                        self.stacks
                            .saturating_add(1)
                            .min(if evolution == EvolutionKind::Impaler {
                                7
                            } else {
                                5
                            });
                } else {
                    self.tracked_target = Some(target);
                    self.stacks = 0;
                }
                self.stack_timer = if evolution == EvolutionKind::Impaler {
                    2.0
                } else {
                    1.5
                };
                1.0 + self.stacks as f32
                    * if evolution == EvolutionKind::Impaler {
                        0.07
                    } else {
                        0.08
                    }
            }
            PassiveKind::HitSpeed => {
                if self.speed_boost_cooldown <= 0.0 {
                    self.speed_boost = if evolution == EvolutionKind::Ace {
                        1.5
                    } else {
                        1.2
                    };
                    self.speed_boost_cooldown = if evolution == EvolutionKind::Ace {
                        2.0
                    } else {
                        2.5
                    };
                }
                1.0
            }
            _ => 1.0,
        }
    }

    pub fn projectile_shape_hit_multiplier(
        &mut self,
        evolution: EvolutionKind,
        target: Entity,
        travel_distance: f32,
    ) -> f32 {
        if crate::evolution::definition(evolution).passive == PassiveKind::DistanceDamage {
            1.0
        } else {
            self.projectile_hit_multiplier(evolution, target, travel_distance)
        }
    }

    pub fn incoming_damage(
        &mut self,
        evolution: EvolutionKind,
        damage: f32,
        frontal: bool,
        _speed_fraction: f32,
    ) -> f32 {
        let passive = crate::evolution::definition(evolution).passive;
        let mut damage = damage;
        let entrenched_after = if evolution == EvolutionKind::Stronghold {
            1.0
        } else {
            1.25
        };
        if passive == PassiveKind::Entrenched && self.stationary >= entrenched_after && !self.firing
        {
            damage *= if evolution == EvolutionKind::Stronghold {
                0.65
            } else {
                0.75
            };
        }
        if passive == PassiveKind::FrontalArmor && frontal {
            damage *= if evolution == EvolutionKind::Vanguard {
                0.55
            } else {
                0.65
            };
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
        evolution: EvolutionKind,
        damage: f32,
        speed_fraction: f32,
    ) -> f32 {
        let passive = crate::evolution::definition(evolution).passive;
        let mut damage = damage;
        let entrenched_after = if evolution == EvolutionKind::Stronghold {
            1.0
        } else {
            1.25
        };
        if passive == PassiveKind::Entrenched && self.stationary >= entrenched_after {
            damage *= if evolution == EvolutionKind::Stronghold {
                0.65
            } else {
                0.75
            };
        }
        if passive == PassiveKind::MomentumArmor && speed_fraction >= 0.8 {
            damage *= if evolution == EvolutionKind::Dreadnought {
                0.65
            } else {
                0.75
            };
        }
        damage
    }
}

pub fn movement_multiplier(runtime: &PassiveRuntime, evolution: EvolutionKind) -> f32 {
    if crate::evolution::definition(evolution).passive == PassiveKind::HitSpeed
        && runtime.speed_boost > 0.0
    {
        if evolution == EvolutionKind::Ace {
            1.25
        } else {
            1.18
        }
    } else {
        1.0
    }
}

pub fn body_damage_multiplier(evolution: EvolutionKind, speed_fraction: f32) -> f32 {
    if crate::evolution::definition(evolution).passive == PassiveKind::MomentumArmor {
        1.0 + (speed_fraction / 0.8).clamp(0.0, 1.0)
            * if evolution == EvolutionKind::Dreadnought {
                0.70
            } else {
                0.50
            }
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
            player_evolution.current_kind,
            dt,
            velocity.0.length() / max_speed.max(1.0),
            mouse.pressed(MouseButton::Left),
            recent.remaining > 0.0,
            health.max,
        );
        if health.current > 0.0
            && player_evolution.passive() == PassiveKind::Entrenched
            && runtime.stationary
                >= if player_evolution.current_kind == EvolutionKind::Stronghold {
                    1.0
                } else {
                    1.25
                }
            && !mouse.pressed(MouseButton::Left)
        {
            health.current = (health.current + 4.0 * dt).min(health.max);
        }
    }
    for (velocity, mut health, mut runtime, recent, evolution, upgrades, cooldown) in &mut bots {
        let max_speed = upgrades.0.movement_speed() * evolution.0.movement_multiplier();
        runtime.update(
            evolution.0.current_kind,
            dt,
            velocity.0.length() / max_speed.max(1.0),
            cooldown.0 > 0.0,
            recent.remaining > 0.0,
            health.max,
        );
        if health.current > 0.0
            && evolution.0.passive() == PassiveKind::Entrenched
            && runtime.stationary
                >= if evolution.0.current_kind == EvolutionKind::Stronghold {
                    1.0
                } else {
                    1.25
                }
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
        let spin = runtime.shot_adjustments(EvolutionKind::Minigun);
        assert_eq!(spin.cooldown_multiplier, 0.75);
        assert_eq!(spin.spread_multiplier, 1.4);
        runtime.stationary = 0.75;
        let stable = runtime.shot_adjustments(EvolutionKind::Stabilizer);
        assert_eq!(stable.speed_multiplier, 1.2);
        assert_eq!(stable.spread_multiplier, 0.4);
    }

    #[test]
    fn marks_and_needler_stacks_are_owner_target_specific() {
        let target = Entity::from_bits(7);
        let mut hunter = PassiveRuntime::default();
        assert_eq!(
            hunter.projectile_hit_multiplier(EvolutionKind::Hunter, target, 0.0),
            1.0
        );
        assert_eq!(
            hunter.projectile_hit_multiplier(EvolutionKind::Hunter, target, 0.0),
            1.25
        );
        let mut needler = PassiveRuntime::default();
        for expected in [1.0, 1.08, 1.16, 1.24, 1.32, 1.40, 1.40] {
            let actual = needler.projectile_hit_multiplier(EvolutionKind::Needler, target, 0.0);
            assert!((actual - expected).abs() < 0.001);
        }
    }

    #[test]
    fn distance_damage_applies_to_tanks_but_not_neutral_shapes() {
        let target = Entity::from_bits(8);
        let mut runtime = PassiveRuntime::default();

        for evolution in [
            EvolutionKind::RailCannon,
            EvolutionKind::Marksman,
            EvolutionKind::Lancer,
            EvolutionKind::Deadeye,
        ] {
            assert!(runtime.projectile_hit_multiplier(evolution, target, 2_000.0) > 1.0);
            assert_eq!(
                runtime.projectile_shape_hit_multiplier(evolution, target, 2_000.0),
                1.0
            );
        }

        let mut hunter = PassiveRuntime::default();
        assert_eq!(
            hunter.projectile_shape_hit_multiplier(EvolutionKind::Hunter, target, 0.0),
            1.0
        );
        assert_eq!(
            hunter.projectile_shape_hit_multiplier(EvolutionKind::Hunter, target, 0.0),
            1.25
        );
    }

    #[test]
    fn shield_absorbs_only_frontal_projectile_damage() {
        let mut runtime = PassiveRuntime {
            shield: 10.0,
            shield_max: 10.0,
            ..default()
        };
        assert_eq!(
            runtime.incoming_damage(EvolutionKind::Bulwark, 7.0, true, 0.0),
            0.0
        );
        assert_eq!(runtime.shield, 3.0);
        assert_eq!(
            runtime.incoming_damage(EvolutionKind::Bulwark, 7.0, false, 0.0),
            7.0
        );
    }

    #[test]
    fn bulwark_starts_full_and_recharges_after_four_seconds() {
        let mut runtime = PassiveRuntime::default();
        runtime.update(EvolutionKind::Bulwark, 0.1, 0.0, false, false, 100.0);
        assert_eq!(runtime.shield, 25.0);
        runtime.shield = 0.0;
        runtime.no_damage = 3.8;
        runtime.update(EvolutionKind::Bulwark, 0.1, 0.0, false, false, 100.0);
        assert_eq!(runtime.shield, 0.0);
        runtime.update(EvolutionKind::Bulwark, 0.2, 0.0, false, false, 100.0);
        assert!(runtime.shield > 0.0);
    }

    #[test]
    fn entrenched_mitigation_requires_holding_fire() {
        let mut runtime = PassiveRuntime {
            stationary: 2.0,
            firing: true,
            ..default()
        };
        assert_eq!(
            runtime.incoming_damage(EvolutionKind::Fortress, 20.0, false, 0.0),
            20.0
        );
        runtime.firing = false;
        assert_eq!(
            runtime.incoming_damage(EvolutionKind::Fortress, 20.0, false, 0.0),
            15.0
        );
    }
}
