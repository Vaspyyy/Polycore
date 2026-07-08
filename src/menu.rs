use bevy::{input::{keyboard::KeyboardInput, ButtonState}, prelude::*};
use crate::{hud, player};

const MODE_HIGHLIGHT_WIDTH: f32 = 154.0;
const MODE_HIGHLIGHT_SINGLE_X: f32 = 10.0;
const MODE_HIGHLIGHT_MULTI_X: f32 = 172.0;

#[derive(Resource, PartialEq, Eq, Clone, Copy)]
pub enum GamePhase {
    Menu,
    Playing,
}

#[derive(Resource, PartialEq, Eq, Clone, Copy)]
pub enum GameMode {
    Singleplayer,
    Multiplayer,
}

#[derive(Resource)]
pub struct PlayerName(pub String);

#[derive(Resource, Default)]
pub struct NameInputFocus(pub bool);

#[derive(Component)]
pub struct MenuRoot;

#[derive(Component)]
pub struct MenuDecoration;

#[derive(Component)]
pub struct NameField;

#[derive(Component)]
pub struct MenuNameText;

#[derive(Component)]
pub struct PlayButton;

#[derive(Component)]
pub struct ModeButton(pub GameMode);

#[derive(Component)]
pub struct ModeHighlight;

pub fn is_playing(phase: Res<GamePhase>) -> bool {
    *phase == GamePhase::Playing
}

