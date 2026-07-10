use crate::{
    combat::CombatStats,
    constants,
    enemy_bot::{BOT_COUNT, EnemyBot, EnemyBotEvolution, EnemyBotHealth, EnemyBotName},
    evolution::{EvolutionKind, EvolutionState},
    menu::{GamePhase, PlayerName},
    player::{Player, PlayerHealth},
    shape::TotalXp,
};
use bevy::prelude::*;

const LEADERBOARD_ENTRY_COUNT: usize = BOT_COUNT + 1;
const LEADERBOARD_WIDTH: f32 = 334.0;
const ROW_HEIGHT: f32 = 34.0;
const ICON_BARREL_SCALE: f32 = 0.38;
const ICON_AIM_ANGLE: f32 = 0.62;

#[derive(Component)]
pub struct LeaderboardRoot;

#[derive(Component)]
pub(crate) struct LeaderboardRow(usize);

#[derive(Component)]
pub(crate) struct LeaderboardRank(usize);

#[derive(Component)]
pub(crate) struct LeaderboardName(usize);

#[derive(Component)]
pub(crate) struct LeaderboardKd(usize);

#[derive(Component)]
pub(crate) struct LeaderboardScore(usize);

#[derive(Component)]
pub(crate) struct LeaderboardIconBody(usize);

#[derive(Component)]
pub(crate) struct LeaderboardIconBarrel {
    row: usize,
    slot: usize,
}

#[derive(Clone)]
struct LeaderboardEntry {
    stable_id: u64,
    name: String,
    score: u32,
    kills: u32,
    deaths: u32,
    evolution: EvolutionState,
    is_player: bool,
    is_alive: bool,
}

pub fn setup_leaderboard(mut commands: Commands) {
    commands
        .spawn((
            LeaderboardRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(14.0),
                top: Val::Px(14.0),
                width: Val::Px(LEADERBOARD_WIDTH),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(2.0),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.037, 0.055, 0.86)),
            BorderColor::all(Color::srgba(0.23, 0.25, 0.34, 0.95)),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("LEADERBOARD"),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                TextShadow {
                    offset: Vec2::new(1.0, 1.0),
                    color: Color::BLACK,
                },
                Node {
                    height: Val::Px(21.0),
                    align_self: AlignSelf::Center,
                    ..default()
                },
            ));

            root.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Px(17.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|header| {
                header.spawn(header_text("#", 20.0));
                header.spawn(header_text("TANK", 38.0));
                header.spawn(header_text("PLAYER", 126.0));
                header.spawn(header_text("K / D", 58.0));
                header.spawn(header_text("SCORE", 68.0));
            });

            for row in 0..LEADERBOARD_ENTRY_COUNT {
                root.spawn((
                    LeaderboardRow(row),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(ROW_HEIGHT),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.13, 0.17, 0.88)),
                ))
                .with_children(|entry| {
                    entry.spawn((LeaderboardRank(row), row_text("1", 20.0, Justify::Center)));
                    entry
                        .spawn(Node {
                            width: Val::Px(38.0),
                            height: Val::Px(ROW_HEIGHT),
                            position_type: PositionType::Relative,
                            ..default()
                        })
                        .with_children(|icon| {
                            for slot in 0..crate::evolution::MAX_BARRELS {
                                icon.spawn((
                                    LeaderboardIconBarrel { row, slot },
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(3.0),
                                        height: Val::Px(13.0),
                                        border: UiRect::all(Val::Px(1.0)),
                                        border_radius: BorderRadius::all(Val::Px(2.0)),
                                        ..default()
                                    },
                                    UiTransform::default(),
                                    BackgroundColor(Color::srgba(0.48, 0.49, 0.52, 1.0)),
                                    BorderColor::all(Color::BLACK),
                                    if slot == 0 {
                                        Visibility::Visible
                                    } else {
                                        Visibility::Hidden
                                    },
                                ));
                            }
                            icon.spawn((
                                LeaderboardIconBody(row),
                                Node {
                                    position_type: PositionType::Absolute,
                                    width: Val::Px(18.0),
                                    height: Val::Px(18.0),
                                    border: UiRect::all(Val::Px(2.0)),
                                    border_radius: BorderRadius::all(Val::Percent(50.0)),
                                    ..default()
                                },
                                UiTransform::default(),
                                BackgroundColor(Color::srgba(
                                    constants::ENEMY_COLOR[0],
                                    constants::ENEMY_COLOR[1],
                                    constants::ENEMY_COLOR[2],
                                    1.0,
                                )),
                                BorderColor::all(Color::BLACK),
                            ));
                        });
                    entry.spawn((
                        LeaderboardName(row),
                        row_text("Player", 126.0, Justify::Left),
                    ));
                    entry.spawn((LeaderboardKd(row), row_text("0 / 0", 58.0, Justify::Center)));
                    entry.spawn((LeaderboardScore(row), row_text("0", 68.0, Justify::Right)));
                });
            }
        });
}

