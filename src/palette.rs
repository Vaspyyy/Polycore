use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PaletteId {
    #[default]
    CoreBlue,
    CircuitMint,
    Ember,
    Aurora,
    CrownGold,
    SignalOrange,
    Verdant,
    RoyalViolet,
    NeonLime,
    Obsidian,
    ApexWhite,
    Spectrum,
}

impl PaletteId {
    pub const ALL: [Self; 12] = [
        Self::CoreBlue,
        Self::CircuitMint,
        Self::Ember,
        Self::Aurora,
        Self::CrownGold,
        Self::SignalOrange,
        Self::Verdant,
        Self::RoyalViolet,
        Self::NeonLime,
        Self::Obsidian,
        Self::ApexWhite,
        Self::Spectrum,
    ];

    pub fn name(self) -> &'static str {
        PALETTES[self as usize].name
    }
}

#[derive(Clone, Copy)]
pub struct TankPalette {
    pub name: &'static str,
    pub body: [f32; 4],
    pub barrel: [f32; 4],
    pub projectile: [f32; 4],
}

const fn rgba(rgb: [u8; 3]) -> [f32; 4] {
    [
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
        1.0,
    ]
}

pub const PALETTES: [TankPalette; 12] = [
    TankPalette {
        name: "Core Blue",
        body: rgba([51, 153, 255]),
        barrel: rgba([104, 107, 115]),
        projectile: rgba([255, 208, 74]),
    },
    TankPalette {
        name: "Circuit Mint",
        body: rgba([53, 208, 160]),
        barrel: rgba([82, 96, 109]),
        projectile: rgba([168, 255, 229]),
    },
    TankPalette {
        name: "Ember",
        body: rgba([240, 82, 82]),
        barrel: rgba([110, 83, 83]),
        projectile: rgba([255, 179, 71]),
    },
    TankPalette {
        name: "Aurora",
        body: rgba([85, 199, 232]),
        barrel: rgba([216, 239, 245]),
        projectile: rgba([183, 250, 255]),
    },
    TankPalette {
        name: "Crown Gold",
        body: rgba([231, 184, 75]),
        barrel: rgba([117, 98, 61]),
        projectile: rgba([255, 241, 168]),
    },
    TankPalette {
        name: "Signal Orange",
        body: rgba([244, 123, 58]),
        barrel: rgba([107, 91, 83]),
        projectile: rgba([255, 208, 168]),
    },
    TankPalette {
        name: "Verdant",
        body: rgba([84, 201, 107]),
        barrel: rgba([80, 104, 84]),
        projectile: rgba([200, 255, 208]),
    },
    TankPalette {
        name: "Royal Violet",
        body: rgba([138, 99, 210]),
        barrel: rgba([97, 87, 116]),
        projectile: rgba([216, 195, 255]),
    },
    TankPalette {
        name: "Neon Lime",
        body: rgba([165, 217, 54]),
        barrel: rgba([92, 104, 72]),
        projectile: rgba([236, 255, 154]),
    },
    TankPalette {
        name: "Obsidian",
        body: rgba([42, 45, 53]),
        barrel: rgba([146, 153, 168]),
        projectile: rgba([242, 244, 248]),
    },
    TankPalette {
        name: "Apex White",
        body: rgba([232, 237, 244]),
        barrel: rgba([119, 128, 143]),
        projectile: rgba([118, 217, 255]),
    },
    TankPalette {
        name: "Spectrum",
        body: rgba([217, 95, 176]),
        barrel: rgba([86, 207, 225]),
        projectile: rgba([255, 209, 102]),
    },
];

pub const BOT_PALETTES: [TankPalette; 5] = [
    TankPalette {
        name: "Bot Red",
        body: rgba([240, 68, 68]),
        barrel: rgba([112, 75, 75]),
        projectile: rgba([255, 150, 118]),
    },
    TankPalette {
        name: "Bot Amber",
        body: rgba([229, 155, 61]),
        barrel: rgba([108, 91, 65]),
        projectile: rgba([255, 215, 117]),
    },
    TankPalette {
        name: "Bot Magenta",
        body: rgba([200, 95, 212]),
        barrel: rgba([99, 77, 111]),
        projectile: rgba([244, 172, 255]),
    },
    TankPalette {
        name: "Bot Green",
        body: rgba([67, 189, 114]),
        barrel: rgba([72, 105, 83]),
        projectile: rgba([149, 244, 183]),
    },
    TankPalette {
        name: "Bot Cyan",
        body: rgba([59, 175, 201]),
        barrel: rgba([70, 99, 108]),
        projectile: rgba([151, 230, 247]),
    },
];

#[derive(Clone)]
pub struct PaletteMaterialSet {
    pub body: Handle<ColorMaterial>,
    pub barrel: Handle<ColorMaterial>,
    pub projectile: Handle<ColorMaterial>,
}

#[derive(Resource, Clone)]
pub struct PaletteMaterials {
    player: Vec<PaletteMaterialSet>,
    bots: Vec<PaletteMaterialSet>,
}

impl PaletteMaterials {
    pub fn player(&self, id: PaletteId) -> &PaletteMaterialSet {
        &self.player[id as usize]
    }

    pub fn bot(&self, index: usize) -> &PaletteMaterialSet {
        &self.bots[index % self.bots.len()]
    }
}

#[derive(Component, Clone, Copy)]
pub struct BotPalette(pub usize);

pub fn setup_palette_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let make = |palette: TankPalette, materials: &mut Assets<ColorMaterial>| PaletteMaterialSet {
        body: materials.add(Color::srgba(
            palette.body[0],
            palette.body[1],
            palette.body[2],
            palette.body[3],
        )),
        barrel: materials.add(Color::srgba(
            palette.barrel[0],
            palette.barrel[1],
            palette.barrel[2],
            palette.barrel[3],
        )),
        projectile: materials.add(Color::srgba(
            palette.projectile[0],
            palette.projectile[1],
            palette.projectile[2],
            palette.projectile[3],
        )),
    };
    commands.insert_resource(PaletteMaterials {
        player: PALETTES
            .iter()
            .copied()
            .map(|palette| make(palette, &mut materials))
            .collect(),
        bots: BOT_PALETTES
            .iter()
            .copied()
            .map(|palette| make(palette, &mut materials))
            .collect(),
    });
}
