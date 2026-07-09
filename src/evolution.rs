use crate::{menu::GamePhase, shape::Level};
use bevy::prelude::*;

const EVOLUTION_CAPS: [u32; 1] = [5];
const LEVEL_5: u32 = 5;
const MAX_OPTIONS: usize = 8;
const CARD_WIDTH: f32 = 178.0;
const CARD_HEIGHT: f32 = 154.0;
const CARD_UI_WIDTH: f32 = 10.5;
const CARD_UI_HEIGHT: f32 = 9.1;
const CARD_UI_GAP: f32 = 1.1;
const CARD_CENTER_X: f32 = CARD_WIDTH / 2.0;
const CARD_CENTER_Y: f32 = 68.0;
const BODY_DIAMETER: f32 = 52.0;

#[derive(Clone, Copy)]
pub struct EvolutionOption {
    pub name: &'static str,
    pub kind: EvolutionKind,
    pub description: &'static str,
    pub background: [f32; 4],
    pub lower_background: [f32; 4],
    pub visual: TankVisual,
}

#[derive(Clone, Copy)]
pub struct TankVisual {
    pub body_scale: f32,
    pub parts: &'static [TankVisualPart],
}

#[derive(Clone, Copy)]
pub enum TankVisualShape {
    Circle { diameter: f32 },
    Rectangle { width: f32, height: f32 },
    Polygon { diameter: f32, sides: usize },
}

#[derive(Clone, Copy)]
pub struct TankVisualPart {
    pub shape: TankVisualShape,
    pub offset: Vec2,
    pub rotation: f32,
    pub color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvolutionKind {
    Tank,
    Gunner,
    Cannon,
    TwinBarrel,
    Sniper,
    RamCore,
    Sprayer,
    Guard,
    Flanker,
}

#[derive(Clone, Copy)]
pub struct BarrelSpec {
    pub angle_offset: f32,
    pub lateral_offset: f32,
    pub width: f32,
    pub length: f32,
    pub damage_multiplier: f32,
}

#[derive(Clone)]
pub struct ChosenEvolution {
    pub level: u32,
}

#[derive(Resource, Clone)]
pub struct EvolutionState {
    pub current_name: String,
    pub current_kind: EvolutionKind,
    pub pending_levels: Vec<u32>,
    pub chosen: Vec<ChosenEvolution>,
}

impl Default for EvolutionState {
    fn default() -> Self {
        Self {
            current_name: "Tank".to_string(),
            current_kind: EvolutionKind::Tank,
            pending_levels: Vec::new(),
            chosen: Vec::new(),
        }
    }
}

impl EvolutionState {
    pub fn reset(&mut self) {
        self.current_name = "Tank".to_string();
        self.current_kind = EvolutionKind::Tank;
        self.pending_levels.clear();
        self.chosen.clear();
    }

    pub fn active_level(&self) -> Option<u32> {
        self.pending_levels.first().copied()
    }

    fn queue_reached_levels(&mut self, level: u32) {
        for cap in EVOLUTION_CAPS {
            if level >= cap && !self.has_handled_cap(cap) {
                self.pending_levels.push(cap);
            }
        }
        self.pending_levels.sort_unstable();
        self.pending_levels.dedup();
    }

    fn has_handled_cap(&self, cap: u32) -> bool {
        self.pending_levels.contains(&cap) || self.chosen.iter().any(|choice| choice.level == cap)
    }

    fn choose_active(&mut self, slot: usize) -> Option<EvolutionOption> {
        let level = self.active_level()?;
        let option = *options_for_level(level)?.get(slot)?;
        self.pending_levels.retain(|pending| *pending != level);
        self.current_name = option.name.to_string();
        self.current_kind = option.kind;
        self.chosen.push(ChosenEvolution { level });
        Some(option)
    }