pub fn setup_menu(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    spawn_menu_decorations(&mut commands, &mut meshes, &mut materials);

    commands
        .spawn((
            MenuRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(20.0),
                padding: UiRect::all(Val::Px(44.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Visibility::Visible,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.24)),
            ));

            root.spawn((
                Text::new("POLYCORE"),
                TextFont {
                    font_size: FontSize::Px(78.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                TextShadow {
                    offset: Vec2::new(5.0, 8.0),
                    color: Color::srgba(0.03, 0.03, 0.04, 0.85),
                },
            ));

            root.spawn(Node {
                width: Val::Px(610.0),
                height: Val::Px(86.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn(menu_select_column("Game Mode", true))
                    .with_children(|column| {
                        column.spawn(menu_label("Game Mode"));
                        column.spawn(mode_select_box()).with_children(|mode| {
                            mode.spawn((
                                ModeHighlight,
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(MODE_HIGHLIGHT_SINGLE_X),
                                    width: Val::Px(MODE_HIGHLIGHT_WIDTH),
                                    height: Val::Px(42.0),
                                    border_radius: BorderRadius::all(Val::Px(4.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.68, 0.45, 0.93, 1.0)),
                            ));
                            mode.spawn(mode_option("Singleplayer", GameMode::Singleplayer))
                                .with_children(|option| {
                                    option.spawn(menu_option_text("Singleplayer"));
                                });
                            mode.spawn(mode_option("Multiplayer", GameMode::Multiplayer))
                                .with_children(|option| {
                                    option.spawn(menu_option_text("Multiplayer"));
                                });
                        });
                    });
                row.spawn(menu_select_column("Region", false))
                    .with_children(|column| {
                        column.spawn(menu_label("Region"));
                        column.spawn(region_select_box()).with_children(|region| {
                            region.spawn((
                                Text::new("Region Placeholder"),
                                TextFont {
                                    font_size: FontSize::Px(21.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                TextShadow {
                                    offset: Vec2::new(2.0, 2.0),
                                    color: Color::BLACK,
                                },
                            ));
                        });
                    });
            });

            root.spawn((
                Button,
                NameField,
                Node {
                    width: Val::Px(610.0),
                    height: Val::Px(80.0),
                    border: UiRect::all(Val::Px(3.0)),
                    border_radius: BorderRadius::all(Val::Px(5.0)),
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(18.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.96)),
                BorderColor::all(Color::srgba(0.74, 0.74, 0.74, 1.0)),
            ))
            .with_children(|name_box| {
                name_box.spawn((
                    MenuNameText,
                    Text::new("<PLAYER NAME>"),
                    TextFont {
                        font_size: FontSize::Px(25.0),
                        ..default()
                    },
                    TextColor(Color::BLACK),
                ));
            });

            root.spawn((
                Button,
                PlayButton,
                Node {
                    width: Val::Px(304.0),
                    height: Val::Px(48.0),
                    border: UiRect::all(Val::Px(3.0)),
                    border_radius: BorderRadius::all(Val::Px(5.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.13, 0.66, 0.82, 1.0)),
                BorderColor::all(Color::srgba(0.10, 0.54, 0.67, 1.0)),
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("Play!"),
                    TextFont {
                        font_size: FontSize::Px(21.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    TextShadow {
                        offset: Vec2::new(2.0, 2.0),
                        color: Color::BLACK,
                    },
                ));
            });
        });
}

fn spawn_menu_decorations(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    let yellow = materials.add(Color::srgba(0.82, 0.72, 0.18, 0.85));
    let red = materials.add(Color::srgba(0.85, 0.18, 0.18, 0.75));
    let positions = [
        (Vec3::new(-390.0, 235.0, -1.0), 4, yellow.clone(), 0.35),
        (Vec3::new(420.0, 190.0, -1.0), 4, yellow.clone(), -0.2),
        (Vec3::new(-265.0, -170.0, -1.0), 3, red.clone(), 0.15),
        (Vec3::new(335.0, -245.0, -1.0), 5, yellow.clone(), 0.55),
        (Vec3::new(15.0, 250.0, -1.0), 6, red, 0.0),
    ];

    for (position, sides, material, rotation) in positions {
        commands.spawn((
            MenuDecoration,
            Mesh2d(meshes.add(RegularPolygon::new(24.0, sides))),
            MeshMaterial2d(material),
            Transform {
                translation: position,
                rotation: Quat::from_rotation_z(rotation),
                ..default()
            },
        ));
    }
}

fn menu_select_column(label: &'static str, wide: bool) -> (Node, Name) {
    (
        Node {
            width: Val::Px(if wide { 340.0 } else { 254.0 }),
            height: Val::Px(86.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
        Name::new(label),
    )
}

fn menu_label(label: &'static str) -> (Text, TextFont, TextColor, TextShadow) {
    (
        Text::new(label),
        TextFont {
            font_size: FontSize::Px(17.0),
            ..default()
        },
        TextColor(Color::WHITE),
        TextShadow {
            offset: Vec2::new(2.0, 2.0),
            color: Color::BLACK,
        },
    )
}

fn mode_select_box() -> (Node, BackgroundColor, BorderColor) {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(64.0),
            border: UiRect::all(Val::Px(3.0)),
            border_radius: BorderRadius::all(Val::Px(5.0)),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.47, 0.28, 0.75, 0.86)),
        BorderColor::all(Color::srgba(0.34, 0.20, 0.58, 1.0)),
    )
}

fn mode_option(label: &'static str, mode: GameMode) -> (Button, ModeButton, Node, BackgroundColor, Name) {
    (
        Button,
        ModeButton(mode),
        Node {
            width: Val::Percent(50.0),
            height: Val::Px(42.0),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::NONE),
        Name::new(label),
    )
}

fn menu_option_text(label: &'static str) -> (Text, TextFont, TextColor, TextShadow) {
    (
        Text::new(label),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::WHITE),
        TextShadow {
            offset: Vec2::new(2.0, 2.0),
            color: Color::BLACK,
        },
    )
}

fn region_select_box() -> (Node, BackgroundColor, BorderColor) {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(64.0),
            border: UiRect::all(Val::Px(3.0)),
            border_radius: BorderRadius::all(Val::Px(5.0)),
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(14.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.32, 0.32, 0.31, 0.88)),
        BorderColor::all(Color::srgba(0.22, 0.22, 0.21, 1.0)),
    )
}

