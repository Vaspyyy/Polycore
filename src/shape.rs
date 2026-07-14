use crate::{
    constants,
    enemy_bot::{EnemyBot, EnemyBotHealth},
    hud::UpgradeState,
    player::{Player, PlayerHealth},
    rng::Rng,
};
use bevy::prelude::*;

#[derive(Component)]
pub struct Shape;

#[derive(Component)]
pub struct Health(pub f32);

#[derive(Component)]
pub struct MaxHealth(pub f32);

#[derive(Component)]
pub struct XpValue(pub u32);

#[derive(Component)]
pub struct ShapeDamage(pub f32);

#[derive(Component)]
pub struct ShapeKind {
    pub sides: u32,
}

impl ShapeKind {
    pub fn name(&self) -> &'static str {
        match self.sides {
            3 => "Triangle",
            4 => "Square",
            5 => "Pentagon",
            6 => "Hexagon",
            _ => "Shape",
        }
    }
}

#[derive(Component, Default)]
pub struct ShapeVelocity(pub Vec2);

#[derive(Component, Default)]
pub struct ShapeContactCooldown(pub f32);

#[derive(Component)]
pub struct ShapeHealthBarBack;

#[derive(Component)]
pub struct ShapeHealthBarFill;

#[derive(Resource)]
pub struct Xp(pub u32);

#[derive(Resource)]
pub struct TotalXp(pub u32);

#[derive(Resource)]
pub struct Level(pub u32);

#[derive(Resource)]
pub struct SpawnTimer(pub f32);

#[derive(Resource, Clone)]
pub struct ShapeAssets {
    meshes: [Handle<Mesh>; 4],
    outlines: [Handle<Mesh>; 4],
    materials: [Handle<ColorMaterial>; 4],
    outline_material: Handle<ColorMaterial>,
    health_bar_back_mesh: Handle<Mesh>,
    health_bar_fill_mesh: Handle<Mesh>,
    health_bar_back_material: Handle<ColorMaterial>,
    health_bar_fill_material: Handle<ColorMaterial>,
}

pub fn setup_shape_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let shape_meshes = std::array::from_fn(|index| {
        meshes.add(RegularPolygon::new(
            constants::SHAPE_RADIUS,
            (index + 3) as u32,
        ))
    });
    let outlines = std::array::from_fn(|index| {
        meshes.add(RegularPolygon::new(
            constants::SHAPE_RADIUS + constants::OUTLINE_THICKNESS,
            (index + 3) as u32,
        ))
    });
    let tier_colors = [
        Color::srgb_u8(96, 211, 148),
        Color::srgb_u8(255, 209, 102),
        Color::srgb_u8(176, 124, 255),
        Color::srgb_u8(255, 107, 107),
    ];
    let shape_materials = tier_colors.map(|color| materials.add(color));
    commands.insert_resource(ShapeAssets {
        meshes: shape_meshes,
        outlines,
        materials: shape_materials,
        outline_material: materials.add(Color::srgba(
            constants::OUTLINE_COLOR[0],
            constants::OUTLINE_COLOR[1],
            constants::OUTLINE_COLOR[2],
            constants::OUTLINE_COLOR[3],
        )),
        health_bar_back_mesh: meshes.add(Rectangle::new(
            constants::SHAPE_HEALTH_BAR_WIDTH,
            constants::SHAPE_HEALTH_BAR_HEIGHT,
        )),
        health_bar_fill_mesh: meshes.add(Rectangle::new(
            constants::SHAPE_HEALTH_BAR_WIDTH,
            constants::SHAPE_HEALTH_BAR_HEIGHT,
        )),
        health_bar_back_material: materials.add(Color::srgba(
            constants::HEALTH_BAR_BG_COLOR[0],
            constants::HEALTH_BAR_BG_COLOR[1],
            constants::HEALTH_BAR_BG_COLOR[2],
            constants::HEALTH_BAR_BG_COLOR[3],
        )),
        health_bar_fill_material: materials.add(Color::srgba(
            constants::HEALTH_BAR_FILL_COLOR[0],
            constants::HEALTH_BAR_FILL_COLOR[1],
            constants::HEALTH_BAR_FILL_COLOR[2],
            constants::HEALTH_BAR_FILL_COLOR[3],
        )),
    });
}