    pub fn barrel_specs(&self) -> &'static [BarrelSpec] {
        match self.current_kind {
            EvolutionKind::Tank => &TANK_BARRELS,
            EvolutionKind::Gunner => &GUNNER_BARRELS,
            EvolutionKind::Cannon => &CANNON_BARRELS,
            EvolutionKind::TwinBarrel => &TWIN_BARRELS,
            EvolutionKind::Sniper => &SNIPER_BARRELS,
            EvolutionKind::RamCore => &RAM_CORE_BARRELS,
            EvolutionKind::Sprayer => &SPRAYER_BARRELS,
            EvolutionKind::Guard => &GUARD_BARRELS,
            EvolutionKind::Flanker => &FLANKER_BARRELS,
        }
    }

    pub fn reload_multiplier(&self) -> f32 {
        match self.current_kind {
            EvolutionKind::Gunner => 0.72,
            EvolutionKind::Cannon => 1.65,
            EvolutionKind::TwinBarrel => 0.95,
            EvolutionKind::Sniper => 1.18,
            EvolutionKind::RamCore => 1.10,
            EvolutionKind::Sprayer => 0.48,
            EvolutionKind::Guard => 1.08,
            EvolutionKind::Flanker => 0.92,
            EvolutionKind::Tank => 1.0,
        }
    }

    pub fn bullet_damage_multiplier(&self) -> f32 {
        match self.current_kind {
            EvolutionKind::Gunner => 0.78,
            EvolutionKind::Cannon => 2.35,
            EvolutionKind::TwinBarrel => 0.82,
            EvolutionKind::Sniper => 1.22,
            EvolutionKind::RamCore => 0.55,
            EvolutionKind::Sprayer => 0.52,
            EvolutionKind::Guard => 0.85,
            EvolutionKind::Flanker => 0.90,
            EvolutionKind::Tank => 1.0,
        }
    }

    pub fn bullet_speed_multiplier(&self) -> f32 {
        match self.current_kind {
            EvolutionKind::Cannon => 0.86,
            EvolutionKind::Sniper => 1.48,
            EvolutionKind::Sprayer => 0.92,
            EvolutionKind::Flanker => 1.05,
            _ => 1.0,
        }
    }

    pub fn bullet_knockback_multiplier(&self) -> f32 {
        match self.current_kind {
            EvolutionKind::Cannon => 2.35,
            EvolutionKind::Sniper => 1.2,
            EvolutionKind::Sprayer => 0.75,
            EvolutionKind::RamCore => 0.8,
            _ => 1.0,
        }
    }

    pub fn projectile_lifetime_multiplier(&self) -> f32 {
        match self.current_kind {
            EvolutionKind::Sniper => 1.70,
            EvolutionKind::Cannon => 1.15,
            EvolutionKind::Sprayer => 0.85,
            _ => 1.0,
        }
    }

    pub fn spread_radians(&self) -> f32 {
        match self.current_kind {
            EvolutionKind::Gunner => 0.045,
            EvolutionKind::Sprayer => 0.24,
            EvolutionKind::Cannon => 0.025,
            EvolutionKind::Sniper => 0.006,
            _ => 0.0,
        }
    }

    pub fn movement_multiplier(&self) -> f32 {
        match self.current_kind {
            EvolutionKind::RamCore => 0.94,
            EvolutionKind::Guard => 0.84,
            EvolutionKind::Flanker => 1.22,
            EvolutionKind::Sniper => 0.96,
            _ => 1.0,
        }
    }

    pub fn max_health_bonus(&self) -> i32 {
        match self.current_kind {
            EvolutionKind::RamCore => 45,
            EvolutionKind::Guard => 55,
            EvolutionKind::Flanker => -20,
            EvolutionKind::Sniper => -10,
            _ => 0,
        }
    }

    pub fn health_regen_bonus(&self) -> f32 {
        match self.current_kind {
            EvolutionKind::Guard => 2.5,
            EvolutionKind::RamCore => 0.6,
            EvolutionKind::Flanker => -0.3,
            _ => 0.0,
        }
    }

    pub fn body_damage_bonus(&self) -> u32 {
        match self.current_kind {
            EvolutionKind::RamCore => 5,
            EvolutionKind::Guard => 2,
            _ => 0,
        }
    }
}

#[derive(Component)]
pub struct EvolutionMenuRoot;

#[derive(Component)]
pub struct EvolutionOptionButton {
    slot: usize,
}

