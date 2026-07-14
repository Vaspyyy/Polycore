use crate::{
    constants,
    menu::GamePhase,
    rng::Rng,
    shape::{Level, TotalXp, Xp},
};
use bevy::prelude::*;

const UPGRADE_COUNT: usize = 8;
const UPGRADE_MAX_LEVEL: u32 = 8;
const UPGRADE_FILL_SPEED: f32 = 180.0;
const UPGRADE_MENU_WIDTH: f32 = 268.0;
const UPGRADE_ROW_WIDTH: f32 = 224.0;
const UPGRADE_ROW_HEIGHT: f32 = 19.8;
const UPGRADE_ROW_RADIUS: f32 = UPGRADE_ROW_HEIGHT / 2.0;
const HEALTH_REGEN_INDEX: usize = 0;
const MAX_HEALTH_INDEX: usize = 1;
const BODY_DAMAGE_INDEX: usize = 2;
const BULLET_SPEED_INDEX: usize = 3;
const BULLET_PENETRATION_INDEX: usize = 4;
const BULLET_DAMAGE_INDEX: usize = 5;
const RELOAD_INDEX: usize = 6;
const MOVEMENT_SPEED_INDEX: usize = 7;
const UPGRADE_LABELS: [&str; UPGRADE_COUNT] = [
    "Health Regen",
    "Max Health",
    "Body Damage",
    "Bullet Speed",
    "Bullet Penetration",
    "Bullet Damage",
    "Reload",
    "Movement Speed",
];
const UPGRADE_COLORS: [[f32; 4]; UPGRADE_COUNT] = [
    [0.96, 0.58, 0.32, 1.0],
    [0.86, 0.24, 0.90, 1.0],
    [0.64, 0.36, 0.86, 1.0],
    [0.33, 0.51, 0.94, 1.0],
    [0.93, 0.80, 0.27, 1.0],
    [0.93, 0.34, 0.31, 1.0],
    [0.45, 0.92, 0.32, 1.0],
    [0.28, 0.85, 0.86, 1.0],
];
const UPGRADE_KEYS: [KeyCode; UPGRADE_COUNT] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum UpgradeKind {
    HealthRegen = HEALTH_REGEN_INDEX,
    MaxHealth = MAX_HEALTH_INDEX,
    BulletDamage = BULLET_DAMAGE_INDEX,
    MovementSpeed = MOVEMENT_SPEED_INDEX,
}

impl UpgradeKind {
    pub const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Resource, Clone)]
pub struct UpgradeState {
    pub points: u32,
    pub levels: [u32; UPGRADE_COUNT],
}

impl Default for UpgradeState {
    fn default() -> Self {
        Self {
            points: 0,
            levels: [0; UPGRADE_COUNT],
        }
    }
}

impl UpgradeState {
    pub fn add_points(&mut self, amount: u32) {
        if !self.is_capped() {
            self.points = self.points.saturating_add(amount);
        }
    }

    pub fn reset(&mut self) {
        self.points = 0;
        self.levels = [0; UPGRADE_COUNT];
    }

    fn spend_point(&mut self, index: usize) -> bool {
        let Some(level) = self.levels.get_mut(index) else {
            return false;
        };
        if self.points == 0 || *level >= UPGRADE_MAX_LEVEL {
            return false;
        }
        self.points -= 1;
        *level += 1;
        true
    }

    pub fn spend_point_on(&mut self, kind: UpgradeKind) -> bool {
        self.spend_point(kind.index())
    }

    pub fn level_of(&self, kind: UpgradeKind) -> u32 {
        self.levels[kind.index()]
    }

    pub fn is_capped(&self) -> bool {
        self.levels.iter().all(|level| *level >= UPGRADE_MAX_LEVEL)
    }

