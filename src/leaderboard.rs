use crate::{
    combat::CombatStats,
    constants,
    dominance::DominanceState,
    enemy_bot::{BOT_COUNT, EnemyBot, EnemyBotEvolution, EnemyBotHealth, EnemyBotName},
    evolution::{EvolutionKind, EvolutionState},
    menu::{GamePhase, PlayerName},
    palette::{BOT_PALETTES, BotPalette, PALETTES},
    player::{Player, PlayerHealth},
    profile::Profile,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::hash::{DefaultHasher, Hash, Hasher};

const LEADERBOARD_ENTRY_COUNT: usize = BOT_COUNT + 1;
const LEADERBOARD_WIDTH: f32 = 334.0;
const LEADERBOARD_OPEN_RIGHT: f32 = 14.0;
const LEADERBOARD_CLOSED_RIGHT: f32 = -(LEADERBOARD_WIDTH + 8.0);
const LEADERBOARD_SLIDE_SPEED: f32 = 1_100.0;
const ROW_HEIGHT: f32 = 34.0;
const ICON_BARREL_SCALE: f32 = 0.38;
const ICON_AIM_ANGLE: f32 = 0.62;

#[derive(Component)]
pub struct LeaderboardRoot {
    collapsed: bool,
    right: f32,
}

#[derive(Component)]
pub struct LeaderboardToggle;

#[derive(Component)]
pub struct LeaderboardToggleIcon;

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

#[derive(SystemParam)]
pub struct LeaderboardUi<'w, 's> {
    rows: Query<
        'w,
        's,
        (&'static LeaderboardRow, &'static mut BackgroundColor),
        (Without<LeaderboardIconBody>, Without<LeaderboardIconBarrel>),
    >,
    texts: Query<
        'w,
        's,
        (
            &'static mut Text,
            &'static mut TextColor,
            Option<&'static LeaderboardRank>,
            Option<&'static LeaderboardName>,
            Option<&'static LeaderboardKd>,
            Option<&'static LeaderboardScore>,
        ),
    >,
    bodies: Query<
        'w,
        's,
        (
            &'static LeaderboardIconBody,
            &'static mut Node,
            &'static mut UiTransform,
            &'static mut BackgroundColor,
            &'static mut BorderColor,
        ),
        (Without<LeaderboardIconBarrel>, Without<LeaderboardRow>),
    >,
    barrels: Query<
        'w,
        's,
        (
            &'static LeaderboardIconBarrel,
            &'static mut Node,
            &'static mut UiTransform,
            &'static mut BackgroundColor,
            &'static mut Visibility,
        ),
        (Without<LeaderboardIconBody>, Without<LeaderboardRow>),
    >,
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
    is_crowned: bool,
    body_color: [f32; 4],
    barrel_color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LeaderboardProbe {
    stable_id: u64,
    name_hash: u64,
    score: u32,
    kills: u32,
    deaths: u32,
    evolution: EvolutionKind,
    is_player: bool,
    is_alive: bool,
    is_crowned: bool,
    color_key: u8,
}

fn leaderboard_name_hash(name: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

pub fn setup_leaderboard(mut commands: Commands) {
    commands
        .spawn((
            LeaderboardRoot {
                collapsed: false,
                right: LEADERBOARD_OPEN_RIGHT,
            },
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(LEADERBOARD_OPEN_RIGHT),
                top: Val::Px(14.0),
                width: Val::Px(LEADERBOARD_WIDTH),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(2.0),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.037, 0.055, 0.70)),
            BorderColor::all(Color::srgba(0.23, 0.25, 0.34, 0.82)),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                Button,
                LeaderboardToggle,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(-31.0),
                    top: Val::Px(7.0),
                    width: Val::Px(27.0),
                    height: Val::Px(27.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.035, 0.037, 0.055, 0.74)),
                BorderColor::all(Color::srgba(0.23, 0.25, 0.34, 0.88)),
            ))
            .with_children(|toggle| {
                toggle.spawn((
                    LeaderboardToggleIcon,
                    Text::new(">"),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::srgba(0.88, 0.90, 0.96, 1.0)),
                ));
            });

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
                    BackgroundColor(Color::srgba(0.12, 0.13, 0.17, 0.74)),
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
    let visibility = if matches!(
        *phase,
        GamePhase::Playing | GamePhase::Paused | GamePhase::Dead
    ) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut current in root.iter_mut() {
        *current = visibility;
    }
}