#[derive(Component)]
pub struct EvolutionDescriptionText;

const BARREL_COLOR: [f32; 4] = [0.62, 0.62, 0.62, 1.0];
const BODY_COLOR: [f32; 4] = [0.08, 0.70, 0.84, 1.0];
const BODY_RING_COLOR: [f32; 4] = [0.05, 0.48, 0.60, 1.0];
const RAM_COLOR: [f32; 4] = [0.28, 0.28, 0.28, 1.0];

pub const MAX_BARRELS: usize = 3;

const TANK_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    angle_offset: 0.0,
    lateral_offset: 0.0,
    width: 6.6,
    length: 30.6,
    damage_multiplier: 1.0,
}];
const GUNNER_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    angle_offset: 0.0,
    lateral_offset: 0.0,
    width: 6.0,
    length: 31.0,
    damage_multiplier: 1.0,
}];
const CANNON_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    angle_offset: 0.0,
    lateral_offset: 0.0,
    width: 11.5,
    length: 30.0,
    damage_multiplier: 1.0,
}];
const TWIN_BARRELS: [BarrelSpec; 2] = [
    BarrelSpec {
        angle_offset: 0.0,
        lateral_offset: -6.0,
        width: 5.4,
        length: 31.0,
        damage_multiplier: 0.88,
    },
    BarrelSpec {
        angle_offset: 0.0,
        lateral_offset: 6.0,
        width: 5.4,
        length: 31.0,
        damage_multiplier: 0.88,
    },
];
const SNIPER_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    angle_offset: 0.0,
    lateral_offset: 0.0,
    width: 4.8,
    length: 45.0,
    damage_multiplier: 1.0,
}];
const RAM_CORE_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    angle_offset: 0.0,
    lateral_offset: 0.0,
    width: 5.4,
    length: 25.0,
    damage_multiplier: 1.0,
}];
const SPRAYER_BARRELS: [BarrelSpec; 3] = [
    BarrelSpec {
        angle_offset: -0.16,
        lateral_offset: -5.0,
        width: 4.6,
        length: 28.0,
        damage_multiplier: 0.86,
    },
    BarrelSpec {
        angle_offset: 0.0,
        lateral_offset: 0.0,
        width: 4.6,
        length: 29.0,
        damage_multiplier: 0.86,
    },
    BarrelSpec {
        angle_offset: 0.16,
        lateral_offset: 5.0,
        width: 4.6,
        length: 28.0,
        damage_multiplier: 0.86,
    },
];
const GUARD_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    angle_offset: 0.0,
    lateral_offset: 0.0,
    width: 6.2,
    length: 27.0,
    damage_multiplier: 1.0,
}];
const FLANKER_BARRELS: [BarrelSpec; 2] = [
    BarrelSpec {
        angle_offset: 0.0,
        lateral_offset: 0.0,
        width: 5.2,
        length: 30.0,
        damage_multiplier: 0.92,
    },
    BarrelSpec {
        angle_offset: std::f32::consts::PI,
        lateral_offset: 0.0,
        width: 5.0,
        length: 24.0,
        damage_multiplier: 0.72,
    },
];

const GUNNER_PARTS: [TankVisualPart; 1] = [TankVisualPart {
    shape: TankVisualShape::Rectangle {
        width: 14.0,
        height: 54.0,
    },
    offset: Vec2::new(-20.0, 12.0),
    rotation: 2.15,
    color: BARREL_COLOR,
}];

const CANNON_PARTS: [TankVisualPart; 1] = [TankVisualPart {
    shape: TankVisualShape::Rectangle {
        width: 20.0,
        height: 52.0,
    },
    offset: Vec2::new(-21.0, 12.0),
    rotation: 2.15,
    color: BARREL_COLOR,
}];

const TWIN_PARTS: [TankVisualPart; 2] = [
    TankVisualPart {
        shape: TankVisualShape::Rectangle {
            width: 12.0,
            height: 54.0,
        },
        offset: Vec2::new(-26.0, 6.0),
        rotation: 2.15,
        color: BARREL_COLOR,
    },
    TankVisualPart {
        shape: TankVisualShape::Rectangle {
            width: 12.0,
            height: 54.0,
        },
        offset: Vec2::new(-16.0, 17.0),
        rotation: 2.15,
        color: BARREL_COLOR,
    },
];

