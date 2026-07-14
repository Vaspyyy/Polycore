// Window
pub const WINDOW_WIDTH: f32 = 1024.0;
pub const WINDOW_HEIGHT: f32 = 768.0;
pub const WINDOW_TITLE: &str = "Polycore";

// Player
pub const PLAYER_RADIUS: f32 = 20.0;
pub const PLAYER_MAX_HEALTH: f32 = 50.0;
pub const PLAYER_SPEED: f32 = 300.0;
pub const PLAYER_ACCEL_TIME: f32 = 1.0;
pub const PLAYER_COLLISION_KNOCKBACK_SPEED: f32 = 420.0;
pub const PLAYER_KNOCKBACK_DAMPING: f32 = 10.0;
pub const PLAYER_DAMAGE_COOLDOWN: f32 = 0.45;
pub const PLAYER_COLOR: [f32; 4] = [0.2, 0.6, 1.0, 1.0];

// Shooting
pub const SHOOT_COOLDOWN: f32 = 0.6;
pub const PROJECTILE_RADIUS: f32 = 4.0;
pub const PROJECTILE_SPEED: f32 = 400.0;
pub const PROJECTILE_LIFETIME: f32 = 2.0;

// Camera zoom
pub const CAMERA_MIN_ZOOM: f32 = 0.55;
pub const CAMERA_SOFT_MAX_ZOOM: f32 = 1.55;
pub const CAMERA_MAX_ZOOM: f32 = 2.25;
pub const CAMERA_ZOOM_SPEED: f32 = 0.14;

// Shapes (XP polygons)
pub const SHAPE_RADIUS: f32 = 25.0;
pub const SHAPE_SPAWN_INTERVAL: f32 = 0.25;
pub const SHAPES_PER_LIVING_TANK: usize = 6;
pub const SHAPE_MAX_COUNT: usize = 100;
pub const SHAPE_SPAWN_SAFE_RADIUS: f32 = 180.0;
pub const SHAPE_KNOCKBACK_SPEED: f32 = 180.0;
pub const SHAPE_COLLISION_KNOCKBACK_SPEED: f32 = 220.0;
pub const SHAPE_SHAPE_KNOCKBACK_SPEED: f32 = 140.0;
pub const SHAPE_SHAPE_DAMAGE: f32 = 1.0;
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
pub const BASE_XP_PER_LEVEL: u32 = 100;
pub const XP_GROWTH_PER_LEVEL: u32 = 35;
pub const BASE_PROJECTILE_DAMAGE: f32 = 3.0;
pub const BASE_BODY_DAMAGE: f32 = 5.0;
pub const SPAWN_PROTECTION_SECS: f32 = 2.0;
pub const SPATIAL_CELL_SIZE: f32 = 128.0;

// Colors
pub const ENEMY_COLOR: [f32; 4] = [0.9, 0.2, 0.2, 1.0];
pub const BARREL_COLOR: [f32; 4] = [0.4, 0.4, 0.4, 1.0];
pub const BG_COLOR: [f32; 4] = [0.065, 0.065, 0.095, 1.0];
pub const GRID_COLOR: [f32; 4] = [0.115, 0.115, 0.165, 1.0];
pub const BORDER_COLOR: [f32; 4] = [0.35, 0.38, 0.46, 1.0];
pub const GRID_SPACING: f32 = 64.0;
pub const GRID_EXTENT: f32 = 4000.0;
pub const BORDER_THICKNESS: f32 = 8.0;
pub const OUTLINE_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub const OUTLINE_THICKNESS: f32 = 3.0;

pub fn arena_half_extent() -> f32 {
    GRID_EXTENT / 2.0
}

// Shape health creates distinct base-damage shot breakpoints: 2, 3, 4, and 8.
pub fn shape_health(sides: u32) -> f32 {
    match sides {
        3 => 4.0,
        4 => 8.0,
        5 => 12.0,
        6 => 24.0,
        _ => 4.0,
    }
}

pub fn shape_xp(sides: u32) -> u32 {
    match sides {
        3 => 20,
        4 => 30,
        5 => 40,
        6 => 100,
        _ => 20,
    }
}
pub fn shape_damage(sides: u32) -> f32 {
    ((sides - 2).max(1) * 5) as f32
}

pub fn xp_required_for_level(level: u32) -> u32 {
    BASE_XP_PER_LEVEL.saturating_add(XP_GROWTH_PER_LEVEL.saturating_mul(level.saturating_sub(1)))
}

pub fn consume_level_ups(xp: &mut u32, level: &mut u32) -> u32 {
    let mut gained = 0u32;
    loop {
        let required = xp_required_for_level(*level);
        if *xp < required {
            break;
        }
        *xp -= required;
        *level = level.saturating_add(1);
        gained = gained.saturating_add(1);
    }
    gained
}

#[cfg(test)]
fn progression_telemetry_seconds(target_level: u32) -> f32 {
    let mut elapsed = 0.0;
    let mut xp = 0.0;
    let mut level = 1;
    while level < target_level && elapsed < 3_600.0 {
        // Recorded competent-farming throughput rises as reload, damage, and evolution improve.
        xp += 2.8 + (level.saturating_sub(1) as f32 * 0.3);
        elapsed += 1.0;
        while xp >= xp_required_for_level(level) as f32 {
            xp -= xp_required_for_level(level) as f32;
            level = level.saturating_add(1);
        }
    }
    elapsed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xp_requirements_escalate_and_support_multiple_levels() {
        assert_eq!(xp_required_for_level(1), 100);
        assert_eq!(xp_required_for_level(2), 135);
        let mut xp = 405;
        let mut level = 1;
        assert_eq!(consume_level_ups(&mut xp, &mut level), 3);
        assert_eq!((xp, level), (0, 4));
    }

    #[test]
    fn progression_telemetry_hits_evolution_targets() {
        let level_five = progression_telemetry_seconds(5);
        let level_fifteen = progression_telemetry_seconds(15);
        assert!((144.0..=216.0).contains(&level_five));
        assert!((720.0..=1080.0).contains(&level_fifteen));
    }
}
