// Window
pub const WINDOW_WIDTH: f32 = 1024.0;
pub const WINDOW_HEIGHT: f32 = 768.0;
pub const WINDOW_TITLE: &str = "Polycore";

// Player
pub const PLAYER_RADIUS: f32 = 20.0;
pub const PLAYER_MAX_HEALTH: u32 = 100;
pub const PLAYER_SPEED: f32 = 300.0;
pub const PLAYER_ACCEL_TIME: f32 = 1.0;
pub const PLAYER_COLLISION_KNOCKBACK_SPEED: f32 = 420.0;
pub const PLAYER_KNOCKBACK_DAMPING: f32 = 10.0;
pub const PLAYER_DAMAGE_COOLDOWN: f32 = 0.45;
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
pub const SHAPE_SPAWN_SAFE_RADIUS: f32 = 180.0;
pub const SHAPE_KNOCKBACK_SPEED: f32 = 180.0;
pub const SHAPE_COLLISION_KNOCKBACK_SPEED: f32 = 220.0;
pub const SHAPE_SHAPE_KNOCKBACK_SPEED: f32 = 140.0;
pub const SHAPE_SHAPE_DAMAGE: u32 = 1;
pub const SHAPE_SHAPE_DAMAGE_COOLDOWN: f32 = 0.35;
pub const SHAPE_KNOCKBACK_DAMPING: f32 = 9.0;

// Tank health bar
pub const HEALTH_BAR_WIDTH: f32 = 58.0;
pub const HEALTH_BAR_HEIGHT: f32 = 7.0;
pub const HEALTH_BAR_OFFSET_Y: f32 = -35.0;
pub const HEALTH_BAR_BG_COLOR: [f32; 4] = [0.25, 0.25, 0.26, 1.0];
pub const HEALTH_BAR_FILL_COLOR: [f32; 4] = [0.45, 1.0, 0.55, 1.0];

// Shape health bar
pub const SHAPE_HEALTH_BAR_WIDTH: f32 = 44.0;
pub const SHAPE_HEALTH_BAR_HEIGHT: f32 = 6.0;
pub const SHAPE_HEALTH_BAR_OFFSET_Y: f32 = -34.0;

// XP
pub const XP_PER_LEVEL: u32 = 100;

// Colors
pub const PROJECTILE_COLOR: [f32; 4] = [1.0, 0.8, 0.2, 1.0];
pub const ENEMY_COLOR: [f32; 4] = [0.9, 0.2, 0.2, 1.0];
pub const BARREL_COLOR: [f32; 4] = [0.4, 0.4, 0.4, 1.0];
pub const BG_COLOR: [f32; 4] = [0.05, 0.05, 0.08, 1.0];
pub const GRID_COLOR: [f32; 4] = [0.1, 0.1, 0.15, 1.0];
pub const BORDER_COLOR: [f32; 4] = [0.35, 0.38, 0.46, 1.0];
pub const GRID_SPACING: f32 = 64.0;
pub const GRID_EXTENT: f32 = 2000.0;
pub const BORDER_THICKNESS: f32 = 8.0;

pub fn arena_half_extent() -> f32 {
    GRID_EXTENT / 2.0
}

// Shape health/XP: low-tier shapes have doubled HP, hexagons have quadrupled HP.
pub fn shape_health(sides: u32) -> u32 {
    let base_health = (sides - 1).max(1);
    if sides == 6 {
        base_health * 4
    } else {
        base_health * 2
    }
}
pub fn shape_xp(sides: u32) -> u32 {
    shape_health(sides) * 5
}
pub fn shape_damage(sides: u32) -> u32 {
    (sides - 2).max(1) * 5
}