const SNIPER_PARTS: [TankVisualPart; 1] = [TankVisualPart {
    shape: TankVisualShape::Rectangle {
        width: 10.0,
        height: 66.0,
    },
    offset: Vec2::new(-27.0, 13.0),
    rotation: 2.15,
    color: BARREL_COLOR,
}];

const RAM_CORE_PARTS: [TankVisualPart; 1] = [TankVisualPart {
    shape: TankVisualShape::Polygon {
        diameter: 64.0,
        sides: 6,
    },
    offset: Vec2::ZERO,
    rotation: 0.55,
    color: RAM_COLOR,
}];

const SPRAYER_PARTS: [TankVisualPart; 3] = [
    TankVisualPart {
        shape: TankVisualShape::Rectangle {
            width: 9.0,
            height: 50.0,
        },
        offset: Vec2::new(-23.0, 5.0),
        rotation: 1.88,
        color: BARREL_COLOR,
    },
    TankVisualPart {
        shape: TankVisualShape::Rectangle {
            width: 9.0,
            height: 50.0,
        },
        offset: Vec2::new(-20.0, 13.0),
        rotation: 2.15,
        color: BARREL_COLOR,
    },
    TankVisualPart {
        shape: TankVisualShape::Rectangle {
            width: 9.0,
            height: 50.0,
        },
        offset: Vec2::new(-15.0, 21.0),
        rotation: 2.42,
        color: BARREL_COLOR,
    },
];

const GUARD_PARTS: [TankVisualPart; 2] = [
    TankVisualPart {
        shape: TankVisualShape::Circle { diameter: 64.0 },
        offset: Vec2::ZERO,
        rotation: 0.0,
        color: BODY_RING_COLOR,
    },
    TankVisualPart {
        shape: TankVisualShape::Rectangle {
            width: 13.0,
            height: 50.0,
        },
        offset: Vec2::new(-20.0, 12.0),
        rotation: 2.15,
        color: BARREL_COLOR,
    },
];

const FLANKER_PARTS: [TankVisualPart; 2] = [
    TankVisualPart {
        shape: TankVisualShape::Rectangle {
            width: 11.0,
            height: 52.0,
        },
        offset: Vec2::new(-21.0, 12.0),
        rotation: 2.15,
        color: BARREL_COLOR,
    },
    TankVisualPart {
        shape: TankVisualShape::Rectangle {
            width: 11.0,
            height: 48.0,
        },
        offset: Vec2::new(21.0, -12.0),
        rotation: 2.15,
        color: BARREL_COLOR,
    },
];

