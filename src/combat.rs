use crate::{
    enemy_bot::{
        EnemyBot, EnemyBotEvolution, EnemyBotHealth, EnemyBotLevel, EnemyBotPlaystyle,
        EnemyBotUpgrades, EnemyBotXp, award_enemy_bot_xp,
    },
    player::Player,
    rng::Rng,
    shape::{TotalXp, Xp},
};
use bevy::prelude::*;

pub const TANK_KILL_XP: u32 = 100;

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CombatStats {
    pub score: u32,
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
    mut player_stats: Query<&mut CombatStats, (With<Player>, Without<EnemyBot>)>,
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
        match death.victim {
            CombatantId::Player => {
                if let Ok(mut stats) = player_stats.single_mut() {
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
        match killer {
            CombatantId::Player => {
                if let Ok(mut stats) = player_stats.single_mut() {
                    stats.kills += 1;
                    stats.score += TANK_KILL_XP;
                }
                xp.0 += TANK_KILL_XP;
                total_xp.0 += TANK_KILL_XP;
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
                        TANK_KILL_XP,
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
