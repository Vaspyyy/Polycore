use bevy::prelude::*;
use crate::{constants, shape::{Level, TotalXp, Xp}};

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct LevelText;

#[derive(Component)]
pub struct XpText;

#[derive(Component)]
pub struct XpFill;

#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
pub struct PlayerNameText;

pub fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(18.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Px(112.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::End,
                row_gap: Val::Px(4.0),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("<PLAYER NAME>"),
                TextFont {
                    font_size: FontSize::Px(32.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                PlayerNameText,
            ));

            root.spawn((
                Node {
                    width: Val::Px(460.0),
                    height: Val::Px(22.0),
                    border: UiRect::all(Val::Px(3.0)),
                    border_radius: BorderRadius::all(Val::Px(11.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.35, 0.92, 0.58, 1.0)),
                BorderColor::all(Color::srgba(0.18, 0.19, 0.20, 1.0)),
            ))
            .with_children(|score_bar| {
                score_bar.spawn((
                    Text::new("Score: 0"),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    TextShadow {
                        offset: Vec2::new(1.5, 1.5),
                        color: Color::BLACK,
                    },
                    ScoreText,
                ));
            });

            root.spawn(Node {
                width: Val::Px(620.0),
                height: Val::Px(28.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Node {
                        width: Val::Px(112.0),
                        height: Val::Px(24.0),
                        border: UiRect::all(Val::Px(3.0)),
                        border_radius: BorderRadius::all(Val::Px(12.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.18, 0.19, 0.20, 1.0)),
                    BorderColor::all(Color::srgba(0.18, 0.19, 0.20, 1.0)),
                ))
                .with_children(|level_badge| {
                    level_badge.spawn((
                        Text::new("Lvl 1"),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        LevelText,
                    ));
                });

                row.spawn((
                    Node {
                        width: Val::Px(500.0),
                        height: Val::Px(24.0),
                        border: UiRect::all(Val::Px(3.0)),
                        border_radius: BorderRadius::all(Val::Px(12.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.18, 0.19, 0.20, 1.0)),
                    BorderColor::all(Color::srgba(0.18, 0.19, 0.20, 1.0)),
                ))
                .with_children(|xp_bar| {
                    xp_bar.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            height: Val::Percent(100.0),
                            width: Val::Percent(0.0),
                            border_radius: BorderRadius::all(Val::Px(12.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.98, 0.86, 0.38, 1.0)),
                        XpFill,
                    ));
                    xp_bar.spawn((
                        Text::new("XP 0 / 100"),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TextShadow {
                            offset: Vec2::new(1.5, 1.5),
                            color: Color::BLACK,
                        },
                        XpText,
                    ));
                });
            });
        });
}

pub fn update_hud(
    xp: Res<Xp>,
    total_xp: Res<TotalXp>,
    level: Res<Level>,
    mut text_query: Query<(
        &mut Text,
        Option<&ScoreText>,
        Option<&LevelText>,
        Option<&XpText>,
    )>,
    mut xp_fill: Query<&mut Node, With<XpFill>>,
) {
    if !(xp.is_changed() || total_xp.is_changed() || level.is_changed()) {
        return;
    }

    let xp_percent = (xp.0 as f32 / constants::XP_PER_LEVEL as f32 * 100.0).clamp(0.0, 100.0);

    for (mut text, score_marker, level_marker, xp_marker) in text_query.iter_mut() {
        if score_marker.is_some() {
            **text = format!("Score: {}", total_xp.0);
        } else if level_marker.is_some() {
            **text = format!("Lvl {}", level.0);
        } else if xp_marker.is_some() {
            **text = format!("XP {} / {}", xp.0, constants::XP_PER_LEVEL);
        }
    }
    for mut node in xp_fill.iter_mut() {
        node.width = Val::Percent(xp_percent);
    }
}