    pub fn spend_random_point(&mut self, rng: &mut Rng) -> bool {
        if self.points == 0 {
            return false;
        }

        let mut options = [0; UPGRADE_COUNT];
        let mut option_count = 0;
        for (index, level) in self.levels.iter().enumerate() {
            if *level < UPGRADE_MAX_LEVEL {
                options[option_count] = index;
                option_count += 1;
            }
        }
        if option_count == 0 {
            return false;
        }

        let option_index = rng.next(option_count as u32) as usize;
        self.spend_point(options[option_index])
    }

    pub fn spend_weighted_point(&mut self, rng: &mut Rng, weights: &[u32]) -> bool {
        if self.points == 0 {
            return false;
        }

        let total_weight: u32 = self
            .levels
            .iter()
            .enumerate()
            .filter(|(_, level)| **level < UPGRADE_MAX_LEVEL)
            .map(|(index, _)| weights.get(index).copied().unwrap_or(1))
            .sum();
        if total_weight == 0 {
            return self.spend_random_point(rng);
        }

        let mut roll = rng.next(total_weight);
        for (index, level) in self.levels.iter().enumerate() {
            if *level >= UPGRADE_MAX_LEVEL {
                continue;
            }
            let weight = weights.get(index).copied().unwrap_or(1);
            if roll < weight {
                return self.spend_point(index);
            }
            roll -= weight;
        }

        false
    }

    pub fn health_regen_per_second(&self) -> f32 {
        self.levels[HEALTH_REGEN_INDEX] as f32 * 1.25
    }

    pub fn max_health(&self) -> f32 {
        constants::PLAYER_MAX_HEALTH + self.levels[MAX_HEALTH_INDEX] as f32 * 20.0
    }

    pub fn body_damage(&self) -> f32 {
        self.levels[BODY_DAMAGE_INDEX] as f32
    }

    pub fn bullet_speed(&self) -> f32 {
        constants::PROJECTILE_SPEED * (1.0 + self.levels[BULLET_SPEED_INDEX] as f32 * 0.08)
    }

    pub fn bullet_penetration(&self) -> u32 {
        1 + self.levels[BULLET_PENETRATION_INDEX]
    }

    pub fn bullet_damage(&self) -> f32 {
        constants::BASE_PROJECTILE_DAMAGE + self.levels[BULLET_DAMAGE_INDEX] as f32
    }

    pub fn reload_cooldown(&self) -> f32 {
        constants::SHOOT_COOLDOWN / (1.0 + self.levels[RELOAD_INDEX] as f32 * 0.12)
    }

    pub fn movement_speed(&self) -> f32 {
        constants::PLAYER_SPEED * (1.0 + self.levels[MOVEMENT_SPEED_INDEX] as f32 * 0.06)
    }
}

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

#[derive(Component)]
pub struct UpgradeMenuRoot;

#[derive(Component)]
pub struct UpgradePointText;

#[derive(Component)]
pub struct UpgradeLevelText(pub usize);

#[derive(Component)]
pub struct UpgradeButton(pub usize);

#[derive(Component)]
pub struct UpgradeFill(pub usize);

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

    spawn_upgrade_menu(&mut commands);
}

