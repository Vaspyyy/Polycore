use crate::{menu::GamePhase, shape::Level};
use bevy::prelude::*;

const EVOLUTION_CAPS: [u32; 2] = [5, 15];
const LEVEL_5: u32 = 5;
const LEVEL_15: u32 = 15;
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
    Minigun,
    Stabilizer,
    Annihilator,
    RailCannon,
    QuadBarrel,
    Crossfire,
    Marksman,
    Hunter,
    Juggernaut,
    Interceptor,
    PentaShot,
    Needler,
    Fortress,
    Bulwark,
    Booster,
    Fighter,
}

impl EvolutionKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Tank => "tank",
            Self::Gunner => "gunner",
            Self::Cannon => "cannon",
            Self::TwinBarrel => "twin_barrel",
            Self::Sniper => "sniper",
            Self::RamCore => "ram_core",
            Self::Sprayer => "sprayer",
            Self::Guard => "guard",
            Self::Flanker => "flanker",
            Self::Minigun => "minigun",
            Self::Stabilizer => "stabilizer",
            Self::Annihilator => "annihilator",
            Self::RailCannon => "rail_cannon",
            Self::QuadBarrel => "quad_barrel",
            Self::Crossfire => "crossfire",
            Self::Marksman => "marksman",
            Self::Hunter => "hunter",
            Self::Juggernaut => "juggernaut",
            Self::Interceptor => "interceptor",
            Self::PentaShot => "penta_shot",
            Self::Needler => "needler",
            Self::Fortress => "fortress",
            Self::Bulwark => "bulwark",
            Self::Booster => "booster",
            Self::Fighter => "fighter",
        }
    }

    pub fn is_level_five(self) -> bool {
        definition(self).level == LEVEL_5
    }

    pub fn is_advanced(self) -> bool {
        definition(self).level == LEVEL_15
    }

    pub fn base(self) -> Self {
        base_kind(self)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PassiveKind {
    #[default]
    None,
    MinigunSpin,
    Stabilized,
    Splash,
    DistanceDamage,
    AlternatingPairs,
    RearKnockback,
    HunterMark,
    MomentumArmor,
    FrontalArmor,
    PhasedFan,
    ConsecutiveHits,
    Entrenched,
    FrontalShield,
    BoosterRecoil,
    HitSpeed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvolutionTag {
    Sustained,
    Precision,
    Burst,
    AreaControl,
    Durability,
    Defense,
    Mobility,
}

#[derive(Clone, Copy, Debug)]
pub struct EvolutionModifiers {
    pub reload: f32,
    pub bullet_damage: f32,
    pub bullet_speed: f32,
    pub knockback: f32,
    pub lifetime: f32,
    pub spread: f32,
    pub movement: f32,
    pub max_health_bonus: f32,
    pub health_regen_bonus: f32,
    pub body_damage_bonus: f32,
}

impl EvolutionModifiers {
    const IDENTITY: Self = Self {
        reload: 1.0,
        bullet_damage: 1.0,
        bullet_speed: 1.0,
        knockback: 1.0,
        lifetime: 1.0,
        spread: 1.0,
        movement: 1.0,
        max_health_bonus: 0.0,
        health_regen_bonus: 0.0,
        body_damage_bonus: 0.0,
    };

    fn compose(self, next: Self) -> Self {
        Self {
            reload: self.reload * next.reload,
            bullet_damage: self.bullet_damage * next.bullet_damage,
            bullet_speed: self.bullet_speed * next.bullet_speed,
            knockback: self.knockback * next.knockback,
            lifetime: self.lifetime * next.lifetime,
            spread: self.spread * next.spread,
            movement: self.movement * next.movement,
            max_health_bonus: self.max_health_bonus + next.max_health_bonus,
            health_regen_bonus: self.health_regen_bonus + next.health_regen_bonus,
            body_damage_bonus: self.body_damage_bonus + next.body_damage_bonus,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EvolutionDefinition {
    pub kind: EvolutionKind,
    pub parent: Option<EvolutionKind>,
    pub level: u32,
    pub modifiers: EvolutionModifiers,
    pub passive: PassiveKind,
    pub tags: &'static [EvolutionTag],
}

const SUSTAINED: &[EvolutionTag] = &[EvolutionTag::Sustained];
const PRECISION: &[EvolutionTag] = &[EvolutionTag::Precision];
const BURST: &[EvolutionTag] = &[EvolutionTag::Burst];
const CONTROL: &[EvolutionTag] = &[EvolutionTag::AreaControl];
const DURABILITY: &[EvolutionTag] = &[EvolutionTag::Durability];
const DEFENSE: &[EvolutionTag] = &[EvolutionTag::Defense];
const MOBILITY: &[EvolutionTag] = &[EvolutionTag::Mobility];

fn modifiers(
    reload: f32,
    damage: f32,
    speed: f32,
    knockback: f32,
    lifetime: f32,
    spread: f32,
    movement: f32,
    health: f32,
    regen: f32,
    body: f32,
) -> EvolutionModifiers {
    EvolutionModifiers {
        reload,
        bullet_damage: damage,
        bullet_speed: speed,
        knockback,
        lifetime,
        spread,
        movement,
        max_health_bonus: health,
        health_regen_bonus: regen,
        body_damage_bonus: body,
    }
}

pub fn definition(kind: EvolutionKind) -> EvolutionDefinition {
    use EvolutionKind as K;
    let (parent, level, values, passive, tags) = match kind {
        K::Tank => (
            None,
            0,
            EvolutionModifiers::IDENTITY,
            PassiveKind::None,
            SUSTAINED,
        ),
        K::Gunner => (
            Some(K::Tank),
            5,
            modifiers(0.72, 0.78, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0),
            PassiveKind::None,
            SUSTAINED,
        ),
        K::Cannon => (
            Some(K::Tank),
            5,
            modifiers(1.65, 2.35, 0.86, 2.35, 1.15, 1.0, 1.0, 0.0, 0.0, 0.0),
            PassiveKind::None,
            BURST,
        ),
        K::TwinBarrel => (
            Some(K::Tank),
            5,
            modifiers(0.95, 0.82, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0),
            PassiveKind::None,
            SUSTAINED,
        ),
        K::Sniper => (
            Some(K::Tank),
            5,
            modifiers(1.18, 1.22, 1.48, 1.2, 1.70, 1.0, 0.96, -10.0, 0.0, 0.0),
            PassiveKind::None,
            PRECISION,
        ),
        K::RamCore => (
            Some(K::Tank),
            5,
            modifiers(1.10, 0.55, 1.0, 0.8, 1.0, 1.0, 0.94, 45.0, 0.6, 5.0),
            PassiveKind::None,
            DURABILITY,
        ),
        K::Sprayer => (
            Some(K::Tank),
            5,
            modifiers(0.48, 0.35, 0.92, 0.75, 0.85, 1.0, 1.0, 0.0, 0.0, 0.0),
            PassiveKind::None,
            CONTROL,
        ),
        K::Guard => (
            Some(K::Tank),
            5,
            modifiers(1.08, 0.85, 1.0, 1.0, 1.0, 1.0, 0.84, 55.0, 2.5, 2.0),
            PassiveKind::None,
            DEFENSE,
        ),
        K::Flanker => (
            Some(K::Tank),
            5,
            modifiers(0.92, 0.90, 1.05, 1.0, 1.0, 1.0, 1.22, -20.0, -0.3, 0.0),
            PassiveKind::None,
            MOBILITY,
        ),
        K::Minigun => (
            Some(K::Gunner),
            15,
            modifiers(1.0, 0.95, 1.0, 1.0, 1.0, 1.0, 0.96, 0.0, 0.0, 0.0),
            PassiveKind::MinigunSpin,
            SUSTAINED,
        ),
        K::Stabilizer => (
            Some(K::Gunner),
            15,
            modifiers(1.05, 1.08, 1.0, 1.0, 1.1, 1.0, 0.96, 0.0, 0.0, 0.0),
            PassiveKind::Stabilized,
            PRECISION,
        ),
        K::Annihilator => (
            Some(K::Cannon),
            15,
            modifiers(1.35, 1.12, 0.82, 1.25, 1.05, 1.0, 0.92, 10.0, 0.0, 0.0),
            PassiveKind::Splash,
            BURST,
        ),
        K::RailCannon => (
            Some(K::Cannon),
            15,
            modifiers(1.15, 0.82, 1.35, 0.65, 1.35, 1.0, 0.98, -10.0, 0.0, 0.0),
            PassiveKind::DistanceDamage,
            PRECISION,
        ),
        K::QuadBarrel => (
            Some(K::TwinBarrel),
            15,
            modifiers(0.88, 0.78, 1.0, 0.9, 1.0, 1.0, 0.98, 0.0, 0.0, 0.0),
            PassiveKind::AlternatingPairs,
            SUSTAINED,
        ),
        K::Crossfire => (
            Some(K::TwinBarrel),
            15,
            modifiers(0.94, 0.84, 1.0, 1.0, 1.0, 1.0, 1.04, 0.0, 0.0, 0.0),
            PassiveKind::RearKnockback,
            DEFENSE,
        ),
        K::Marksman => (
            Some(K::Sniper),
            15,
            modifiers(1.12, 1.02, 1.08, 1.0, 1.25, 1.0, 0.95, -5.0, 0.0, 0.0),
            PassiveKind::DistanceDamage,
            PRECISION,
        ),
        K::Hunter => (
            Some(K::Sniper),
            15,
            modifiers(0.92, 0.82, 1.0, 0.9, 1.05, 1.0, 1.02, 0.0, 0.0, 0.0),
            PassiveKind::HunterMark,
            PRECISION,
        ),
        K::Juggernaut => (
            Some(K::RamCore),
            15,
            modifiers(1.15, 0.75, 0.9, 1.0, 0.9, 1.0, 0.95, 40.0, 0.5, 4.0),
            PassiveKind::MomentumArmor,
            DURABILITY,
        ),
        K::Interceptor => (
            Some(K::RamCore),
            15,
            modifiers(1.0, 0.9, 1.05, 1.0, 1.0, 1.0, 1.08, 10.0, 0.0, 1.0),
            PassiveKind::FrontalArmor,
            DEFENSE,
        ),
        K::PentaShot => (
            Some(K::Sprayer),
            15,
            modifiers(1.10, 0.82, 0.95, 1.0, 0.95, 1.0, 0.96, 0.0, 0.0, 0.0),
            PassiveKind::PhasedFan,
            CONTROL,
        ),
        K::Needler => (
            Some(K::Sprayer),
            15,
            modifiers(0.85, 0.92, 1.10, 0.7, 1.0, 0.32, 1.04, -5.0, 0.0, 0.0),
            PassiveKind::ConsecutiveHits,
            SUSTAINED,
        ),
        K::Fortress => (
            Some(K::Guard),
            15,
            modifiers(1.18, 0.92, 0.92, 1.1, 1.0, 1.0, 0.82, 35.0, 1.0, 2.0),
            PassiveKind::Entrenched,
            DEFENSE,
        ),
        K::Bulwark => (
            Some(K::Guard),
            15,
            modifiers(1.05, 0.95, 1.0, 1.0, 1.0, 1.0, 0.90, 20.0, 0.5, 1.0),
            PassiveKind::FrontalShield,
            DEFENSE,
        ),
        K::Booster => (
            Some(K::Flanker),
            15,
            modifiers(0.88, 0.82, 1.05, 1.0, 0.95, 1.0, 1.12, -10.0, 0.0, 0.0),
            PassiveKind::BoosterRecoil,
            MOBILITY,
        ),
        K::Fighter => (
            Some(K::Flanker),
            15,
            modifiers(0.92, 0.78, 1.0, 0.9, 1.0, 1.0, 1.05, 0.0, 0.0, 0.0),
            PassiveKind::HitSpeed,
            MOBILITY,
        ),
    };
    EvolutionDefinition {
        kind,
        parent,
        level,
        modifiers: values,
        passive,
        tags,
    }
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
        let option = *options_for_level(level, self.current_kind)?.get(slot)?;
        self.pending_levels.retain(|pending| *pending != level);
        self.current_name = option.name.to_string();
        self.current_kind = option.kind;
        self.chosen.push(ChosenEvolution { level });
        Some(option)
    }

    pub fn choose_kind_for_level(&mut self, level: u32, kind: EvolutionKind) -> bool {
        self.queue_reached_levels(level);
        let Some(active_level) = self.active_level() else {
            return false;
        };
        let Some(slot) = options_for_level(active_level, self.current_kind)
            .and_then(|options| options.iter().position(|option| option.kind == kind))
        else {
            return false;
        };

        self.choose_active(slot).is_some()
    }

    pub fn body_scale(&self) -> f32 {
        match base_kind(self.current_kind) {
            EvolutionKind::Cannon => 1.04,
            EvolutionKind::Sniper => 0.92,
            EvolutionKind::Sprayer => 0.96,
            EvolutionKind::Guard => 0.88,
            EvolutionKind::Flanker => 0.95,
            _ => 1.0,
        }
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
            EvolutionKind::Minigun => &MINIGUN_BARRELS,
            EvolutionKind::Stabilizer => &STABILIZER_BARRELS,
            EvolutionKind::Annihilator => &ANNIHILATOR_BARRELS,
            EvolutionKind::RailCannon => &RAIL_CANNON_BARRELS,
            EvolutionKind::QuadBarrel => &QUAD_BARRELS,
            EvolutionKind::Crossfire => &CROSSFIRE_BARRELS,
            EvolutionKind::Marksman => &MARKSMAN_BARRELS,
            EvolutionKind::Hunter => &HUNTER_BARRELS,
            EvolutionKind::Juggernaut => &JUGGERNAUT_BARRELS,
            EvolutionKind::Interceptor => &INTERCEPTOR_BARRELS,
            EvolutionKind::PentaShot => &PENTA_BARRELS,
            EvolutionKind::Needler => &NEEDLER_BARRELS,
            EvolutionKind::Fortress => &FORTRESS_BARRELS,
            EvolutionKind::Bulwark => &BULWARK_BARRELS,
            EvolutionKind::Booster => &BOOSTER_BARRELS,
            EvolutionKind::Fighter => &FIGHTER_BARRELS,
        }
    }

    pub fn reload_multiplier(&self) -> f32 {
        effective_modifiers(self.current_kind).reload
    }

    pub fn bullet_damage_multiplier(&self) -> f32 {
        effective_modifiers(self.current_kind).bullet_damage
    }

    pub fn bullet_speed_multiplier(&self) -> f32 {
        effective_modifiers(self.current_kind).bullet_speed
    }

    pub fn bullet_knockback_multiplier(&self) -> f32 {
        effective_modifiers(self.current_kind).knockback
    }

    pub fn projectile_lifetime_multiplier(&self) -> f32 {
        effective_modifiers(self.current_kind).lifetime
    }

    pub fn spread_radians(&self) -> f32 {
        let base = match base_kind(self.current_kind) {
            EvolutionKind::Gunner => 0.045,
            EvolutionKind::Sprayer => 0.24,
            EvolutionKind::Cannon => 0.025,
            EvolutionKind::Sniper => 0.006,
            _ => 0.0,
        };
        base * effective_modifiers(self.current_kind).spread
    }

    pub fn movement_multiplier(&self) -> f32 {
        effective_modifiers(self.current_kind).movement
    }

    pub fn max_health_bonus(&self) -> i32 {
        effective_modifiers(self.current_kind)
            .max_health_bonus
            .round() as i32
    }

    pub fn health_regen_bonus(&self) -> f32 {
        effective_modifiers(self.current_kind).health_regen_bonus
    }

    pub fn body_damage_bonus(&self) -> u32 {
        effective_modifiers(self.current_kind)
            .body_damage_bonus
            .max(0.0)
            .round() as u32
    }

    pub fn passive(&self) -> PassiveKind {
        definition(self.current_kind).passive
    }

    pub fn penetration_bonus(&self) -> u32 {
        u32::from(self.current_kind == EvolutionKind::RailCannon) * 2
    }

    pub fn active_options(&self) -> Option<&'static [EvolutionOption]> {
        options_for_level(self.active_level()?, self.current_kind)
    }
}

fn effective_modifiers(kind: EvolutionKind) -> EvolutionModifiers {
    let definition = definition(kind);
    debug_assert_eq!(definition.kind, kind);
    definition.parent.map_or(definition.modifiers, |parent| {
        effective_modifiers(parent).compose(definition.modifiers)
    })
}

fn base_kind(kind: EvolutionKind) -> EvolutionKind {
    let definition = definition(kind);
    match definition.parent {
        Some(EvolutionKind::Tank) | None => kind,
        Some(parent) => base_kind(parent),
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

pub const MAX_BARRELS: usize = 5;

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
const MINIGUN_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    width: 7.0,
    length: 34.0,
    ..GUNNER_BARRELS[0]
}];
const STABILIZER_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    width: 5.2,
    length: 40.0,
    ..GUNNER_BARRELS[0]
}];
const ANNIHILATOR_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    width: 15.5,
    length: 34.0,
    ..CANNON_BARRELS[0]
}];
const RAIL_CANNON_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    width: 4.0,
    length: 50.0,
    ..CANNON_BARRELS[0]
}];
const QUAD_BARRELS: [BarrelSpec; 4] = [
    BarrelSpec {
        lateral_offset: -9.0,
        damage_multiplier: 0.58,
        ..TWIN_BARRELS[0]
    },
    BarrelSpec {
        lateral_offset: -3.0,
        damage_multiplier: 0.58,
        ..TWIN_BARRELS[0]
    },
    BarrelSpec {
        lateral_offset: 3.0,
        damage_multiplier: 0.58,
        ..TWIN_BARRELS[0]
    },
    BarrelSpec {
        lateral_offset: 9.0,
        damage_multiplier: 0.58,
        ..TWIN_BARRELS[0]
    },
];
const CROSSFIRE_BARRELS: [BarrelSpec; 4] = [
    BarrelSpec {
        lateral_offset: -5.0,
        damage_multiplier: 0.72,
        ..TWIN_BARRELS[0]
    },
    BarrelSpec {
        lateral_offset: 5.0,
        damage_multiplier: 0.72,
        ..TWIN_BARRELS[0]
    },
    BarrelSpec {
        angle_offset: std::f32::consts::PI,
        lateral_offset: -5.0,
        damage_multiplier: 0.55,
        ..TWIN_BARRELS[0]
    },
    BarrelSpec {
        angle_offset: std::f32::consts::PI,
        lateral_offset: 5.0,
        damage_multiplier: 0.55,
        ..TWIN_BARRELS[0]
    },
];
const MARKSMAN_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    width: 4.2,
    length: 53.0,
    ..SNIPER_BARRELS[0]
}];
const HUNTER_BARRELS: [BarrelSpec; 2] = [
    BarrelSpec {
        lateral_offset: -4.0,
        damage_multiplier: 0.72,
        ..SNIPER_BARRELS[0]
    },
    BarrelSpec {
        lateral_offset: 4.0,
        damage_multiplier: 0.72,
        ..SNIPER_BARRELS[0]
    },
];
const JUGGERNAUT_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    width: 5.0,
    length: 24.0,
    ..RAM_CORE_BARRELS[0]
}];
const INTERCEPTOR_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    width: 6.0,
    length: 28.0,
    ..RAM_CORE_BARRELS[0]
}];
const PENTA_BARRELS: [BarrelSpec; 5] = [
    BarrelSpec {
        angle_offset: -0.32,
        lateral_offset: -8.0,
        damage_multiplier: 0.48,
        ..SPRAYER_BARRELS[0]
    },
    BarrelSpec {
        angle_offset: -0.16,
        lateral_offset: -4.0,
        damage_multiplier: 0.48,
        ..SPRAYER_BARRELS[0]
    },
    BarrelSpec {
        angle_offset: 0.0,
        lateral_offset: 0.0,
        damage_multiplier: 0.48,
        ..SPRAYER_BARRELS[0]
    },
    BarrelSpec {
        angle_offset: 0.16,
        lateral_offset: 4.0,
        damage_multiplier: 0.48,
        ..SPRAYER_BARRELS[0]
    },
    BarrelSpec {
        angle_offset: 0.32,
        lateral_offset: 8.0,
        damage_multiplier: 0.48,
        ..SPRAYER_BARRELS[0]
    },
];
const NEEDLER_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    width: 4.0,
    length: 35.0,
    damage_multiplier: 1.0,
    ..SPRAYER_BARRELS[0]
}];
const FORTRESS_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    width: 8.0,
    length: 30.0,
    ..GUARD_BARRELS[0]
}];
const BULWARK_BARRELS: [BarrelSpec; 1] = [BarrelSpec {
    width: 6.5,
    length: 31.0,
    ..GUARD_BARRELS[0]
}];
const BOOSTER_BARRELS: [BarrelSpec; 3] = [
    FLANKER_BARRELS[0],
    BarrelSpec {
        angle_offset: std::f32::consts::PI,
        lateral_offset: -5.0,
        damage_multiplier: 0.62,
        ..FLANKER_BARRELS[1]
    },
    BarrelSpec {
        angle_offset: std::f32::consts::PI,
        lateral_offset: 5.0,
        damage_multiplier: 0.62,
        ..FLANKER_BARRELS[1]
    },
];
const FIGHTER_BARRELS: [BarrelSpec; 4] = [
    FLANKER_BARRELS[0],
    FLANKER_BARRELS[1],
    BarrelSpec {
        angle_offset: std::f32::consts::FRAC_PI_2,
        damage_multiplier: 0.58,
        ..FLANKER_BARRELS[0]
    },
    BarrelSpec {
        angle_offset: -std::f32::consts::FRAC_PI_2,
        damage_multiplier: 0.58,
        ..FLANKER_BARRELS[0]
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

macro_rules! advanced_option {
    ($name:literal, $kind:ident, $description:literal, $parts:ident, $scale:expr, $color:expr) => {
        EvolutionOption {
            name: $name,
            kind: EvolutionKind::$kind,
            description: $description,
            background: $color,
            lower_background: [$color[0] * 0.72, $color[1] * 0.72, $color[2] * 0.72, 1.0],
            visual: TankVisual {
                body_scale: $scale,
                parts: &$parts,
            },
        }
    };
}

const GUNNER_ADVANCED: [EvolutionOption; 2] = [
    advanced_option!(
        "Minigun",
        Minigun,
        "Sustained fire spins up by 25%, at the cost of growing spread.",
        GUNNER_PARTS,
        1.0,
        [0.34, 0.87, 0.72, 1.0]
    ),
    advanced_option!(
        "Stabilizer",
        Stabilizer,
        "Slow down to gain a much tighter, faster stream.",
        SNIPER_PARTS,
        1.0,
        [0.42, 0.78, 0.92, 1.0]
    ),
];
const CANNON_ADVANCED: [EvolutionOption; 2] = [
    advanced_option!(
        "Annihilator",
        Annihilator,
        "Huge slow shells create a falloff blast on impact.",
        CANNON_PARTS,
        1.08,
        [0.95, 0.35, 0.32, 1.0]
    ),
    advanced_option!(
        "Rail Cannon",
        RailCannon,
        "Thin penetrators accelerate damage over long travel distance.",
        SNIPER_PARTS,
        1.0,
        [0.78, 0.52, 0.94, 1.0]
    ),
];
const TWIN_ADVANCED: [EvolutionOption; 2] = [
    advanced_option!(
        "Quad Barrel",
        QuadBarrel,
        "Alternating paired volleys maintain steady forward pressure.",
        TWIN_PARTS,
        1.0,
        [0.32, 0.82, 0.88, 1.0]
    ),
    advanced_option!(
        "Crossfire",
        Crossfire,
        "Forward and rear pairs punish pursuers with heavy knockback.",
        FLANKER_PARTS,
        1.0,
        [0.45, 0.72, 0.92, 1.0]
    ),
];
const SNIPER_ADVANCED: [EvolutionOption; 2] = [
    advanced_option!(
        "Marksman",
        Marksman,
        "Damage rises with projectile travel distance.",
        SNIPER_PARTS,
        0.90,
        [0.45, 0.92, 0.34, 1.0]
    ),
    advanced_option!(
        "Hunter",
        Hunter,
        "Mark a target, then punish it with two stronger follow-up hits.",
        TWIN_PARTS,
        0.94,
        [0.66, 0.89, 0.34, 1.0]
    ),
];
const RAM_ADVANCED: [EvolutionOption; 2] = [
    advanced_option!(
        "Juggernaut",
        Juggernaut,
        "Momentum increases body damage and contact resistance.",
        RAM_CORE_PARTS,
        1.10,
        [0.38, 0.52, 0.94, 1.0]
    ),
    advanced_option!(
        "Interceptor",
        Interceptor,
        "A visible frontal armor arc reduces incoming projectile damage.",
        GUARD_PARTS,
        1.02,
        [0.35, 0.65, 0.95, 1.0]
    ),
];
const SPRAYER_ADVANCED: [EvolutionOption; 2] = [
    advanced_option!(
        "Penta Shot",
        PentaShot,
        "A phased five-shot fan fills gaps and controls space.",
        SPRAYER_PARTS,
        0.98,
        [0.98, 0.65, 0.22, 1.0]
    ),
    advanced_option!(
        "Needler",
        Needler,
        "Repeated hits on one target ramp damage to five stacks.",
        GUNNER_PARTS,
        0.94,
        [0.94, 0.78, 0.25, 1.0]
    ),
];
const GUARD_ADVANCED: [EvolutionOption; 2] = [
    advanced_option!(
        "Fortress",
        Fortress,
        "Settle and stop firing to gain regeneration and damage reduction.",
        GUARD_PARTS,
        0.92,
        [0.61, 0.39, 0.92, 1.0]
    ),
    advanced_option!(
        "Bulwark",
        Bulwark,
        "A frontal shield absorbs projectiles and recharges out of combat.",
        GUARD_PARTS,
        0.90,
        [0.50, 0.42, 0.91, 1.0]
    ),
];
const FLANKER_ADVANCED: [EvolutionOption; 2] = [
    advanced_option!(
        "Booster",
        Booster,
        "Rear fire adds forward acceleration up to a controlled cap.",
        FLANKER_PARTS,
        0.94,
        [0.96, 0.84, 0.35, 1.0]
    ),
    advanced_option!(
        "Fighter",
        Fighter,
        "All-side coverage and confirmed hits trigger short speed bursts.",
        FLANKER_PARTS,
        0.96,
        [0.91, 0.74, 0.28, 1.0]
    ),
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
                    for (slot, option) in LEVEL_5_OPTIONS.iter().enumerate().skip(row_start).take(2)
                    {
                        row.spawn((
                            Button,
                            EvolutionOptionButton { slot },
                            evolution_card_node(),
                            BackgroundColor(Color::srgba(0.33, 0.34, 0.35, 1.0)),
                            BorderColor::all(Color::srgba(0.33, 0.34, 0.35, 1.0)),
                        ))
                        .with_children(|card| {
                            spawn_card_background(card, *option);
                            spawn_tank_visual(card, option.visual);
                            spawn_card_label(card, option.name);
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
    mut root: Query<&mut Visibility, (With<EvolutionMenuRoot>, Without<EvolutionOptionButton>)>,
    mut cards: Query<&mut Visibility, (With<EvolutionOptionButton>, Without<EvolutionMenuRoot>)>,
) {
    let visible = *phase == GamePhase::Playing && state.active_level().is_some();
    for mut visibility in root.iter_mut() {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible {
        for mut visibility in &mut cards {
            *visibility = Visibility::Hidden;
        }
    }
}

pub fn refresh_evolution_cards(
    mut commands: Commands,
    phase: Res<GamePhase>,
    state: Res<EvolutionState>,
    mut buttons: Query<(Entity, &EvolutionOptionButton, &mut Visibility), With<Button>>,
) {
    if !state.is_changed() {
        return;
    }
    if *phase != GamePhase::Playing || state.active_level().is_none() {
        for (_, _, mut visibility) in &mut buttons {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let options = state.active_options().unwrap_or(&LEVEL_5_OPTIONS);
    for (entity, button, mut visibility) in &mut buttons {
        let Some(option) = options.get(button.slot).copied() else {
            *visibility = Visibility::Hidden;
            continue;
        };
        *visibility = Visibility::Visible;
        commands.entity(entity).despawn_related::<Children>();
        commands.entity(entity).with_children(|card| {
            spawn_card_background(card, option);
            spawn_tank_visual(card, option.visual);
            spawn_card_label(card, option.name);
        });
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
        .and_then(|(_, button)| state.active_options()?.get(button.slot));

    for (mut text, mut visibility) in descriptions.iter_mut() {
        if let Some(option) = hovered {
            **text = format!("{}\n{}", option.name, option.description);
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn options_for_level(
    level: u32,
    current_kind: EvolutionKind,
) -> Option<&'static [EvolutionOption]> {
    match level {
        LEVEL_5 => Some(&LEVEL_5_OPTIONS),
        LEVEL_15 => match current_kind {
            EvolutionKind::Gunner => Some(&GUNNER_ADVANCED),
            EvolutionKind::Cannon => Some(&CANNON_ADVANCED),
            EvolutionKind::TwinBarrel => Some(&TWIN_ADVANCED),
            EvolutionKind::Sniper => Some(&SNIPER_ADVANCED),
            EvolutionKind::RamCore => Some(&RAM_ADVANCED),
            EvolutionKind::Sprayer => Some(&SPRAYER_ADVANCED),
            EvolutionKind::Guard => Some(&GUARD_ADVANCED),
            EvolutionKind::Flanker => Some(&FLANKER_ADVANCED),
            _ => None,
        },
        _ => None,
    }
}

pub fn advanced_kinds(parent: EvolutionKind) -> Option<[EvolutionKind; 2]> {
    let options = options_for_level(LEVEL_15, parent)?;
    Some([options[0].kind, options[1].kind])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_five_and_fifteen_queue_once() {
        let mut state = EvolutionState::default();

        state.queue_reached_levels(100);
        state.queue_reached_levels(100);

        assert_eq!(state.pending_levels, vec![5, 15]);
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
    fn level_fifteen_options_follow_parent_branch() {
        assert_eq!(options_for_level(5, EvolutionKind::Tank).unwrap().len(), 8);
        assert_eq!(
            options_for_level(15, EvolutionKind::Gunner).unwrap().len(),
            2
        );
        assert_eq!(
            options_for_level(15, EvolutionKind::Gunner).unwrap()[0].kind,
            EvolutionKind::Minigun
        );
        assert!(options_for_level(15, EvolutionKind::Tank).is_none());
        assert!(options_for_level(100, EvolutionKind::Tank).is_none());
    }

    #[test]
    fn level_five_evolutions_have_supported_barrel_counts() {
        for option in LEVEL_5_OPTIONS {
            let state = EvolutionState {
                current_kind: option.kind,
                ..default()
            };

            assert!(!state.barrel_specs().is_empty());
            assert!(state.barrel_specs().len() <= MAX_BARRELS);
        }
    }

    #[test]
    fn every_advanced_evolution_has_a_valid_parent_and_barrels() {
        for options in [
            &GUNNER_ADVANCED,
            &CANNON_ADVANCED,
            &TWIN_ADVANCED,
            &SNIPER_ADVANCED,
            &RAM_ADVANCED,
            &SPRAYER_ADVANCED,
            &GUARD_ADVANCED,
            &FLANKER_ADVANCED,
        ] {
            for option in options {
                let definition = definition(option.kind);
                let state = EvolutionState {
                    current_kind: option.kind,
                    ..default()
                };
                assert_eq!(definition.level, 15);
                assert!(definition.parent.is_some());
                assert!(!state.barrel_specs().is_empty());
                assert!(state.barrel_specs().len() <= MAX_BARRELS);
            }
        }
    }

    #[test]
    fn sprayer_preserves_fractional_projectile_damage() {
        let state = EvolutionState {
            current_kind: EvolutionKind::Sprayer,
            ..default()
        };
        assert!(
            (crate::constants::BASE_PROJECTILE_DAMAGE * state.bullet_damage_multiplier() - 1.05)
                .abs()
                < 0.0001
        );
    }
}
