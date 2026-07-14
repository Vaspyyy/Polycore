use crate::{
    enemy_bot::{
        EnemyBot, EnemyBotEvolution, EnemyBotHealth, EnemyBotLevel, EnemyBotPlaystyle,
        EnemyBotUpgrades, EnemyBotXp, award_enemy_bot_xp,
    },
    player::{Player, PlayerHealth},
    rng::Rng,
    shape::{TotalXp, Xp},
};
use bevy::prelude::*;

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CombatStats {
    pub life_score: u32,
    pub kills: u32,
    pub deaths: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatantId {
    Player,
    EnemyBot(Entity),
}

#[derive(Clone, Copy, Debug)]
struct CombatDeath {
    victim: CombatantId,
    killer: Option<CombatantId>,
}

#[derive(Resource, Default)]
pub struct CombatDeathQueue(Vec<CombatDeath>);

impl CombatDeathQueue {
    pub fn record(&mut self, victim: CombatantId, killer: Option<CombatantId>) {
        self.0.push(CombatDeath { victim, killer });
    }
}

pub fn resolve_combat_deaths(
    mut queue: ResMut<CombatDeathQueue>,
    mut player_stats: Query<(&mut CombatStats, &PlayerHealth), (With<Player>, Without<EnemyBot>)>,
    mut bots: Query<
        (
            &mut CombatStats,
            &mut EnemyBotXp,
            &mut EnemyBotLevel,
            &mut EnemyBotUpgrades,
            &mut EnemyBotEvolution,
            &mut EnemyBotHealth,
            &EnemyBotPlaystyle,
        ),
        (With<EnemyBot>, Without<Player>),
    >,
    mut xp: ResMut<Xp>,
    mut total_xp: ResMut<TotalXp>,
    mut rng: ResMut<Rng>,
) {
    let deaths = std::mem::take(&mut queue.0);
    for death in deaths {
        let victim_score = match death.victim {
            CombatantId::Player => player_stats
                .single()
                .map_or(0, |(stats, _)| stats.life_score),
            CombatantId::EnemyBot(entity) => bots.get(entity).map_or(0, |bot| bot.0.life_score),
        };
        match death.victim {
            CombatantId::Player => {
                if let Ok((mut stats, _)) = player_stats.single_mut() {
                    stats.deaths += 1;
                }
            }
            CombatantId::EnemyBot(entity) => {
                if let Ok((mut stats, ..)) = bots.get_mut(entity) {
                    stats.deaths += 1;
                }
            }
        }

        let Some(killer) = death.killer.filter(|killer| *killer != death.victim) else {
            continue;
        };
        let reward = kill_xp(victim_score);
        match killer {
            CombatantId::Player => {
                let Ok((mut stats, health)) = player_stats.single_mut() else {
                    continue;
                };
                if health.current > 0.0 {
                    stats.kills += 1;
                    stats.life_score = stats.life_score.saturating_add(reward);
                    xp.0 = xp.0.saturating_add(reward);
                    total_xp.0 = total_xp.0.saturating_add(reward);
                }
            }
            CombatantId::EnemyBot(entity) => {
                if let Ok((
                    mut stats,
                    mut bot_xp,
                    mut bot_level,
                    mut bot_upgrades,
                    mut bot_evolution,
                    mut bot_health,
                    playstyle,
                )) = bots.get_mut(entity)
                {
                    stats.kills += 1;
                    award_enemy_bot_xp(
                        reward,
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
}

pub fn kill_xp(victim_life_score: u32) -> u32 {
    100u32.saturating_add(victim_life_score / 5).min(300)
}

pub fn reset_life_stats(stats: &mut CombatStats) {
    stats.life_score = 0;
}

pub fn reset_match_stats(stats: &mut CombatStats) {
    *stats = CombatStats::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounty_scales_and_caps() {
        assert_eq!(kill_xp(0), 100);
        assert_eq!(kill_xp(500), 200);
        assert_eq!(kill_xp(10_000), 300);
    }

    #[test]
    fn life_reset_preserves_match_kd() {
        let mut stats = CombatStats {
            life_score: 900,
            kills: 4,
            deaths: 2,
        };
        reset_life_stats(&mut stats);
        assert_eq!((stats.life_score, stats.kills, stats.deaths), (0, 4, 2));
        reset_match_stats(&mut stats);
        assert_eq!((stats.life_score, stats.kills, stats.deaths), (0, 0, 0));
    }
}