fn spawn_upgrade_menu(commands: &mut Commands) {
    commands
        .spawn((
            UpgradeMenuRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(18.0),
                bottom: Val::Px(52.0),
                width: Val::Px(UPGRADE_MENU_WIDTH),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                UpgradePointText,
                Text::new("x0"),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                TextShadow {
                    offset: Vec2::new(2.0, 2.0),
                    color: Color::srgba(0.05, 0.05, 0.06, 1.0),
                },
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(-2.0),
                    top: Val::Px(-30.0),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::radians(0.45)),
            ));

            for index in 0..UPGRADE_COUNT {
                root.spawn((
                    Button,
                    UpgradeButton(index),
                    upgrade_row_node(),
                    BackgroundColor(Color::srgba(0.23, 0.24, 0.25, 1.0)),
                ))
                .with_children(|row| {
                    let color = UPGRADE_COLORS[index];
                    row.spawn((
                        UpgradeFill(index),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            height: Val::Percent(100.0),
                            width: Val::Percent(0.0),
                            border_radius: BorderRadius::all(Val::Px(UPGRADE_ROW_RADIUS)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(color[0], color[1], color[2], color[3])),
                    ));

                    row.spawn((
                        Text::new(UPGRADE_LABELS[index]),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TextShadow {
                            offset: Vec2::new(1.0, 1.0),
                            color: Color::BLACK,
                        },
                        Node {
                            margin: UiRect::left(Val::Px(12.0)),
                            flex_grow: 1.0,
                            min_width: Val::Px(142.0),
                            ..default()
                        },
                    ));

                    row.spawn((
                        UpgradeLevelText(index),
                        Text::new(format!("[{}]", index + 1)),
                        TextFont {
                            font_size: FontSize::Px(10.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TextShadow {
                            offset: Vec2::new(1.0, 1.0),
                            color: Color::BLACK,
                        },
                        Node {
                            width: Val::Px(24.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                    ));

                    row.spawn((
                        Node {
                            width: Val::Px(37.0),
                            height: Val::Px(UPGRADE_ROW_HEIGHT),
                            border_radius: BorderRadius::all(Val::Px(UPGRADE_ROW_RADIUS)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(color[0], color[1], color[2], color[3])),
                    ))
                    .with_children(|plus| {
                        plus.spawn((
                            Text::new("+"),
                            TextFont {
                                font_size: FontSize::Px(22.0),
                                ..default()
                            },
                            TextColor(Color::srgba(0.12, 0.12, 0.13, 1.0)),
                        ));
                    });
                });
            }
        });
}

fn upgrade_row_node() -> Node {
    Node {
        width: Val::Px(UPGRADE_ROW_WIDTH),
        height: Val::Px(UPGRADE_ROW_HEIGHT),
        border_radius: BorderRadius::all(Val::Px(UPGRADE_ROW_RADIUS)),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        overflow: Overflow::clip(),
        ..default()
    }
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

    let required_xp = constants::xp_required_for_level(level.0);
    let xp_percent = (xp.0 as f32 / required_xp as f32 * 100.0).clamp(0.0, 100.0);

    for (mut text, score_marker, level_marker, xp_marker) in text_query.iter_mut() {
        if score_marker.is_some() {
            **text = format!("Score: {}", total_xp.0);
        } else if level_marker.is_some() {
            **text = format!("Lvl {}", level.0);
        } else if xp_marker.is_some() {
            **text = format!("XP {} / {}", xp.0, required_xp);
        }
    }
    for mut node in xp_fill.iter_mut() {
        node.width = Val::Percent(xp_percent);
    }
}

pub fn update_upgrade_menu(
    phase: Res<GamePhase>,
    upgrades: Res<UpgradeState>,
    mut root: Query<&mut Visibility, With<UpgradeMenuRoot>>,
    mut texts: Query<(
        &mut Text,
        Option<&UpgradePointText>,
        Option<&UpgradeLevelText>,
    )>,
) {
    if !(phase.is_changed() || upgrades.is_changed()) {
        return;
    }

    let visible = *phase == GamePhase::Playing && upgrades.points > 0;
    for mut visibility in root.iter_mut() {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    for (mut text, point_text, level_text) in texts.iter_mut() {
        if point_text.is_some() {
            **text = format!("x{}", upgrades.points);
        } else if let Some(level_text) = level_text {
            **text = format!("[{}]", level_text.0 + 1);
        }
    }
}

pub fn animate_upgrade_fills(
    time: Res<Time>,
    upgrades: Res<UpgradeState>,
    mut fills: Query<(&mut Node, &UpgradeFill)>,
) {
    let max_delta = UPGRADE_FILL_SPEED * time.delta_secs();

    for (mut node, fill) in fills.iter_mut() {
        let target =
            (upgrades.levels[fill.0] as f32 / UPGRADE_MAX_LEVEL as f32 * 100.0).clamp(0.0, 100.0);
        let current = match node.width {
            Val::Percent(value) => value,
            _ => 0.0,
        };
        let next = approach_percent(current, target, max_delta);
        if (next - current).abs() > f32::EPSILON {
            node.width = Val::Percent(next);
        }
    }
}

fn approach_percent(current: f32, target: f32, max_delta: f32) -> f32 {
    let delta = target - current;
    if delta.abs() <= max_delta {
        target
    } else {
        current + delta.signum() * max_delta
    }
}

pub fn handle_upgrade_buttons(
    mut upgrades: ResMut<UpgradeState>,
    mut buttons: Query<
        (&Interaction, &UpgradeButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, upgrade, mut color) in buttons.iter_mut() {
        let base = Color::srgba(0.23, 0.24, 0.25, 1.0);
        match *interaction {
            Interaction::Pressed => {
                upgrades.spend_point(upgrade.0);
                *color = BackgroundColor(Color::srgba(0.16, 0.17, 0.18, 1.0));
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgba(0.31, 0.32, 0.33, 1.0));
            }
            Interaction::None => {
                *color = BackgroundColor(base);
            }
        }
    }
}

pub fn handle_upgrade_hotkeys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut upgrades: ResMut<UpgradeState>,
) {
    if upgrades.points == 0 {
        return;
    }

    for (index, key) in UPGRADE_KEYS.iter().enumerate() {
        if keyboard.just_pressed(*key) {
            upgrades.spend_point(index);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrades_stop_spending_at_max_level() {
        let mut upgrades = UpgradeState {
            points: UPGRADE_MAX_LEVEL + 2,
            ..default()
        };

        for _ in 0..UPGRADE_MAX_LEVEL {
            assert!(upgrades.spend_point(0));
        }

        assert_eq!(upgrades.levels[0], UPGRADE_MAX_LEVEL);
        assert!(!upgrades.spend_point(0));
        assert_eq!(upgrades.points, 2);
    }

    #[test]
    fn upgrade_fill_approaches_target_percent() {
        assert_eq!(approach_percent(0.0, 12.5, 5.0), 5.0);
        assert_eq!(approach_percent(10.0, 12.5, 5.0), 12.5);
        assert_eq!(approach_percent(20.0, 12.5, 5.0), 15.0);
    }

    #[test]
    fn upgrade_effects_scale_from_selected_levels() {
        let mut upgrades = UpgradeState::default();
        upgrades.levels[HEALTH_REGEN_INDEX] = 2;
        upgrades.levels[MAX_HEALTH_INDEX] = 3;
        upgrades.levels[BODY_DAMAGE_INDEX] = 4;
        upgrades.levels[BULLET_SPEED_INDEX] = 5;
        upgrades.levels[BULLET_PENETRATION_INDEX] = 6;
        upgrades.levels[BULLET_DAMAGE_INDEX] = 7;
        upgrades.levels[RELOAD_INDEX] = 8;
        upgrades.levels[MOVEMENT_SPEED_INDEX] = 2;

        assert_eq!(upgrades.health_regen_per_second(), 2.5);
        assert_eq!(upgrades.max_health(), constants::PLAYER_MAX_HEALTH + 60.0);
        assert_eq!(upgrades.body_damage(), 4.0);
        assert!(upgrades.bullet_speed() > constants::PROJECTILE_SPEED);
        assert_eq!(upgrades.bullet_penetration(), 7);
        assert_eq!(upgrades.bullet_damage(), 10.0);
        assert!(upgrades.reload_cooldown() < constants::SHOOT_COOLDOWN);
        assert!(upgrades.movement_speed() > constants::PLAYER_SPEED);
    }

    #[test]
    fn capped_build_does_not_bank_more_points() {
        let mut upgrades = UpgradeState {
            points: 0,
            levels: [UPGRADE_MAX_LEVEL; UPGRADE_COUNT],
        };
        upgrades.add_points(20);
        assert!(upgrades.is_capped());
        assert_eq!(upgrades.points, 0);
    }
}
