use bevy::prelude::*;
use crate::shape::{Xp, Level};

#[derive(Component)]
pub struct HudText;

pub fn setup_hud(mut commands: Commands) {
    commands.spawn((
        Text::new("Level: 1 | XP: 0"),
        TextFont {
            font_size: FontSize::Px(24.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        HudText,
    ));
}

pub fn update_hud(
    xp: Res<Xp>,
    level: Res<Level>,
    mut query: Query<&mut Text, With<HudText>>,
) {
    if xp.is_changed() || level.is_changed() {
        for mut text in query.iter_mut() {
            **text = format!("Level: {} | XP: {}", level.0, xp.0);
        }
    }
}