pub fn setup_xp(mut commands: Commands) {
    commands.insert_resource(Xp(0));
    commands.insert_resource(TotalXp(0));
    commands.insert_resource(Level(1));
    commands.insert_resource(SpawnTimer(0.0));
}

pub fn shape_spawn(
    mut commands: Commands,
    assets: Res<ShapeAssets>,
    time: Res<Time>,
    mut rng: ResMut<Rng>,
    mut timer: ResMut<SpawnTimer>,
    shapes: Query<(), With<Shape>>,
    player: Query<(&Transform, &PlayerHealth), With<Player>>,
    bots: Query<(&Transform, &EnemyBotHealth), With<EnemyBot>>,
) {
    timer.0 -= time.delta_secs();
    if timer.0 > 0.0 {
        return;
    }
    let living_tanks = usize::from(
        player
            .single()
            .is_ok_and(|(_, health)| health.current > 0.0),
    ) + bots
        .iter()
        .filter(|(_, health)| health.current > 0.0)
        .count();
    let target = (living_tanks * constants::SHAPES_PER_LIVING_TANK).min(constants::SHAPE_MAX_COUNT);
    if shapes.iter().count() >= target {
        return;
    }
    timer.0 = constants::SHAPE_SPAWN_INTERVAL;

    let player_pos = player
        .single()
        .map_or(Vec2::ZERO, |(transform, _)| transform.translation.xy());
    let spawn_center = random_spawn_center(player_pos, &bots, &mut rng);
    let spawn_pos = random_spawn_position(spawn_center, &mut rng);

    let roll = rng.next(100);
    let sides = shape_sides_for_roll(roll);
    let hp = constants::shape_health(sides);
    let xp = constants::shape_xp(sides);
    let damage = constants::shape_damage(sides);

    let asset_index = (sides - 3) as usize;

    commands
        .spawn((
            Shape,
            Health(hp),
            MaxHealth(hp),
            XpValue(xp),
            ShapeDamage(damage),
            ShapeKind { sides },
            ShapeVelocity::default(),
            ShapeContactCooldown::default(),
            Mesh2d(assets.meshes[asset_index].clone()),
            MeshMaterial2d(assets.materials[asset_index].clone()),
            Transform::from_xyz(spawn_pos.x, spawn_pos.y, 0.0),
        ))
        .with_children(|shape| {
            shape.spawn((
                Mesh2d(assets.outlines[asset_index].clone()),
                MeshMaterial2d(assets.outline_material.clone()),
                Transform::from_xyz(0.0, 0.0, -0.2),
            ));
            shape.spawn((
                ShapeHealthBarBack,
                Mesh2d(assets.health_bar_back_mesh.clone()),
                MeshMaterial2d(assets.health_bar_back_material.clone()),
                Transform::from_xyz(0.0, constants::SHAPE_HEALTH_BAR_OFFSET_Y, 2.0),
                Visibility::Hidden,
            ));
            shape.spawn((
                ShapeHealthBarFill,
                Mesh2d(assets.health_bar_fill_mesh.clone()),
                MeshMaterial2d(assets.health_bar_fill_material.clone()),
                Transform::from_xyz(0.0, constants::SHAPE_HEALTH_BAR_OFFSET_Y, 3.0),
                Visibility::Hidden,
            ));
        });
}

