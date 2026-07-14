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

#[derive(Component)]
pub struct AchievementProgressText;

pub fn setup_profile_panel(mut commands: Commands) {
    commands
        .spawn((
            MenuRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                bottom: Val::Px(18.0),
                width: Val::Px(610.0),
                height: Val::Px(106.0),
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
            panel.spawn((
                AchievementProgressText,
                Text::new("NEXT ACHIEVEMENT"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgba(0.66, 0.72, 0.84, 1.0)),
                TextLayout::new(Justify::Center, LineBreak::NoWrap),
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
    mut achievement_text: Query<
        &mut Text,
        (
            With<AchievementProgressText>,
            Without<PaletteNameText>,
            Without<ProfileRecordsText>,
        ),
    >,
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
    if let Ok(mut text) = achievement_text.single_mut() {
        **text = next_achievement_progress(&profile);
    }
}

fn next_achievement_progress(profile: &Profile) -> String {
    let records = &profile.data.records;
    let progress = |achievement| match achievement {
        AchievementId::ShapeHunter => (records.shapes_destroyed, 10, "destroy shapes"),
        AchievementId::FirstKill => (records.lifetime_kills, 1, "defeat a tank"),
        AchievementId::FirstEvolution => (
            u32::from(!records.used_level_five_evolutions.is_empty()),
            1,
            "choose a level 5 evolution",
        ),
        AchievementId::ClaimCrown => (
            u32::from(records.total_crown_time_secs > 0.0),
            1,
            "claim the crown",
        ),
        AchievementId::CrownThirty => (
            records.best_crown_streak_secs as u32,
            30,
            "hold the crown (seconds)",
        ),
        AchievementId::Survivor => (records.longest_life_secs as u32, 300, "survive (seconds)"),
        AchievementId::ScoreThousand => (records.best_life_score, 1_000, "score in one life"),
        AchievementId::FiveKillLife => (records.best_life_kills, 5, "kills in one life"),
        AchievementId::AdvancedEvolution => (0, 1, "choose a level 15 evolution"),
        AchievementId::CrownOneTwenty => (
            records.best_crown_streak_secs as u32,
            120,
            "hold the crown (seconds)",
        ),
        AchievementId::EvolutionMastery => (
            records.used_level_five_evolutions.len() as u32,
            8,
            "use distinct level 5 evolutions",
        ),
        AchievementId::FinalForm => (
            u32::from(!records.used_level_thirty_evolutions.is_empty()),
            1,
            "reach a level 30 capstone",
        ),
        AchievementId::FullArsenal => (
            records.used_level_thirty_evolutions.len() as u32,
            16,
            "reach all level 30 capstones",
        ),
        AchievementId::RichVein => (
            records.hotspot_high_tier_shapes_destroyed,
            100,
            "destroy hotspot pentagons or hexagons",
        ),
        AchievementId::HoldTheZone => (
            records.hotspots_survived,
            1,
            "survive a hotspot from its opening",
        ),
    };

    let Some((achievement, (current, target, requirement))) = AchievementId::ALL
        .into_iter()
        .filter(|achievement| !profile.data.achievements.contains(achievement))
        .map(|achievement| (achievement, progress(achievement)))
        .max_by(|(_, a), (_, b)| (a.0 as f32 / a.1 as f32).total_cmp(&(b.0 as f32 / b.1 as f32)))
    else {
        return "ALL ACHIEVEMENTS UNLOCKED".to_string();
    };
    format!(
        "NEXT {}: {}/{} — {}",
        achievement.title(),
        current.min(target),
        target,
        requirement
    )
}
