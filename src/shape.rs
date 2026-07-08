use crate::{constants, hud::UpgradeState, player::Player, rng::Rng};
use bevy::prelude::*;

#[derive(Component)]
pub struct Shape;

#[derive(Component)]
pub struct Health(pub u32);

#[derive(Component)]
pub struct MaxHealth(pub u32);

#[derive(Component)]
pub struct XpValue(pub u32);

#[derive(Component)]
pub struct ShapeDamage(pub u32);

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

pub fn setup_xp(mut commands: Commands) {
    commands.insert_resource(Xp(0));
    commands.insert_resource(TotalXp(0));
    commands.insert_resource(Level(1));
    commands.insert_resource(SpawnTimer(0.0));
}

pub fn shape_spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    mut rng: ResMut<Rng>,
    mut timer: ResMut<SpawnTimer>,
    shapes: Query<(), With<Shape>>,
    player: Query<&Transform, With<Player>>,
) {
    timer.0 -= time.delta_secs();
    if timer.0 > 0.0 {
        return;
    }
    if shapes.iter().count() >= constants::SHAPE_MAX_COUNT {
        return;
    }
    timer.0 = constants::SHAPE_SPAWN_INTERVAL;

    let Ok(player_transform) = player.single() else {
        return;
    };
    let spawn_pos = random_spawn_position(player_transform.translation.xy(), &mut rng);

    let sides = 3 + rng.next(4) as u32;
    let hp = constants::shape_health(sides);
    let xp = constants::shape_xp(sides);
    let damage = constants::shape_damage(sides);

    // Darker shade for higher-HP shapes
    let t = (sides - 3) as f32 / 3.0; // 0.0 .. 1.0
    let r = constants::ENEMY_COLOR[0] * (1.0 - t * 0.5);
    let g = constants::ENEMY_COLOR[1] * (1.0 - t * 0.5);
    let b = constants::ENEMY_COLOR[2] * (1.0 - t * 0.5);

    let health_bar_back_mesh = meshes.add(Rectangle::new(
        constants::SHAPE_HEALTH_BAR_WIDTH,
        constants::SHAPE_HEALTH_BAR_HEIGHT,
    ));
    let health_bar_fill_mesh = meshes.add(Rectangle::new(
        constants::SHAPE_HEALTH_BAR_WIDTH,
        constants::SHAPE_HEALTH_BAR_HEIGHT,
    ));
    let health_bar_back_material = materials.add(Color::srgba(
        constants::HEALTH_BAR_BG_COLOR[0],
        constants::HEALTH_BAR_BG_COLOR[1],
        constants::HEALTH_BAR_BG_COLOR[2],
        constants::HEALTH_BAR_BG_COLOR[3],
    ));
    let health_bar_fill_material = materials.add(Color::srgba(
        constants::HEALTH_BAR_FILL_COLOR[0],
        constants::HEALTH_BAR_FILL_COLOR[1],
        constants::HEALTH_BAR_FILL_COLOR[2],
        constants::HEALTH_BAR_FILL_COLOR[3],
    ));

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
            Mesh2d(meshes.add(RegularPolygon::new(constants::SHAPE_RADIUS, sides))),
            MeshMaterial2d(materials.add(Color::srgba(r, g, b, 1.0))),
            Transform::from_xyz(spawn_pos.x, spawn_pos.y, 0.0),
        ))
        .with_children(|shape| {
            shape.spawn((
                ShapeHealthBarBack,
                Mesh2d(health_bar_back_mesh),
                MeshMaterial2d(health_bar_back_material),
                Transform::from_xyz(0.0, constants::SHAPE_HEALTH_BAR_OFFSET_Y, 2.0),
                Visibility::Hidden,
            ));
            shape.spawn((
                ShapeHealthBarFill,
                Mesh2d(health_bar_fill_mesh),
                MeshMaterial2d(health_bar_fill_material),
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

fn rng_offset(rng: &mut Rng, range: f32) -> f32 {
    rng.next(range as u32) as f32 - range / 2.0
}

pub fn check_level_up(
    mut xp: ResMut<Xp>,
    mut level: ResMut<Level>,
    mut upgrades: ResMut<UpgradeState>,
) {
    while xp.0 >= constants::XP_PER_LEVEL {
        xp.0 -= constants::XP_PER_LEVEL;
        level.0 += 1;
        upgrades.add_points(1);
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
    mut bars: Query<(
        &mut Transform,
        &mut Visibility,
        Option<&ShapeHealthBarBack>,
        Option<&ShapeHealthBarFill>,
    )>,
) {
    for (health, max_health, children) in shapes.iter() {
        let is_damaged = health.0 < max_health.0;
        let visibility = if is_damaged {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let health_fraction = (health.0 as f32 / max_health.0 as f32).clamp(0.0, 1.0);

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
}