pub fn handle_leaderboard_toggle(
    mut root: Single<&mut LeaderboardRoot>,
    mut buttons: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<LeaderboardToggle>),
    >,
    mut icons: Query<&mut Text, With<LeaderboardToggleIcon>>,
) {
    for (interaction, mut background) in buttons.iter_mut() {
        *background = match *interaction {
            Interaction::Pressed => {
                root.collapsed = !root.collapsed;
                for mut icon in icons.iter_mut() {
                    **icon = if root.collapsed { "<" } else { ">" }.to_string();
                }
                BackgroundColor(Color::srgba(0.16, 0.18, 0.25, 0.88))
            }
            Interaction::Hovered => BackgroundColor(Color::srgba(0.10, 0.11, 0.16, 0.84)),
            Interaction::None => BackgroundColor(Color::srgba(0.035, 0.037, 0.055, 0.74)),
        };
    }
}

pub fn animate_leaderboard(time: Res<Time>, mut root: Single<(&mut LeaderboardRoot, &mut Node)>) {
    let (state, node) = &mut *root;
    let target = if state.collapsed {
        LEADERBOARD_CLOSED_RIGHT
    } else {
        LEADERBOARD_OPEN_RIGHT
    };
    let max_delta = LEADERBOARD_SLIDE_SPEED * time.delta_secs();
    if (state.right - target).abs() <= f32::EPSILON {
        return;
    }
    state.right = move_towards(state.right, target, max_delta);
    node.right = Val::Px(state.right);
}

fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    current + (target - current).clamp(-max_delta, max_delta)
}