fn random_spawn_position(player_pos: Vec2, rng: &mut Rng) -> Vec2 {
    let half = constants::arena_half_extent() - constants::SHAPE_RADIUS;
    let safe_radius_sq = constants::SHAPE_SPAWN_SAFE_RADIUS * constants::SHAPE_SPAWN_SAFE_RADIUS;

    for _ in 0..16 {
        let x = player_pos.x + rng_offset(rng, constants::WINDOW_WIDTH);
        let y = player_pos.y + rng_offset(rng, constants::WINDOW_HEIGHT);
        let candidate = Vec2::new(x.clamp(-half, half), y.clamp(-half, half));
        if candidate.distance_squared(player_pos) >= safe_radius_sq {
            return candidate;
        }
    }

    let directions = [
        Vec2::new(1.0, 0.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, -1.0),
        Vec2::new(1.0, 1.0).normalize(),
        Vec2::new(-1.0, 1.0).normalize(),
        Vec2::new(1.0, -1.0).normalize(),
        Vec2::new(-1.0, -1.0).normalize(),
    ];

    for distance in [
        constants::SHAPE_SPAWN_SAFE_RADIUS,
        constants::SHAPE_SPAWN_SAFE_RADIUS * 1.5,
        constants::SHAPE_SPAWN_SAFE_RADIUS * 2.0,
    ] {
        for direction in directions {
            let candidate = player_pos + direction * distance;
            let candidate = Vec2::new(
                candidate.x.clamp(-half, half),
                candidate.y.clamp(-half, half),
            );
            if candidate.distance_squared(player_pos) >= safe_radius_sq {
                return candidate;
            }
        }
    }

    Vec2::new(
        player_pos.x.clamp(-half, half),
        player_pos.y.clamp(-half, half),
    )
}

fn random_spawn_center(
    player_pos: Vec2,
    bots: &Query<(&Transform, &EnemyBotHealth), With<EnemyBot>>,
    rng: &mut Rng,
) -> Vec2 {
    let live_bot_count = bots
        .iter()
        .filter(|(_, health)| health.current > 0.0)
        .count();
    let center_index = rng.next((live_bot_count + 1) as u32) as usize;
    if center_index == 0 {
        return player_pos;
    }

    bots.iter()
        .filter(|(_, health)| health.current > 0.0)
        .nth(center_index - 1)
        .map(|(transform, _)| transform.translation.xy())
        .unwrap_or(player_pos)
}

fn rng_offset(rng: &mut Rng, range: f32) -> f32 {
    rng.next(range as u32) as f32 - range / 2.0
}

pub fn check_level_up(
    mut xp: ResMut<Xp>,
    mut level: ResMut<Level>,
    mut upgrades: ResMut<UpgradeState>,
) {
    let gained = constants::consume_level_ups(&mut xp.0, &mut level.0);
    upgrades.add_points(gained);
}

fn shape_sides_for_roll(roll: u32) -> u32 {
    match roll {
        0..=44 => 3,
        45..=74 => 4,
        75..=94 => 5,
        _ => 6,
    }
}

pub fn shape_knockback_update(
    time: Res<Time>,
    mut shapes: Query<
        (
            &mut Transform,
            &mut ShapeVelocity,
            &mut ShapeContactCooldown,
        ),
        With<Shape>,
    >,
) {
    let dt = time.delta_secs();
    let half = constants::arena_half_extent() - constants::SHAPE_RADIUS;
    let damping = (1.0 - constants::SHAPE_KNOCKBACK_DAMPING * dt).clamp(0.0, 1.0);

    for (mut transform, mut velocity, mut contact_cooldown) in shapes.iter_mut() {
        transform.translation += velocity.0.extend(0.0) * dt;
        transform.translation.x = transform.translation.x.clamp(-half, half);
        transform.translation.y = transform.translation.y.clamp(-half, half);
        velocity.0 *= damping;
        contact_cooldown.0 = (contact_cooldown.0 - dt).max(0.0);
    }
}

