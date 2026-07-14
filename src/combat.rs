use crate::{
    enemy_bot::{
        EnemyBot, EnemyBotBrain, EnemyBotEvolution, EnemyBotHealth, EnemyBotLevel,
        EnemyBotPlaystyle, EnemyBotUpgrades, EnemyBotXp, award_enemy_bot_progress,
    },
    player::Player,
    rng::Rng,
    shape::{TotalXp, Xp},
};
use bevy::prelude::*;

pub const MAX_KILL_XP: u32 = 900;

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CombatStats {
    pub life_score: u32,
    pub kills: u32,
    pub deaths: u32,
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LifeGeneration(pub u32);

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
            &EnemyBotBrain,
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
            CombatantId::Player => player_stats.single().map_or(0, |stats| stats.life_score),
            CombatantId::EnemyBot(entity) => bots.get(entity).map_or(0, |bot| bot.0.life_score),
        };
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
        let score_reward = kill_score(victim_score);
        let xp_reward = kill_xp(victim_score);
        match killer {
            CombatantId::Player => {
                let Ok(mut stats) = player_stats.single_mut() else {
                    continue;
                };
                stats.kills += 1;
                stats.life_score = stats.life_score.saturating_add(score_reward);
                xp.0 = xp.0.saturating_add(xp_reward);
                total_xp.0 = total_xp.0.saturating_add(xp_reward);
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
                    brain,
                )) = bots.get_mut(entity)
                {
                    stats.kills += 1;
                    award_enemy_bot_progress(
                        xp_reward,
                        score_reward,
                        &mut bot_xp,
                        &mut bot_level,
                        &mut bot_upgrades,
                        &mut bot_evolution,
                        &mut bot_health,
                        playstyle,
                        brain.adaptive_evolution_tag(),
                        &mut stats,
                        &mut rng,
                    );
                }
            }
        }
    }
}

pub fn kill_score(victim_life_score: u32) -> u32 {
    victim_life_score / 2
}

pub fn kill_xp(victim_life_score: u32) -> u32 {
    kill_score(victim_life_score).min(MAX_KILL_XP)
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

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(CombatDeathQueue::default())
            .insert_resource(Xp(0))
            .insert_resource(TotalXp(0))
            .insert_resource(Rng::new(7))
            .add_systems(Update, resolve_combat_deaths);
        app.world_mut().spawn((Player, CombatStats::default()));
        app
    }

    fn spawn_bot(app: &mut App, stats: CombatStats) -> Entity {
        app.world_mut()
            .spawn((
                EnemyBot,
                stats,
                EnemyBotXp::default(),
                EnemyBotLevel(1),
                EnemyBotUpgrades(crate::hud::UpgradeState::default()),
                EnemyBotEvolution(crate::evolution::EvolutionState::default()),
                EnemyBotHealth {
                    current: 0.0,
                    max: 50.0,
                },
                EnemyBotPlaystyle::Brawler,
                EnemyBotBrain::default(),
            ))
            .id()
    }

    #[test]
    fn bounty_scales_and_caps() {
        assert_eq!(kill_score(0), 0);
        assert_eq!(kill_score(500), 250);
        assert_eq!(kill_score(10_000), 5_000);
        assert_eq!(kill_xp(0), 0);
        assert_eq!(kill_xp(500), 250);
        assert_eq!(kill_xp(1_800), 900);
        assert_eq!(kill_xp(10_000), 900);
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

    #[test]
    fn simultaneous_trade_awards_both_combatants() {
        let mut app = test_app();
        let bot = spawn_bot(&mut app, CombatStats::default());
        {
            let mut queue = app.world_mut().resource_mut::<CombatDeathQueue>();
            queue.record(CombatantId::Player, Some(CombatantId::EnemyBot(bot)));
            queue.record(CombatantId::EnemyBot(bot), Some(CombatantId::Player));
        }

        app.update();

        let mut player_stats = app
            .world_mut()
            .query_filtered::<&CombatStats, With<Player>>();
        let player_stats = player_stats.single(app.world()).unwrap();
        let bot_stats = app.world().get::<CombatStats>(bot).unwrap();
        assert_eq!((player_stats.kills, player_stats.deaths), (1, 1));
        assert_eq!((bot_stats.kills, bot_stats.deaths), (1, 1));
    }

    #[test]
    fn player_kill_adds_half_score_but_caps_xp() {
        let mut app = test_app();
        let bot = spawn_bot(
            &mut app,
            CombatStats {
                life_score: 4_000,
                ..default()
            },
        );
        app.world_mut()
            .resource_mut::<CombatDeathQueue>()
            .record(CombatantId::EnemyBot(bot), Some(CombatantId::Player));

        app.update();

        let player_stats = app
            .world_mut()
            .query_filtered::<&CombatStats, With<Player>>()
            .single(app.world())
            .unwrap();
        assert_eq!(player_stats.life_score, 2_000);
        assert_eq!(app.world().resource::<Xp>().0, 900);
        assert_eq!(app.world().resource::<TotalXp>().0, 900);
    }
}
