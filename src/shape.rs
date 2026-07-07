use bevy::prelude::*;
use crate::{constants, rng::Rng};

#[derive(Component)]
pub struct Shape;

#[derive(Component)]
pub struct Health(pub u32);

#[derive(Component)]
pub struct XpValue(pub u32);

#[derive(Resource)]
pub struct Xp(pub u32);

#[derive(Resource)]
pub struct Level(pub u32);

#[derive(Resource)]
pub struct SpawnTimer(pub f32);

pub fn setup_xp(mut commands: Commands) {
    commands.insert_resource(Xp(0));
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
) {
    timer.0 -= time.delta_secs();
    if timer.0 > 0.0 {
        return;
    }
    if shapes.iter().count() >= constants::SHAPE_MAX_COUNT {
        return;
    }
    timer.0 = constants::SHAPE_SPAWN_INTERVAL;

    let x = (rng.next(constants::WINDOW_WIDTH as u32) as f32) - constants::WINDOW_WIDTH / 2.0;
    let y = (rng.next(constants::WINDOW_HEIGHT as u32) as f32) - constants::WINDOW_HEIGHT / 2.0;

    let sides = 3 + rng.next(4) as u32;
    let hp = constants::shape_health(sides);
    let xp = constants::shape_xp(sides);

    // Darker shade for higher-HP shapes
    let t = (sides - 3) as f32 / 3.0; // 0.0 .. 1.0
    let r = constants::ENEMY_COLOR[0] * (1.0 - t * 0.5);
    let g = constants::ENEMY_COLOR[1] * (1.0 - t * 0.5);
    let b = constants::ENEMY_COLOR[2] * (1.0 - t * 0.5);

    commands.spawn((
        Shape,
        Health(hp),
        XpValue(xp),
        Mesh2d(meshes.add(RegularPolygon::new(constants::SHAPE_RADIUS, sides))),
        MeshMaterial2d(materials.add(Color::srgba(r, g, b, 1.0))),
        Transform::from_xyz(x, y, 0.0),
    ));
}

pub fn check_level_up(mut xp: ResMut<Xp>, mut level: ResMut<Level>) {
    while xp.0 >= constants::XP_PER_LEVEL {
        xp.0 -= constants::XP_PER_LEVEL;
        level.0 += 1;
    }
}