#[allow(clippy::type_complexity)]
pub fn update_leaderboard(
    player_name: Res<PlayerName>,
    player_evolution: Res<EvolutionState>,
    dominance: Res<DominanceState>,
    profile: Res<Profile>,
    player: Query<(Entity, &PlayerHealth, &CombatStats), With<Player>>,
    bots: Query<
        (
            Entity,
            &EnemyBotName,
            &EnemyBotHealth,
            &CombatStats,
            &EnemyBotEvolution,
            &BotPalette,
        ),
        With<EnemyBot>,
    >,
    mut ui: LeaderboardUi,
    mut current_probe: Local<Vec<LeaderboardProbe>>,
    mut last_probe: Local<Vec<LeaderboardProbe>>,
) {
    current_probe.clear();
    if let Ok((entity, health, stats)) = player.single() {
        let display_name = if player_name.0.is_empty() {
            "Player"
        } else {
            &player_name.0
        };
        current_probe.push(LeaderboardProbe {
            stable_id: entity.to_bits(),
            name_hash: leaderboard_name_hash(display_name),
            score: stats.life_score,
            kills: stats.kills,
            deaths: stats.deaths,
            evolution: player_evolution.current_kind,
            is_player: true,
            is_alive: health.current > 0.0,
            is_crowned: dominance.leader == Some(entity),
            color_key: profile.data.selected_palette as u8,
        });
    }
    current_probe.extend(bots.iter().map(
        |(entity, name, health, stats, evolution, palette_id)| LeaderboardProbe {
            stable_id: entity.to_bits(),
            name_hash: leaderboard_name_hash(&name.0),
            score: stats.life_score,
            kills: stats.kills,
            deaths: stats.deaths,
            evolution: evolution.0.current_kind,
            is_player: false,
            is_alive: health.current > 0.0,
            is_crowned: dominance.leader == Some(entity),
            color_key: 32 + palette_id.0 as u8,
        },
    ));
    current_probe.sort_by(|a, b| {
        crate::dominance::compare_rank(
            (a.score, a.kills, a.deaths, a.stable_id),
            (b.score, b.kills, b.deaths, b.stable_id),
        )
    });
    if *last_probe == *current_probe {
        return;
    }
    last_probe.clear();
    last_probe.extend(current_probe.iter().copied());

    let mut entries = Vec::with_capacity(LEADERBOARD_ENTRY_COUNT);
    if let Ok((entity, health, stats)) = player.single() {
        let palette_id = profile.data.selected_palette;
        let palette = PALETTES[palette_id as usize];
        entries.push(LeaderboardEntry {
            stable_id: entity.to_bits(),
            name: if player_name.0.is_empty() {
                "Player".to_string()
            } else {
                player_name.0.clone()
            },
            score: stats.life_score,
            kills: stats.kills,
            deaths: stats.deaths,
            evolution: player_evolution.clone(),
            is_player: true,
            is_alive: health.current > 0.0,
            is_crowned: dominance.leader == Some(entity),
            body_color: palette.body,
            barrel_color: palette.barrel,
        });
    }
    entries.extend(
        bots.iter()
            .map(|(entity, name, health, stats, evolution, palette_id)| {
                let palette = BOT_PALETTES[palette_id.0 % BOT_PALETTES.len()];
                LeaderboardEntry {
                    stable_id: entity.to_bits(),
                    name: name.0.clone(),
                    score: stats.life_score,
                    kills: stats.kills,
                    deaths: stats.deaths,
                    evolution: evolution.0.clone(),
                    is_player: false,
                    is_alive: health.current > 0.0,
                    is_crowned: dominance.leader == Some(entity),
                    body_color: palette.body,
                    barrel_color: palette.barrel,
                }
            }),
    );
    entries.sort_by(|a, b| {
        crate::dominance::compare_rank(
            (a.score, a.kills, a.deaths, a.stable_id),
            (b.score, b.kills, b.deaths, b.stable_id),
        )
    });

    for (row, mut background) in ui.rows.iter_mut() {
        let Some(entry) = entries.get(row.0) else {
            continue;
        };
        *background = BackgroundColor(if !entry.is_alive {
            Color::srgba(0.075, 0.078, 0.095, 0.58)
        } else if entry.is_player {
            Color::srgba(0.10, 0.26, 0.39, 0.78)
        } else if row.0 % 2 == 0 {
            Color::srgba(0.14, 0.145, 0.19, 0.76)
        } else {
            Color::srgba(0.105, 0.11, 0.15, 0.76)
        });
    }

    for (mut text, mut color, rank, name, kd, score) in ui.texts.iter_mut() {
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
            if entry.is_crowned {
                format!("^ {}", entry.name)
            } else {
                entry.name.clone()
            }
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

    for (marker, mut node, mut transform, mut color, mut border) in ui.bodies.iter_mut() {
        let Some(entry) = entries.get(marker.0) else {
            continue;
        };
        let mut size = 18.0 * entry.evolution.body_scale();
        if entry.evolution.current_kind.base() == EvolutionKind::Guard {
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
        node.border_radius = if entry.evolution.current_kind.base() == EvolutionKind::RamCore {
            BorderRadius::all(Val::Percent(18.0))
        } else {
            BorderRadius::all(Val::Percent(50.0))
        };
        *transform = if entry.evolution.current_kind.base() == EvolutionKind::RamCore {
            UiTransform::from_rotation(Rot2::radians(0.52))
        } else {
            UiTransform::default()
        };
        let tank_color = entry.body_color;
        *color = BackgroundColor(Color::srgba(
            tank_color[0],
            tank_color[1],
            tank_color[2],
            if entry.is_alive { 1.0 } else { 0.45 },
        ));
    }

    for (marker, mut node, mut transform, mut color, mut visibility) in ui.barrels.iter_mut() {
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
            entry.barrel_color[0],
            entry.barrel_color[1],
            entry.barrel_color[2],
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

    #[test]
    fn slide_step_stops_at_its_target() {
        assert_eq!(move_towards(14.0, -342.0, 100.0), -86.0);
        assert_eq!(move_towards(-330.0, -342.0, 100.0), -342.0);
        assert_eq!(move_towards(-50.0, 14.0, 100.0), 14.0);
    }
}
