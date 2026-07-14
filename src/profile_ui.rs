use crate::{
    menu::MenuRoot,
    palette::PaletteId,
    profile::{AchievementId, Profile},
};
use bevy::prelude::*;

#[derive(Component)]
pub struct PreviousPalette;

#[derive(Component)]
pub struct NextPalette;

#[derive(Component)]
pub struct PaletteNameText;

#[derive(Component)]
pub struct ProfileRecordsText;

pub fn setup_profile_panel(mut commands: Commands) {
    commands
        .spawn((
            MenuRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                bottom: Val::Px(18.0),
                width: Val::Px(610.0),
                height: Val::Px(86.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(7.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                ..default()
            },
            UiTransform::from_translation(Val2::px(-305.0, 0.0)),
            BackgroundColor(Color::srgba(0.035, 0.037, 0.055, 0.82)),
            BorderColor::all(Color::srgba(0.25, 0.27, 0.36, 0.9)),
            Visibility::Visible,
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(30.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|palettes| {
                    palettes.spawn(profile_button("<", PreviousPalette));
                    palettes.spawn((
                        PaletteNameText,
                        Text::new("Palette: Core Blue"),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TextLayout::new(Justify::Center, LineBreak::NoWrap),
                        Node {
                            width: Val::Px(460.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                    ));
                    palettes.spawn(profile_button(">", NextPalette));
                });
            panel.spawn((
                ProfileRecordsText,
                Text::new("BEST 0  |  CROWN 0:00  |  K/D 0/0"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgba(0.78, 0.81, 0.88, 1.0)),
                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(22.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
            ));
        });
}

fn profile_button<T: Component>(
    label: &'static str,
    marker: T,
) -> (Button, T, Node, BackgroundColor, BorderColor, Text) {
    (
        Button,
        marker,
        Node {
            width: Val::Px(28.0),
            height: Val::Px(28.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.12, 0.14, 0.20, 0.95)),
        BorderColor::all(Color::srgba(0.35, 0.38, 0.48, 1.0)),
        Text::new(label),
    )
}

pub fn handle_palette_buttons(
    mut profile: ResMut<Profile>,
    buttons: Query<
        (&Interaction, Option<&PreviousPalette>, Option<&NextPalette>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, previous, next) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let unlocked = PaletteId::ALL
            .into_iter()
            .filter(|palette| profile.data.unlocked_palettes.contains(palette))
            .collect::<Vec<_>>();
        let current = unlocked
            .iter()
            .position(|palette| *palette == profile.data.selected_palette)
            .unwrap_or(0);
        let next_index = if previous.is_some() {
            (current + unlocked.len() - 1) % unlocked.len()
        } else if next.is_some() {
            (current + 1) % unlocked.len()
        } else {
            continue;
        };
        profile.select_palette(unlocked[next_index]);
    }
}

pub fn update_profile_panel(
    profile: Res<Profile>,
    mut palette_text: Query<&mut Text, With<PaletteNameText>>,
    mut records_text: Query<&mut Text, (With<ProfileRecordsText>, Without<PaletteNameText>)>,
) {
    if !profile.is_changed() {
        return;
    }
    if let Ok(mut text) = palette_text.single_mut() {
        **text = format!(
            "Palette: {}  ({}/{})",
            profile.data.selected_palette.name(),
            profile.data.unlocked_palettes.len(),
            PaletteId::ALL.len()
        );
    }
    if let Ok(mut text) = records_text.single_mut() {
        let records = &profile.data.records;
        **text = format!(
            "BEST {}  |  CROWN {}:{:02}  |  K/D {}/{}  |  ACH {}/{}",
            records.best_life_score,
            records.best_crown_streak_secs as u32 / 60,
            records.best_crown_streak_secs as u32 % 60,
            records.lifetime_kills,
            records.lifetime_deaths,
            profile.data.achievements.len(),
            AchievementId::ALL.len(),
        );
    }
}
