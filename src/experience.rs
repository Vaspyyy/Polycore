use crate::{menu::GamePhase, profile::Profile};
use bevy::{prelude::*, window::MonitorSelection};

#[derive(Component)]
pub struct PauseRoot;

#[derive(Component, Clone, Copy)]
pub enum SettingsAction {
    Resume,
    ShakeDown,
    ShakeUp,
    DamageIndicators,
    Fullscreen,
}

#[derive(Component, Clone, Copy)]
pub enum SettingsValue {
    Shake,
    DamageIndicators,
    Fullscreen,
}

pub fn setup_experience_ui(mut commands: Commands) {
    commands
        .spawn((
            PauseRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.01, 0.015, 0.68)),
            GlobalZIndex(80),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(420.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(11.0),
                    padding: UiRect::all(Val::Px(22.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.035, 0.04, 0.065, 0.96)),
                BorderColor::all(Color::srgba(0.28, 0.31, 0.42, 1.0)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("PAUSED"),
                    TextFont {
                        font_size: FontSize::Px(27.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                spawn_setting_row(
                    panel,
                    "Screen shake",
                    SettingsValue::Shake,
                    SettingsAction::ShakeDown,
                    SettingsAction::ShakeUp,
                );
                panel.spawn(setting_toggle(
                    "Damage indicators",
                    SettingsValue::DamageIndicators,
                    SettingsAction::DamageIndicators,
                ));
                panel.spawn(setting_toggle(
                    "Fullscreen",
                    SettingsValue::Fullscreen,
                    SettingsAction::Fullscreen,
                ));
                panel.spawn(action_button("Resume", SettingsAction::Resume));
            });
        });
}

fn spawn_setting_row(
    panel: &mut ChildSpawnerCommands,
    label: &'static str,
    value: SettingsValue,
    down: SettingsAction,
    up: SettingsAction,
) {
    panel
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(34.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|row| {
            row.spawn(setting_label(label));
            row.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(7.0),
                ..default()
            })
            .with_children(|controls| {
                controls.spawn(small_button("-", down));
                controls.spawn((
                    value,
                    Text::new("0%"),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Node {
                        width: Val::Px(52.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                ));
                controls.spawn(small_button("+", up));
            });
        });
}

fn setting_toggle(
    label: &'static str,
    value: SettingsValue,
    action: SettingsAction,
) -> (
    Button,
    SettingsAction,
    SettingsValue,
    Node,
    BackgroundColor,
    Text,
) {
    (
        Button,
        action,
        value,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(34.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.10, 0.12, 0.18, 1.0)),
        Text::new(label),
    )
}

fn action_button(
    label: &'static str,
    action: SettingsAction,
) -> (Button, SettingsAction, Node, BackgroundColor, Text) {
    (
        Button,
        action,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(36.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.12, 0.50, 0.64, 1.0)),
        Text::new(label),
    )
}

fn small_button(
    label: &'static str,
    action: SettingsAction,
) -> (Button, SettingsAction, Node, BackgroundColor, Text) {
    (
        Button,
        action,
        Node {
            width: Val::Px(28.0),
            height: Val::Px(28.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.14, 0.17, 0.24, 1.0)),
        Text::new(label),
    )
}

fn setting_label(label: &'static str) -> (Text, TextFont, TextColor) {
    (
        Text::new(label),
        TextFont {
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(Color::srgba(0.82, 0.84, 0.90, 1.0)),
    )
}

pub fn handle_pause_input(keyboard: Res<ButtonInput<KeyCode>>, mut phase: ResMut<GamePhase>) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }
    *phase = match *phase {
        GamePhase::Playing => GamePhase::Paused,
        GamePhase::Paused => GamePhase::Playing,
        other => other,
    };
}

pub fn sync_pause_visibility(
    phase: Res<GamePhase>,
    mut roots: Query<&mut Visibility, With<PauseRoot>>,
) {
    if !phase.is_changed() {
        return;
    }
    for mut visibility in &mut roots {
        *visibility = if *phase == GamePhase::Paused {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn handle_settings_buttons(
    mut phase: ResMut<GamePhase>,
    mut profile: ResMut<Profile>,
    buttons: Query<(&Interaction, &SettingsAction), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            SettingsAction::Resume => *phase = GamePhase::Playing,
            SettingsAction::ShakeDown => profile.data.settings.screen_shake -= 0.1,
            SettingsAction::ShakeUp => profile.data.settings.screen_shake += 0.1,
            SettingsAction::DamageIndicators => {
                profile.data.settings.damage_indicators = !profile.data.settings.damage_indicators;
            }
            SettingsAction::Fullscreen => {
                profile.data.settings.fullscreen = !profile.data.settings.fullscreen;
            }
        }
        profile.data.settings.screen_shake = profile.data.settings.screen_shake.clamp(0.0, 1.0);
        profile.mark_dirty();
    }
}

pub fn update_settings_labels(
    profile: Res<Profile>,
    mut labels: Query<(&SettingsValue, &mut Text)>,
) {
    if !profile.is_changed() {
        return;
    }
    let settings = &profile.data.settings;
    for (value, mut text) in &mut labels {
        **text = match value {
            SettingsValue::Shake => format!("{}%", (settings.screen_shake * 100.0).round()),
            SettingsValue::DamageIndicators => format!(
                "Damage indicators: {}",
                if settings.damage_indicators {
                    "On"
                } else {
                    "Off"
                }
            ),
            SettingsValue::Fullscreen => format!(
                "Fullscreen: {}",
                if settings.fullscreen { "On" } else { "Off" }
            ),
        };
    }
}

pub fn apply_window_settings(profile: Res<Profile>, mut window: Single<&mut Window>) {
    if !profile.is_changed() {
        return;
    }
    window.mode = if profile.data.settings.fullscreen {
        bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        bevy::window::WindowMode::Windowed
    };
}