const LEVEL_5_OPTIONS: [EvolutionOption; MAX_OPTIONS] = [
    EvolutionOption {
        name: "Gunner",
        kind: EvolutionKind::Gunner,
        description: "High reload rate, low damage, light spread. Reliable beginner DPS.",
        background: [0.53, 0.92, 0.82, 1.0],
        lower_background: [0.37, 0.76, 0.72, 1.0],
        visual: TankVisual {
            body_scale: 1.0,
            parts: &GUNNER_PARTS,
        },
    },
    EvolutionOption {
        name: "Cannon",
        kind: EvolutionKind::Cannon,
        description: "Slow reload, heavy bullets, high knockback. Hits hard, punishes misses.",
        background: [1.0, 0.63, 0.61, 1.0],
        lower_background: [0.80, 0.39, 0.40, 1.0],
        visual: TankVisual {
            body_scale: 1.04,
            parts: &CANNON_PARTS,
        },
    },
    EvolutionOption {
        name: "Twin Barrel",
        kind: EvolutionKind::TwinBarrel,
        description: "Adds a second barrel with slightly reduced bullet damage. Steady pressure.",
        background: [0.56, 0.93, 0.95, 1.0],
        lower_background: [0.40, 0.77, 0.78, 1.0],
        visual: TankVisual {
            body_scale: 1.0,
            parts: &TWIN_PARTS,
        },
    },
    EvolutionOption {
        name: "Sniper",
        kind: EvolutionKind::Sniper,
        description: "Longer range and faster bullets. Strong at distance, weak up close.",
        background: [0.62, 0.95, 0.45, 1.0],
        lower_background: [0.45, 0.74, 0.34, 1.0],
        visual: TankVisual {
            body_scale: 0.92,
            parts: &SNIPER_PARTS,
        },
    },
    EvolutionOption {
        name: "Ram Core",
        kind: EvolutionKind::RamCore,
        description: "More body damage and max health, but lower bullet damage. Contact fighter.",
        background: [0.55, 0.68, 1.0, 1.0],
        lower_background: [0.40, 0.52, 0.82, 1.0],
        visual: TankVisual {
            body_scale: 1.0,
            parts: &RAM_CORE_PARTS,
        },
    },
    EvolutionOption {
        name: "Sprayer",
        kind: EvolutionKind::Sprayer,
        description: "Very high fire rate with high spread. Good into swarms, poor precision.",
        background: [1.0, 0.74, 0.34, 1.0],
        lower_background: [0.82, 0.57, 0.23, 1.0],
        visual: TankVisual {
            body_scale: 0.96,
            parts: &SPRAYER_PARTS,
        },
    },
    EvolutionOption {
        name: "Guard",
        kind: EvolutionKind::Guard,
        description: "More health and passive regeneration, but slower movement. Defensive starter.",
        background: [0.74, 0.50, 1.0, 1.0],
        lower_background: [0.55, 0.36, 0.78, 1.0],
        visual: TankVisual {
            body_scale: 0.88,
            parts: &GUARD_PARTS,
        },
    },
    EvolutionOption {
        name: "Flanker",
        kind: EvolutionKind::Flanker,
        description: "Faster movement and turn pressure, lower max health. Hit-and-run style.",
        background: [1.0, 0.91, 0.47, 1.0],
        lower_background: [0.78, 0.69, 0.34, 1.0],
        visual: TankVisual {
            body_scale: 0.95,
            parts: &FLANKER_PARTS,
        },
    },
];

pub fn setup_evolution_menu(mut commands: Commands) {
    commands
        .spawn((
            EvolutionMenuRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(2.4),
                top: Val::VMin(2.0),
                width: Val::VMin(CARD_UI_WIDTH * 2.0 + CARD_UI_GAP),
                flex_direction: FlexDirection::Column,
                row_gap: Val::VMin(1.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Upgrades"),
                TextFont {
                    font_size: FontSize::VMin(4.1),
                    ..default()
                },
                TextColor(Color::WHITE),
                TextShadow {
                    offset: Vec2::new(3.5, 4.0),
                    color: Color::BLACK,
                },
            ));

            for row_start in (0..MAX_OPTIONS).step_by(2) {
                root.spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::VMin(CARD_UI_HEIGHT),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::VMin(CARD_UI_GAP),
                    ..default()
                })
                .with_children(|row| {
                    for slot in row_start..(row_start + 2).min(MAX_OPTIONS) {
                        row.spawn((
                            Button,
                            EvolutionOptionButton { slot },
                            evolution_card_node(),
                            BackgroundColor(Color::srgba(0.33, 0.34, 0.35, 1.0)),
                            BorderColor::all(Color::srgba(0.33, 0.34, 0.35, 1.0)),
                        ))
                        .with_children(|card| {
                            spawn_card_background(card, LEVEL_5_OPTIONS[slot]);
                            spawn_tank_visual(card, LEVEL_5_OPTIONS[slot].visual);
                            spawn_card_label(card, LEVEL_5_OPTIONS[slot].name);
                        });
                    }
                });
            }

            root.spawn((
                EvolutionDescriptionText,
                Text::new("Hover an evolution to inspect it."),
                TextFont {
                    font_size: FontSize::VMin(1.45),
                    ..default()
                },
                TextColor(Color::srgba(0.92, 0.94, 0.96, 1.0)),
                TextLayout::new(Justify::Left, LineBreak::WordBoundary),
                Node {
                    width: Val::VMin(34.0),
                    min_height: Val::VMin(5.4),
                    padding: UiRect::all(Val::VMin(0.75)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    border: UiRect::all(Val::VMin(0.18)),
                    border_radius: BorderRadius::all(Val::VMin(0.35)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.02, 0.025, 0.96)),
                BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.95)),
                Visibility::Hidden,
            ));
        });
}

