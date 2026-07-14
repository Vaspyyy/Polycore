use crate::{
    combat::CombatStats,
    enemy_bot::{EnemyBot, EnemyBotBrain, EnemyBotHealth, EnemyBotName},
    evolution::EvolutionState,
    menu::{GamePhase, PlayerName, RunStats},
    player::{Player, PlayerHealth},
    profile::{AchievementId, Profile},
    rng::Rng,
    shape::Level,
};
use bevy::prelude::*;
use std::cmp::Ordering;

const CHALLENGE_MIN_DELAY: f32 = 20.0;
const CHALLENGE_DELAY_RANGE: f32 = 10.0;
const CHALLENGE_MAX_DISTANCE: f32 = 1_200.0;
const CHALLENGE_BREAK_DISTANCE: f32 = 1_400.0;
const CHALLENGE_DURATION: f32 = 10.0;
const CROWN_PROFILE_COMMIT_INTERVAL: f32 = 5.0;
const LEADER_INDICATOR_MARGIN: f32 = 52.0;

#[derive(Clone, Copy, Debug)]
pub struct RankCandidate {
    pub entity: Entity,
    pub score: u32,
    pub alive: bool,
}

pub fn unique_positive_leader(candidates: &[RankCandidate]) -> Option<Entity> {
    let best_score = candidates
        .iter()
        .filter(|candidate| candidate.alive && candidate.score > 0)
        .map(|candidate| candidate.score)
        .max()?;
    let mut leaders = candidates
        .iter()
        .filter(|candidate| candidate.alive && candidate.score == best_score);
    let leader = leaders.next()?.entity;
    leaders.next().is_none().then_some(leader)
}

pub fn compare_rank(a: (u32, u32, u32, u64), b: (u32, u32, u32, u64)) -> Ordering {
    b.0.cmp(&a.0)
        .then_with(|| b.1.cmp(&a.1))
        .then_with(|| a.2.cmp(&b.2))
        .then_with(|| a.3.cmp(&b.3))
}

#[derive(Resource, Debug, Default)]
pub struct DominanceState {
    pub leader: Option<Entity>,
    pub leader_name: String,
    pub streak_secs: f32,
    pending_player_crown_secs: f32,
    player_was_leader: bool,
}

#[derive(Component)]
pub struct CrownHolder;

#[derive(Component)]
pub struct CrownMarker {
    owner: Entity,
}

#[derive(Component, Clone, Copy)]
pub struct LeaderChallenger {
    pub target: Entity,
    pub remaining: f32,
}

#[derive(Resource)]
pub struct LeaderChallengeDirector {
    timer: f32,
    challenger: Option<Entity>,
}

impl Default for LeaderChallengeDirector {
    fn default() -> Self {
        Self {
            timer: CHALLENGE_MIN_DELAY,
            challenger: None,
        }
    }
}

#[derive(Resource, Default)]
pub struct PlayerShapeKills(pub u32);

#[derive(Message, Clone, Copy)]
pub struct AchievementUnlocked(pub AchievementId);

#[derive(Component)]
pub struct CrownHudText;

#[derive(Component)]
pub struct LeaderIndicatorRoot;

#[derive(Component)]
pub struct LeaderIndicatorArrow;

#[derive(Component)]
pub struct LeaderIndicatorText;

#[derive(Component)]
pub struct AchievementToast {
    remaining: f32,
}

#[derive(Resource, Default)]
pub struct ProfileProgressTracker {
    match_kills: u32,
    match_deaths: u32,
    shape_kills: u32,
    life_start_kills: u32,
}