pub fn handle_play_button(
    mut phase: ResMut<GamePhase>,
    mut focus: ResMut<NameInputFocus>,
    mut play_buttons: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<PlayButton>)>,
    mut visibility_query: Query<(
        &mut Visibility,
        Option<&MenuRoot>,
        Option<&hud::HudRoot>,
        Option<&MenuDecoration>,
        Option<&player::Player>,
        Option<&player::Barrel>,
    )>,
    mut name_fields: Query<&mut BorderColor, With<NameField>>,
) {
    for (interaction, mut color) in play_buttons.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *phase = GamePhase::Playing;
                focus.0 = false;
                *color = BackgroundColor(Color::srgba(0.10, 0.54, 0.67, 1.0));
                for mut border in name_fields.iter_mut() {
                    *border = BorderColor::all(Color::srgba(0.74, 0.74, 0.74, 1.0));
                }
                for (mut visibility, menu_root, hud_root, decoration, player, barrel) in visibility_query.iter_mut() {
                    if menu_root.is_some() || decoration.is_some() {
                        *visibility = Visibility::Hidden;
                    } else if hud_root.is_some() || player.is_some() || barrel.is_some() {
                        *visibility = Visibility::Visible;
                    }
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgba(0.18, 0.74, 0.90, 1.0));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgba(0.13, 0.66, 0.82, 1.0));
            }
        }
    }
}

pub fn handle_mode_buttons(
    mut mode: ResMut<GameMode>,
    buttons: Query<(&Interaction, &ModeButton), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, button) in buttons.iter() {
        if *interaction == Interaction::Pressed {
            *mode = button.0;
        }
    }
}

pub fn update_mode_highlight(
    time: Res<Time>,
    mode: Res<GameMode>,
    mut highlights: Query<&mut Node, With<ModeHighlight>>,
) {
    let target = match *mode {
        GameMode::Singleplayer => MODE_HIGHLIGHT_SINGLE_X,
        GameMode::Multiplayer => MODE_HIGHLIGHT_MULTI_X,
    };
    let speed = 720.0 * time.delta_secs();

    for mut node in highlights.iter_mut() {
        let Val::Px(current) = node.left else {
            node.left = Val::Px(target);
            continue;
        };
        let delta = target - current;
        node.left = if delta.abs() <= speed {
            Val::Px(target)
        } else {
            Val::Px(current + delta.signum() * speed)
        };
    }
}

pub fn handle_name_field_clicks(
    mut focus: ResMut<NameInputFocus>,
    mut fields: Query<(&Interaction, &mut BorderColor), (Changed<Interaction>, With<NameField>)>,
) {
    for (interaction, mut border) in fields.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                focus.0 = true;
                *border = BorderColor::all(Color::srgba(0.20, 0.64, 0.95, 1.0));
            }
            Interaction::Hovered if !focus.0 => {
                *border = BorderColor::all(Color::srgba(0.60, 0.60, 0.62, 1.0));
            }
            Interaction::None if !focus.0 => {
                *border = BorderColor::all(Color::srgba(0.74, 0.74, 0.74, 1.0));
            }
            _ => {}
        }
    }
}

pub fn handle_name_keyboard(
    mut keyboard: MessageReader<KeyboardInput>,
    phase: Res<GamePhase>,
    mut focus: ResMut<NameInputFocus>,
    mut name: ResMut<PlayerName>,
) {
    if *phase != GamePhase::Menu || !focus.0 {
        keyboard.clear();
        return;
    }

    for event in keyboard.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        match event.key_code {
            KeyCode::Enter | KeyCode::Escape => {
                focus.0 = false;
            }
            KeyCode::Backspace => {
                name.0.pop();
            }
            _ => {
                if let Some(text) = &event.text {
                    for character in text.chars() {
                        if !character.is_control() && name.0.chars().count() < 18 {
                            name.0.push(character);
                        }
                    }
                }
            }
        }
    }
}

pub fn sync_player_name_text(
    name: Res<PlayerName>,
    focus: Res<NameInputFocus>,
    mut texts: Query<(&mut Text, Option<&MenuNameText>, Option<&hud::PlayerNameText>)>,
) {
    if !(name.is_changed() || focus.is_changed()) {
        return;
    }

    let display_name = if name.0.is_empty() {
        "<PLAYER NAME>".to_string()
    } else if focus.0 {
        format!("{}_", name.0)
    } else {
        name.0.clone()
    };

    for (mut text, menu_name, hud_name) in texts.iter_mut() {
        if menu_name.is_some() || hud_name.is_some() {
            **text = display_name.clone();
        }
    }
}