fn header_text(label: &'static str, width: f32) -> (Text, TextFont, TextColor, Node) {
    (
        Text::new(label),
        TextFont {
            font_size: FontSize::Px(9.0),
            ..default()
        },
        TextColor(Color::srgba(0.66, 0.69, 0.78, 1.0)),
        Node {
            width: Val::Px(width),
            justify_content: JustifyContent::Center,
            ..default()
        },
    )
}

fn row_text(
    label: &'static str,
    width: f32,
    justify: Justify,
) -> (Text, TextFont, TextColor, TextLayout, Node) {
    (
        Text::new(label),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::new(justify, LineBreak::NoWrap),
        Node {
            width: Val::Px(width),
            overflow: Overflow::clip(),
            ..default()
        },
    )
}

pub fn sync_leaderboard_visibility(
    phase: Res<GamePhase>,
    mut root: Query<&mut Visibility, With<LeaderboardRoot>>,
) {
    if !phase.is_changed() {
        return;
    }
    let visibility = if *phase == GamePhase::Playing {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut current in root.iter_mut() {
        *current = visibility;
    }
}

#[allow(clippy::type_complexity)]
pub fn update_leaderboard(
    player_name: Res<PlayerName>,
    player_evolution: Res<EvolutionState>,
    player_score: Res<TotalXp>,
    player: Query<(Entity, &PlayerHealth, &CombatStats), With<Player>>,
    bots: Query<
        (
            Entity,
            &EnemyBotName,
            &EnemyBotHealth,
            &CombatStats,
            &EnemyBotEvolution,
        ),
        With<EnemyBot>,
    >,
    mut rows: Query<
        (&LeaderboardRow, &mut BackgroundColor),
        (Without<LeaderboardIconBody>, Without<LeaderboardIconBarrel>),
    >,
    mut texts: Query<(
        &mut Text,
        &mut TextColor,
        Option<&LeaderboardRank>,
        Option<&LeaderboardName>,
        Option<&LeaderboardKd>,
        Option<&LeaderboardScore>,
    )>,
    mut bodies: Query<
        (
            &LeaderboardIconBody,
            &mut Node,
            &mut UiTransform,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (Without<LeaderboardIconBarrel>, Without<LeaderboardRow>),
    >,
    mut barrels: Query<
        (
            &LeaderboardIconBarrel,
            &mut Node,
            &mut UiTransform,
            &mut BackgroundColor,
            &mut Visibility,
        ),
        (Without<LeaderboardIconBody>, Without<LeaderboardRow>),
    >,
) {
    let mut entries = Vec::with_capacity(LEADERBOARD_ENTRY_COUNT);
    if let Ok((entity, health, stats)) = player.single() {
        entries.push(LeaderboardEntry {
            stable_id: entity.to_bits(),
            name: if player_name.0.is_empty() {
                "Player".to_string()
            } else {
                player_name.0.clone()
            },
            score: player_score.0,
            kills: stats.kills,
            deaths: stats.deaths,
            evolution: player_evolution.clone(),
            is_player: true,
            is_alive: health.current > 0,
        });
    }
    entries.extend(bots.iter().map(
        |(entity, name, health, stats, evolution)| LeaderboardEntry {
            stable_id: entity.to_bits(),
            name: name.0.clone(),
            score: stats.score,
            kills: stats.kills,
            deaths: stats.deaths,
            evolution: evolution.0.clone(),
            is_player: false,
            is_alive: health.current > 0,
        },
    ));
    entries.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.kills.cmp(&a.kills))
            .then_with(|| a.deaths.cmp(&b.deaths))
            .then_with(|| a.stable_id.cmp(&b.stable_id))
    });

    for (row, mut background) in rows.iter_mut() {
        let Some(entry) = entries.get(row.0) else {
            continue;
        };
        *background = BackgroundColor(if !entry.is_alive {
            Color::srgba(0.075, 0.078, 0.095, 0.72)
        } else if entry.is_player {
            Color::srgba(0.10, 0.26, 0.39, 0.92)
        } else if row.0 % 2 == 0 {
            Color::srgba(0.14, 0.145, 0.19, 0.90)
        } else {
            Color::srgba(0.105, 0.11, 0.15, 0.90)
        });
    }

    for (mut text, mut color, rank, name, kd, score) in texts.iter_mut() {
        let slot = rank
            .map(|marker| marker.0)
            .or_else(|| name.map(|marker| marker.0))
            .or_else(|| kd.map(|marker| marker.0))
            .or_else(|| score.map(|marker| marker.0));
        let Some(entry) = slot.and_then(|slot| entries.get(slot)) else {
            continue;
        };
        **text = if rank.is_some() {
            format!("{}", slot.unwrap_or(0) + 1)
        } else if name.is_some() {
            entry.name.clone()
        } else if kd.is_some() {
            format!("{} / {}", entry.kills, entry.deaths)
        } else {
            format_score(entry.score)
        };
        *color = TextColor(if entry.is_alive {
            Color::WHITE
        } else {
            Color::srgba(0.58, 0.60, 0.66, 1.0)
        });
    }

    for (marker, mut node, mut transform, mut color, mut border) in bodies.iter_mut() {
        let Some(entry) = entries.get(marker.0) else {
            continue;
        };
        let mut size = 18.0 * entry.evolution.body_scale();
        if entry.evolution.current_kind == EvolutionKind::Guard {
            size += 3.0;
            node.border = UiRect::all(Val::Px(3.0));
            *border = BorderColor::all(Color::srgba(0.38, 0.22, 0.55, 1.0));
        } else {
            node.border = UiRect::all(Val::Px(2.0));
            *border = BorderColor::all(Color::BLACK);
        }
        node.left = Val::Px((38.0 - size) / 2.0);
        node.top = Val::Px((ROW_HEIGHT - size) / 2.0);
        node.width = Val::Px(size);
        node.height = Val::Px(size);
        node.border_radius = if entry.evolution.current_kind == EvolutionKind::RamCore {
            BorderRadius::all(Val::Percent(18.0))
        } else {
            BorderRadius::all(Val::Percent(50.0))
        };
        *transform = if entry.evolution.current_kind == EvolutionKind::RamCore {
            UiTransform::from_rotation(Rot2::radians(0.52))
        } else {
            UiTransform::default()
        };
        let tank_color = if entry.is_player {
            constants::PLAYER_COLOR
        } else {
            constants::ENEMY_COLOR
        };
        *color = BackgroundColor(Color::srgba(
            tank_color[0],
            tank_color[1],
            tank_color[2],
            if entry.is_alive { 1.0 } else { 0.45 },
        ));
    }

    for (marker, mut node, mut transform, mut color, mut visibility) in barrels.iter_mut() {
        let Some(entry) = entries.get(marker.row) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let Some(spec) = entry.evolution.barrel_specs().get(marker.slot) else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let width = (spec.width * ICON_BARREL_SCALE).max(2.5);
        let length = spec.length * ICON_BARREL_SCALE;
        let body_radius = 9.0 * entry.evolution.body_scale();
        let angle = ICON_AIM_ANGLE + spec.angle_offset;
        let forward = Vec2::new(angle.sin(), -angle.cos());
        let right = Vec2::new(angle.cos(), angle.sin());
        let center = Vec2::new(19.0, ROW_HEIGHT / 2.0)
            + forward * (body_radius - 1.5 + length / 2.0)
            + right * spec.lateral_offset * ICON_BARREL_SCALE;
        node.left = Val::Px(center.x - width / 2.0);
        node.top = Val::Px(center.y - length / 2.0);
        node.width = Val::Px(width);
        node.height = Val::Px(length);
        *transform = UiTransform::from_rotation(Rot2::radians(angle));
        *color = BackgroundColor(Color::srgba(
            constants::BARREL_COLOR[0],
            constants::BARREL_COLOR[1],
            constants::BARREL_COLOR[2],
            if entry.is_alive { 1.0 } else { 0.42 },
        ));
        *visibility = Visibility::Visible;
    }
}

fn format_score(score: u32) -> String {
    if score >= 1_000_000 {
        format!("{:.1}m", score as f32 / 1_000_000.0)
    } else if score >= 10_000 {
        format!("{:.1}k", score as f32 / 1_000.0)
    } else {
        score.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_format_stays_compact() {
        assert_eq!(format_score(950), "950");
        assert_eq!(format_score(12_500), "12.5k");
        assert_eq!(format_score(1_250_000), "1.2m");
    }
}