pub fn setup_dominance_ui(
    mut commands: Commands,
    tanks: Query<Entity, Or<(With<Player>, With<EnemyBot>)>>,
) {
    for owner in &tanks {
        commands.entity(owner).with_children(|tank| {
            tank.spawn((
                CrownMarker { owner },
                Text2d::new("CROWN"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(255, 215, 92)),
                TextShadow {
                    offset: Vec2::new(1.5, 1.5),
                    color: Color::BLACK,
                },
                Transform::from_xyz(0.0, 53.0, 5.0),
                Visibility::Hidden,
            ));
        });
    }

    commands.spawn((
        CrownHudText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(Color::srgb_u8(255, 222, 110)),
        TextShadow {
            offset: Vec2::new(1.5, 1.5),
            color: Color::BLACK,
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(14.0),
            left: Val::Percent(50.0),
            ..default()
        },
        UiTransform::from_translation(Val2::px(-90.0, 0.0)),
        Visibility::Hidden,
        Pickable::IGNORE,
    ));

    commands
        .spawn((
            LeaderIndicatorRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(156.0),
                height: Val::Px(34.0),
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.03, 0.05, 0.76)),
            Visibility::Hidden,
            GlobalZIndex(30),
            Pickable::IGNORE,
        ))
        .with_children(|indicator| {
            indicator.spawn((
                LeaderIndicatorArrow,
                Text::new(">"),
                TextFont {
                    font_size: FontSize::Px(21.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(255, 215, 92)),
            ));
            indicator.spawn((
                LeaderIndicatorText,
                Text::new("Leader"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

#[allow(clippy::too_many_arguments)]
pub fn update_dominance(
    time: Res<Time>,
    phase: Res<GamePhase>,
    mut commands: Commands,
    mut state: ResMut<DominanceState>,
    player_name: Res<PlayerName>,
    player: Query<(Entity, &PlayerHealth, &CombatStats), (With<Player>, Without<EnemyBot>)>,
    bots: Query<
        (Entity, &EnemyBotName, &EnemyBotHealth, &CombatStats),
        (With<EnemyBot>, Without<Player>),
    >,
    current_holders: Query<Entity, With<CrownHolder>>,
    mut markers: Query<(&CrownMarker, &mut Visibility), (With<CrownMarker>, Without<CrownHudText>)>,
    mut hud: Query<(&mut Text, &mut Visibility), (With<CrownHudText>, Without<CrownMarker>)>,
    mut profile: ResMut<Profile>,
    mut achievements: MessageWriter<AchievementUnlocked>,
) {
    if *phase == GamePhase::Paused {
        return;
    }
    if !matches!(
        *phase,
        GamePhase::Playing | GamePhase::Paused | GamePhase::Dead
    ) {
        if phase.is_changed() {
            commit_crown_progress(&mut state, &mut profile);
            state.leader = None;
            state.leader_name.clear();
            state.streak_secs = 0.0;
            state.player_was_leader = false;
            for entity in &current_holders {
                commands.entity(entity).remove::<CrownHolder>();
            }
        }
        return;
    }

    let mut candidates = Vec::with_capacity(bots.iter().len() + 1);
    let player_snapshot = player.single().ok();
    if let Some((entity, health, stats)) = player_snapshot {
        candidates.push(RankCandidate {
            entity,
            score: stats.life_score,
            alive: health.current > 0.0,
        });
    }
    candidates.extend(bots.iter().map(|(entity, _, health, stats)| RankCandidate {
        entity,
        score: stats.life_score,
        alive: health.current > 0.0,
    }));
    let leader = unique_positive_leader(&candidates);
    if state.leader == leader {
        if leader.is_some() {
            state.streak_secs += time.delta_secs();
        }
    } else {
        if state.player_was_leader {
            commit_crown_progress(&mut state, &mut profile);
        }
        for entity in &current_holders {
            commands.entity(entity).remove::<CrownHolder>();
        }
        state.leader = leader;
        state.streak_secs = 0.0;
        state.leader_name = leader.map_or_else(String::new, |leader_entity| {
            if player_snapshot.is_some_and(|(entity, _, _)| entity == leader_entity) {
                display_player_name(&player_name)
            } else {
                bots.get(leader_entity)
                    .map_or_else(|_| "Leader".to_string(), |(_, name, _, _)| name.0.clone())
            }
        });
        if let Some(entity) = leader {
            commands.entity(entity).insert(CrownHolder);
        }
    }

    for (marker, mut visibility) in &mut markers {
        let next = if Some(marker.owner) == state.leader {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != next {
            *visibility = next;
        }
    }

    let player_is_leader =
        player_snapshot.is_some_and(|(entity, _, _)| Some(entity) == state.leader);
    if let Ok((mut text, mut visibility)) = hud.single_mut() {
        if player_is_leader {
            let next = format!(
                "CROWN {:02}:{:02}  BEST {:02}:{:02}",
                state.streak_secs as u32 / 60,
                state.streak_secs as u32 % 60,
                profile
                    .data
                    .records
                    .best_crown_streak_secs
                    .max(state.streak_secs) as u32
                    / 60,
                profile
                    .data
                    .records
                    .best_crown_streak_secs
                    .max(state.streak_secs) as u32
                    % 60,
            );
            if **text != next {
                **text = next;
            }
            if *visibility != Visibility::Visible {
                *visibility = Visibility::Visible;
            }
        } else {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
    }

    if player_is_leader {
        let dt = time.delta_secs();
        state.pending_player_crown_secs += dt;
        if state.pending_player_crown_secs >= CROWN_PROFILE_COMMIT_INTERVAL {
            commit_crown_progress(&mut state, &mut profile);
        }
        if !profile
            .data
            .achievements
            .contains(&AchievementId::ClaimCrown)
        {
            unlock(&mut profile, AchievementId::ClaimCrown, &mut achievements);
        }
        if state.streak_secs >= 30.0
            && !profile
                .data
                .achievements
                .contains(&AchievementId::CrownThirty)
        {
            unlock(&mut profile, AchievementId::CrownThirty, &mut achievements);
        }
        if state.streak_secs >= 120.0
            && !profile
                .data
                .achievements
                .contains(&AchievementId::CrownOneTwenty)
        {
            unlock(
                &mut profile,
                AchievementId::CrownOneTwenty,
                &mut achievements,
            );
        }
    }
    state.player_was_leader = player_is_leader;
}

fn commit_crown_progress(state: &mut DominanceState, profile: &mut Profile) {
    if state.pending_player_crown_secs <= 0.0 {
        return;
    }
    profile.data.records.total_crown_time_secs += state.pending_player_crown_secs;
    profile.data.records.best_crown_streak_secs = profile
        .data
        .records
        .best_crown_streak_secs
        .max(state.streak_secs);
    state.pending_player_crown_secs = 0.0;
    profile.mark_dirty();
}

pub fn flush_crown_progress_on_exit(
    mut exits: MessageReader<bevy::app::AppExit>,
    mut state: ResMut<DominanceState>,
    mut profile: ResMut<Profile>,
    run_stats: Res<RunStats>,
    player_stats: Query<&CombatStats, With<Player>>,
) {
    if exits.read().next().is_none() {
        return;
    }
    commit_crown_progress(&mut state, &mut profile);
    if run_stats.time_alive > profile.data.records.longest_life_secs {
        profile.data.records.longest_life_secs = run_stats.time_alive;
        profile.mark_dirty();
    }
    if let Ok(stats) = player_stats.single()
        && stats.life_score > profile.data.records.best_life_score
    {
        profile.data.records.best_life_score = stats.life_score;
        profile.mark_dirty();
    }
    let _ = profile.save();
}

pub fn update_leader_indicator(
    state: Res<DominanceState>,
    phase: Res<GamePhase>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    player: Query<(Entity, &Transform), (With<Player>, Without<EnemyBot>)>,
    transforms: Query<&GlobalTransform, Or<(With<Player>, With<EnemyBot>)>>,
    mut root: Query<(&mut Node, &mut Visibility), With<LeaderIndicatorRoot>>,
    mut arrow: Query<&mut UiTransform, With<LeaderIndicatorArrow>>,
    mut label: Query<&mut Text, With<LeaderIndicatorText>>,
) {
    let Ok((mut node, mut visibility)) = root.single_mut() else {
        return;
    };
    let Ok((player_entity, player_transform)) = player.single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let Some(leader) = state.leader.filter(|leader| *leader != player_entity) else {
        *visibility = Visibility::Hidden;
        return;
    };
    if !matches!(
        *phase,
        GamePhase::Playing | GamePhase::Paused | GamePhase::Dead
    ) {
        *visibility = Visibility::Hidden;
        return;
    }
    let Ok(leader_transform) = transforms.get(leader) else {
        *visibility = Visibility::Hidden;
        return;
    };
    let size = Vec2::new(window.width(), window.height());
    let center = size * 0.5;
    let viewport = camera
        .0
        .world_to_viewport(camera.1, leader_transform.translation())
        .ok();
    if viewport.is_some_and(|point| {
        point.x >= 0.0 && point.y >= 0.0 && point.x <= size.x && point.y <= size.y
    }) {
        *visibility = Visibility::Hidden;
        return;
    }

    let world_delta = leader_transform.translation().xy() - player_transform.translation.xy();
    let direction = viewport
        .map(|point| point - center)
        .filter(|delta| delta.length_squared() > 0.01)
        .unwrap_or(world_delta)
        .normalize_or_zero();
    if direction == Vec2::ZERO {
        *visibility = Visibility::Hidden;
        return;
    }
    let bounds = center - Vec2::splat(LEADER_INDICATOR_MARGIN);
    let scale_x = if direction.x.abs() > 0.001 {
        bounds.x / direction.x.abs()
    } else {
        f32::INFINITY
    };
    let scale_y = if direction.y.abs() > 0.001 {
        bounds.y / direction.y.abs()
    } else {
        f32::INFINITY
    };
    let position = center + direction * scale_x.min(scale_y);
    node.left = Val::Px((position.x - 78.0).clamp(4.0, size.x - 160.0));
    node.top = Val::Px((position.y - 17.0).clamp(4.0, size.y - 38.0));
    *visibility = Visibility::Visible;
    if let Ok(mut transform) = arrow.single_mut() {
        transform.rotation = Rot2::radians(direction.y.atan2(direction.x));
    }
    if let Ok(mut text) = label.single_mut() {
        **text = format!(
            "{}  {}m",
            state.leader_name,
            (world_delta.length() / 10.0).round() as u32 * 10
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_challenge_director(
    time: Res<Time<Fixed>>,
    phase: Res<GamePhase>,
    state: Res<DominanceState>,
    mut director: ResMut<LeaderChallengeDirector>,
    mut commands: Commands,
    mut rng: ResMut<Rng>,
    transforms: Query<&Transform, Or<(With<Player>, With<EnemyBot>)>>,
    bots: Query<(Entity, &Transform, &EnemyBotHealth, &EnemyBotBrain), With<EnemyBot>>,
    challengers: Query<&LeaderChallenger, With<EnemyBot>>,
) {
    if !matches!(
        *phase,
        GamePhase::Playing | GamePhase::Paused | GamePhase::Dead
    ) {
        return;
    }
    let Some(leader) = state.leader else {
        if let Some(entity) = director.challenger.take() {
            commands.entity(entity).remove::<LeaderChallenger>();
        }
        return;
    };
    let Ok(leader_transform) = transforms.get(leader) else {
        return;
    };
    let dt = time.delta_secs();
    if let Some(entity) = director.challenger {
        let valid = bots.get(entity).is_ok_and(|(_, transform, health, brain)| {
            challengers.get(entity).is_ok_and(|challenge| {
                challenge.target == leader
                    && challenge.remaining > dt
                    && health.current / health.max.max(1.0) >= 0.5
                    && !brain.fleeing
                    && transform
                        .translation
                        .xy()
                        .distance(leader_transform.translation.xy())
                        <= CHALLENGE_BREAK_DISTANCE
            })
        });
        if valid {
            if let Ok(challenge) = challengers.get(entity) {
                commands.entity(entity).insert(LeaderChallenger {
                    remaining: challenge.remaining - dt,
                    ..*challenge
                });
            }
            return;
        }
        commands.entity(entity).remove::<LeaderChallenger>();
        director.challenger = None;
        director.timer = random_challenge_delay(&mut rng);
    }

    director.timer -= dt;
    if director.timer > 0.0 {
        return;
    }
    let eligible = bots
        .iter()
        .filter(|(entity, transform, health, brain)| {
            *entity != leader
                && health.current / health.max.max(1.0) >= 0.7
                && !brain.fleeing
                && brain.retaliation_timer <= 0.0
                && transform
                    .translation
                    .xy()
                    .distance(leader_transform.translation.xy())
                    <= CHALLENGE_MAX_DISTANCE
        })
        .map(|(entity, ..)| entity)
        .collect::<Vec<_>>();
    if eligible.is_empty() {
        director.timer = 5.0;
        return;
    }
    let challenger = eligible[rng.next(eligible.len() as u32) as usize];
    commands.entity(challenger).insert(LeaderChallenger {
        target: leader,
        remaining: CHALLENGE_DURATION,
    });
    director.challenger = Some(challenger);
}

#[allow(clippy::too_many_arguments)]
pub fn update_profile_progress(
    phase: Res<GamePhase>,
    mut profile: ResMut<Profile>,
    mut tracker: ResMut<ProfileProgressTracker>,
    shape_kills: Res<PlayerShapeKills>,
    run_stats: Res<RunStats>,
    level: Res<Level>,
    evolution: Res<EvolutionState>,
    player: Query<(&CombatStats, &PlayerHealth), With<Player>>,
    mut achievements: MessageWriter<AchievementUnlocked>,
) {
    if !matches!(
        *phase,
        GamePhase::Playing | GamePhase::Paused | GamePhase::Dead
    ) {
        return;
    }
    let Ok((stats, _)) = player.single() else {
        return;
    };
    let kill_delta = stats.kills.saturating_sub(tracker.match_kills);
    let death_delta = stats.deaths.saturating_sub(tracker.match_deaths);
    let shape_delta = shape_kills.0.saturating_sub(tracker.shape_kills);
    let life_kills = stats.kills.saturating_sub(tracker.life_start_kills);
    tracker.match_kills = stats.kills;
    tracker.match_deaths = stats.deaths;
    tracker.shape_kills = shape_kills.0;

    let records = &profile.data.records;
    let evolution_is_new = evolution.current_kind.is_level_five()
        && !records
            .used_level_five_evolutions
            .contains(evolution.current_kind.id());
    let record_changed = kill_delta > 0
        || death_delta > 0
        || shape_delta > 0
        || stats.life_score > records.best_life_score
        || life_kills > records.best_life_kills
        || level.0 > records.highest_level
        || evolution_is_new
        || (run_stats.time_alive > records.longest_life_secs
            && (death_delta > 0 || run_stats.time_alive - records.longest_life_secs >= 5.0));
    if record_changed {
        let records = &mut profile.data.records;
        records.lifetime_kills = records.lifetime_kills.saturating_add(kill_delta);
        records.lifetime_deaths = records.lifetime_deaths.saturating_add(death_delta);
        records.shapes_destroyed = records.shapes_destroyed.saturating_add(shape_delta);
        records.best_life_score = records.best_life_score.max(stats.life_score);
        records.longest_life_secs = records.longest_life_secs.max(run_stats.time_alive);
        records.best_life_kills = records.best_life_kills.max(life_kills);
        records.highest_level = records.highest_level.max(level.0);
        if evolution_is_new {
            records
                .used_level_five_evolutions
                .insert(evolution.current_kind.id().to_string());
        }
        profile.mark_dirty();
    }
    if death_delta > 0 {
        tracker.life_start_kills = stats.kills;
    }

    if profile.data.records.shapes_destroyed >= 10
        && !profile
            .data
            .achievements
            .contains(&AchievementId::ShapeHunter)
    {
        unlock(&mut profile, AchievementId::ShapeHunter, &mut achievements);
    }
    if profile.data.records.lifetime_kills >= 1
        && !profile
            .data
            .achievements
            .contains(&AchievementId::FirstKill)
    {
        unlock(&mut profile, AchievementId::FirstKill, &mut achievements);
    }
    if evolution.current_kind != crate::evolution::EvolutionKind::Tank
        && !profile
            .data
            .achievements
            .contains(&AchievementId::FirstEvolution)
    {
        unlock(
            &mut profile,
            AchievementId::FirstEvolution,
            &mut achievements,
        );
    }
    if run_stats.time_alive >= 300.0
        && !profile.data.achievements.contains(&AchievementId::Survivor)
    {
        unlock(&mut profile, AchievementId::Survivor, &mut achievements);
    }
    if stats.life_score >= 1_000
        && !profile
            .data
            .achievements
            .contains(&AchievementId::ScoreThousand)
    {
        unlock(
            &mut profile,
            AchievementId::ScoreThousand,
            &mut achievements,
        );
    }
    if life_kills >= 5
        && !profile
            .data
            .achievements
            .contains(&AchievementId::FiveKillLife)
    {
        unlock(&mut profile, AchievementId::FiveKillLife, &mut achievements);
    }
    if evolution.current_kind.is_advanced()
        && !profile
            .data
            .achievements
            .contains(&AchievementId::AdvancedEvolution)
    {
        unlock(
            &mut profile,
            AchievementId::AdvancedEvolution,
            &mut achievements,
        );
    }
    if profile.data.records.used_level_five_evolutions.len() >= 8
        && !profile
            .data
            .achievements
            .contains(&AchievementId::EvolutionMastery)
    {
        unlock(
            &mut profile,
            AchievementId::EvolutionMastery,
            &mut achievements,
        );
    }
}

pub fn show_achievement_toasts(
    mut commands: Commands,
    mut messages: MessageReader<AchievementUnlocked>,
) {
    for message in messages.read() {
        commands.spawn((
            AchievementToast { remaining: 4.0 },
            Text::new(format!(
                "ACHIEVEMENT: {}\nPalette unlocked: {}",
                message.0.title(),
                message.0.palette().name()
            )),
            TextFont {
                font_size: FontSize::Px(15.0),
                ..default()
            },
            TextColor(Color::WHITE),
            TextShadow {
                offset: Vec2::new(1.5, 1.5),
                color: Color::BLACK,
            },
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(58.0),
                left: Val::Percent(50.0),
                width: Val::Px(280.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                ..default()
            },
            UiTransform::from_translation(Val2::px(-140.0, 0.0)),
            BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.92)),
            BorderColor::all(Color::srgb_u8(255, 215, 92)),
            GlobalZIndex(50),
            Pickable::IGNORE,
        ));
    }
}

pub fn tick_achievement_toasts(
    time: Res<Time>,
    mut commands: Commands,
    mut toasts: Query<(Entity, &mut AchievementToast)>,
) {
    for (entity, mut toast) in &mut toasts {
        toast.remaining -= time.delta_secs();
        if toast.remaining <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn unlock(
    profile: &mut Profile,
    achievement: AchievementId,
    messages: &mut MessageWriter<AchievementUnlocked>,
) {
    if profile.unlock(achievement) {
        messages.write(AchievementUnlocked(achievement));
    }
}

fn display_player_name(name: &PlayerName) -> String {
    if name.0.trim().is_empty() {
        "Player".to_string()
    } else {
        name.0.clone()
    }
}

fn random_challenge_delay(rng: &mut Rng) -> f32 {
    CHALLENGE_MIN_DELAY + rng.next(10_001) as f32 / 10_000.0 * CHALLENGE_DELAY_RANGE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_break_distance_has_outward_hysteresis() {
        assert!(std::hint::black_box(CHALLENGE_BREAK_DISTANCE) > CHALLENGE_MAX_DISTANCE);
    }

    #[test]
    fn crown_requires_one_living_positive_leader() {
        let a = Entity::from_bits(1);
        let b = Entity::from_bits(2);
        assert_eq!(
            unique_positive_leader(&[
                RankCandidate {
                    entity: a,
                    score: 10,
                    alive: true
                },
                RankCandidate {
                    entity: b,
                    score: 8,
                    alive: true
                },
            ]),
            Some(a)
        );
        assert_eq!(
            unique_positive_leader(&[
                RankCandidate {
                    entity: a,
                    score: 10,
                    alive: true
                },
                RankCandidate {
                    entity: b,
                    score: 10,
                    alive: true
                },
            ]),
            None
        );
        assert_eq!(
            unique_positive_leader(&[RankCandidate {
                entity: a,
                score: 0,
                alive: true
            }]),
            None
        );
        assert_eq!(
            unique_positive_leader(&[RankCandidate {
                entity: a,
                score: 10,
                alive: false
            }]),
            None
        );
    }

    #[test]
    fn challenge_delays_stay_inside_requested_window() {
        let mut rng = Rng::new(7);
        for _ in 0..100 {
            let delay = random_challenge_delay(&mut rng);
            assert!((20.0..=30.0).contains(&delay));
        }
    }
}