fn evolution_card_node() -> Node {
    Node {
        width: Val::VMin(CARD_UI_WIDTH),
        height: Val::VMin(CARD_UI_HEIGHT),
        border: UiRect::all(Val::VMin(0.38)),
        border_radius: BorderRadius::all(Val::VMin(0.34)),
        overflow: Overflow::clip(),
        ..default()
    }
}

fn spawn_card_background(card: &mut ChildSpawnerCommands, option: EvolutionOption) {
    card.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(
            option.background[0],
            option.background[1],
            option.background[2],
            option.background[3],
        )),
    ));
    card.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            bottom: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(45.0),
            ..default()
        },
        BackgroundColor(Color::srgba(
            option.lower_background[0],
            option.lower_background[1],
            option.lower_background[2],
            option.lower_background[3],
        )),
    ));
}

fn spawn_tank_visual(card: &mut ChildSpawnerCommands, visual: TankVisual) {
    for part in visual.parts {
        spawn_visual_part(card, *part, true);
    }

    let body_diameter = BODY_DIAMETER * visual.body_scale;
    spawn_visual_part(
        card,
        TankVisualPart {
            shape: TankVisualShape::Circle {
                diameter: body_diameter,
            },
            offset: Vec2::ZERO,
            rotation: 0.0,
            color: BODY_COLOR,
        },
        true,
    );
}

fn spawn_visual_part(card: &mut ChildSpawnerCommands, part: TankVisualPart, outlined: bool) {
    if outlined {
        let outline_part = match part.shape {
            TankVisualShape::Circle { diameter } => TankVisualPart {
                shape: TankVisualShape::Circle {
                    diameter: diameter + 6.0,
                },
                color: [0.0, 0.0, 0.0, 0.40],
                ..part
            },
            TankVisualShape::Rectangle { width, height } => TankVisualPart {
                shape: TankVisualShape::Rectangle {
                    width: width + 6.0,
                    height: height + 6.0,
                },
                color: [0.0, 0.0, 0.0, 0.40],
                ..part
            },
            TankVisualShape::Polygon { diameter, sides } => TankVisualPart {
                shape: TankVisualShape::Polygon {
                    diameter: diameter + 6.0,
                    sides,
                },
                color: [0.0, 0.0, 0.0, 0.40],
                ..part
            },
        };
        spawn_visual_part(card, outline_part, false);
    }

    match part.shape {
        TankVisualShape::Circle { diameter } => {
            card.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: design_x(CARD_CENTER_X + part.offset.x - diameter / 2.0),
                    top: design_y(CARD_CENTER_Y - part.offset.y - diameter / 2.0),
                    width: design_x(diameter),
                    height: design_y(diameter),
                    border_radius: BorderRadius::all(Val::Percent(50.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(
                    part.color[0],
                    part.color[1],
                    part.color[2],
                    part.color[3],
                )),
            ));
        }
        TankVisualShape::Rectangle { width, height } => {
            card.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: design_x(CARD_CENTER_X + part.offset.x - width / 2.0),
                    top: design_y(CARD_CENTER_Y - part.offset.y - height / 2.0),
                    width: design_x(width),
                    height: design_y(height),
                    border_radius: BorderRadius::all(Val::Percent(9.0)),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::radians(part.rotation)),
                BackgroundColor(Color::srgba(
                    part.color[0],
                    part.color[1],
                    part.color[2],
                    part.color[3],
                )),
            ));
        }
        TankVisualShape::Polygon { diameter, .. } => {
            card.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: design_x(CARD_CENTER_X + part.offset.x - diameter / 2.0),
                    top: design_y(CARD_CENTER_Y - part.offset.y - diameter / 2.0),
                    width: design_x(diameter),
                    height: design_y(diameter),
                    border_radius: BorderRadius::all(Val::Percent(13.0)),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::radians(part.rotation)),
                BackgroundColor(Color::srgba(
                    part.color[0],
                    part.color[1],
                    part.color[2],
                    part.color[3],
                )),
            ));
        }
    }
}

