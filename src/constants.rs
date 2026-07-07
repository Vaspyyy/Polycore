// Window
pub const WINDOW_WIDTH: f32 = 1024.0;
pub const WINDOW_HEIGHT: f32 = 768.0;
pub const WINDOW_TITLE: &str = "Polycore";

// Player
pub const PLAYER_RADIUS: f32 = 20.0;
pub const PLAYER_SPEED: f32 = 300.0;
pub const PLAYER_COLOR: [f32; 4] = [0.2, 0.6, 1.0, 1.0];

// Shooting
pub const SHOOT_COOLDOWN: f32 = 0.3;
pub const PROJECTILE_RADIUS: f32 = 4.0;
pub const PROJECTILE_SPEED: f32 = 600.0;
pub const PROJECTILE_LIFETIME: f32 = 2.0;

// Shapes (XP polygons)
pub const SHAPE_RADIUS: f32 = 25.0;
pub const SHAPE_SPAWN_INTERVAL: f32 = 1.5;
pub const SHAPE_MAX_COUNT: usize = 20;

// XP
pub const XP_PER_LEVEL: u32 = 100;

// Colors
pub const PROJECTILE_COLOR: [f32; 4] = [1.0, 0.8, 0.2, 1.0];
pub const ENEMY_COLOR: [f32; 4] = [0.9, 0.2, 0.2, 1.0];
pub const BARREL_COLOR: [f32; 4] = [0.4, 0.4, 0.4, 1.0];
pub const BG_COLOR: [f32; 4] = [0.05, 0.05, 0.08, 1.0];
pub const GRID_COLOR: [f32; 4] = [0.1, 0.1, 0.15, 1.0];
pub const GRID_SPACING: f32 = 64.0;
pub const GRID_EXTENT: f32 = 2000.0;

// Shape health/XP: sides - 1 = HP, HP * 5 = XP
pub fn shape_health(sides: u32) -> u32 { (sides - 1).max(1) }
pub fn shape_xp(sides: u32) -> u32 { shape_health(sides) * 5 }