pub fn update_shape_health_bars(
    shapes: Query<(&Health, &MaxHealth, &Children), With<Shape>>,
    mut bars: Query<
        (
            &mut Transform,
            &mut Visibility,
            Option<&ShapeHealthBarBack>,
            Option<&ShapeHealthBarFill>,
        ),
        Or<(With<ShapeHealthBarBack>, With<ShapeHealthBarFill>)>,
    >,
) {
    for (health, max_health, children) in shapes.iter() {
        let is_damaged = health.0 < max_health.0;
        let visibility = if is_damaged {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let health_fraction = (health.0 / max_health.0).clamp(0.0, 1.0);

        for child in children.iter() {
            let Ok((mut transform, mut bar_visibility, back, fill)) = bars.get_mut(child) else {
                continue;
            };

            *bar_visibility = visibility;
            if back.is_some() {
                transform.translation = Vec3::new(0.0, constants::SHAPE_HEALTH_BAR_OFFSET_Y, 2.0);
                transform.scale.x = 1.0;
            } else if fill.is_some() {
                transform.translation = Vec3::new(
                    -constants::SHAPE_HEALTH_BAR_WIDTH * (1.0 - health_fraction) / 2.0,
                    constants::SHAPE_HEALTH_BAR_OFFSET_Y,
                    3.0,
                );
                transform.scale.x = health_fraction;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_position_stays_near_player_at_arena_edges() {
        let shape_half = constants::arena_half_extent() - constants::SHAPE_RADIUS;
        let safe_radius_sq =
            constants::SHAPE_SPAWN_SAFE_RADIUS * constants::SHAPE_SPAWN_SAFE_RADIUS;
        let player_half = constants::arena_half_extent() - constants::PLAYER_RADIUS;
        let player_positions = [
            Vec2::new(-player_half, player_half),
            Vec2::new(player_half, player_half),
            Vec2::new(-player_half, -player_half),
            Vec2::new(player_half, -player_half),
        ];

        for player_pos in player_positions {
            let mut rng = Rng::new(12345);
            for _ in 0..20 {
                let spawn_pos = random_spawn_position(player_pos, &mut rng);

                assert!(spawn_pos.x >= -shape_half && spawn_pos.x <= shape_half);
                assert!(spawn_pos.y >= -shape_half && spawn_pos.y <= shape_half);
                assert!(spawn_pos.distance_squared(player_pos) >= safe_radius_sq);
                assert!(
                    spawn_pos.distance_squared(player_pos)
                        <= constants::WINDOW_WIDTH * constants::WINDOW_WIDTH
                );
            }
        }
    }

    #[test]
    fn higher_tier_shapes_do_more_collision_damage() {
        assert!(constants::shape_damage(4) > constants::shape_damage(3));
        assert!(constants::shape_damage(5) > constants::shape_damage(4));
        assert!(constants::shape_damage(6) > constants::shape_damage(5));
    }

    #[test]
    fn shape_health_uses_requested_tier_scaling() {
        assert_eq!(constants::shape_health(3), 4.0);
        assert_eq!(constants::shape_health(4), 8.0);
        assert_eq!(constants::shape_health(5), 12.0);
        assert_eq!(constants::shape_health(6), 24.0);
    }

    #[test]
    fn shape_xp_is_independent_from_durability() {
        assert_eq!(constants::shape_xp(3), 20);
        assert_eq!(constants::shape_xp(4), 30);
        assert_eq!(constants::shape_xp(5), 40);
        assert_eq!(constants::shape_xp(6), 100);
    }

    #[test]
    fn base_projectiles_have_distinct_shape_breakpoints() {
        let shots = |sides| {
            (constants::shape_health(sides) / constants::BASE_PROJECTILE_DAMAGE).ceil() as u32
        };

        assert_eq!([shots(3), shots(4), shots(5), shots(6)], [2, 3, 4, 8]);
    }

    #[test]
    fn tier_roll_boundaries_match_spawn_distribution() {
        assert_eq!(shape_sides_for_roll(44), 3);
        assert_eq!(shape_sides_for_roll(45), 4);
        assert_eq!(shape_sides_for_roll(75), 5);
        assert_eq!(shape_sides_for_roll(95), 6);
    }
}