fn spawn_card_label(card: &mut ChildSpawnerCommands, label: &'static str) {
    card.spawn((
        Text::new(label),
        TextFont {
            font_size: FontSize::VMin(if label.len() > 10 { 1.12 } else { 1.55 }),
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::new(Justify::Center, LineBreak::NoWrap),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::VMin(0.22),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
}

fn design_x(value: f32) -> Val {
    Val::Percent(value / CARD_WIDTH * 100.0)
}

fn design_y(value: f32) -> Val {
    Val::Percent(value / CARD_HEIGHT * 100.0)
}

pub fn queue_evolution_choices(level: Res<Level>, mut state: ResMut<EvolutionState>) {
    if !level.is_changed() {
        return;
    }

    state.queue_reached_levels(level.0);
}

pub fn update_evolution_menu(
    phase: Res<GamePhase>,
    state: Res<EvolutionState>,
    mut root: Query<&mut Visibility, With<EvolutionMenuRoot>>,
) {
    if !(phase.is_changed() || state.is_changed()) {
        return;
    }

    let visible = *phase == GamePhase::Playing && state.active_level().is_some();
    for mut visibility in root.iter_mut() {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn handle_evolution_buttons(
    mut state: ResMut<EvolutionState>,
    mut buttons: Query<
        (&Interaction, &EvolutionOptionButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, button, mut color) in buttons.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                state.choose_active(button.slot);
                *color = BackgroundColor(Color::srgba(0.23, 0.24, 0.25, 1.0));
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgba(0.42, 0.43, 0.44, 1.0));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgba(0.33, 0.34, 0.35, 1.0));
            }
        }
    }
}

pub fn update_evolution_hover_description(
    state: Res<EvolutionState>,
    buttons: Query<(&Interaction, &EvolutionOptionButton), With<Button>>,
    mut descriptions: Query<(&mut Text, &mut Visibility), With<EvolutionDescriptionText>>,
) {
    if state.active_level().is_none() {
        for (_, mut visibility) in descriptions.iter_mut() {
            *visibility = Visibility::Hidden;
        }
        return;
    }

    let hovered = buttons
        .iter()
        .find(|(interaction, _)| **interaction == Interaction::Hovered)
        .and_then(|(_, button)| LEVEL_5_OPTIONS.get(button.slot));

    for (mut text, mut visibility) in descriptions.iter_mut() {
        if let Some(option) = hovered {
            **text = format!("{}\n{}", option.name, option.description);
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn options_for_level(level: u32) -> Option<&'static [EvolutionOption]> {
    match level {
        LEVEL_5 => Some(&LEVEL_5_OPTIONS),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_level_five_queues_evolution() {
        let mut state = EvolutionState::default();

        state.queue_reached_levels(100);
        state.queue_reached_levels(100);

        assert_eq!(state.pending_levels, vec![5]);
    }

    #[test]
    fn choosing_active_evolution_updates_current_name() {
        let mut state = EvolutionState::default();
        state.queue_reached_levels(5);

        let choice = state.choose_active(2).unwrap();

        assert_eq!(choice.name, "Twin Barrel");
        assert_eq!(state.current_name, "Twin Barrel");
        assert_eq!(state.current_kind, EvolutionKind::TwinBarrel);
        assert!(state.pending_levels.is_empty());
        assert_eq!(state.chosen.len(), 1);
    }

    #[test]
    fn only_level_five_has_options_for_now() {
        assert_eq!(options_for_level(5).unwrap().len(), 8);
        assert!(options_for_level(15).is_none());
        assert!(options_for_level(100).is_none());
    }

    #[test]
    fn level_five_evolutions_have_supported_barrel_counts() {
        for option in LEVEL_5_OPTIONS {
            let mut state = EvolutionState::default();
            state.current_kind = option.kind;

            assert!(!state.barrel_specs().is_empty());
            assert!(state.barrel_specs().len() <= MAX_BARRELS);
        }
    }
}
