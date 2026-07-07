use bevy::prelude::*;
use crate::{constants, projectile::ShootCooldown};

#[derive(Component)]
pub struct Player;

#[derive(Component, Default)]
pub struct Velocity(pub Vec2);

pub fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let player_id = commands.spawn((
        Player,
        Velocity::default(),
        ShootCooldown(0.0),
        Mesh2d(meshes.add(Circle::new(constants::PLAYER_RADIUS))),
        MeshMaterial2d(materials.add(Color::srgba(
            constants::PLAYER_COLOR[0],
            constants::PLAYER_COLOR[1],
            constants::PLAYER_COLOR[2],
            constants::PLAYER_COLOR[3],
        ))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    )).id();

    // Turret barrel (rectangle indicating aim direction)
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(4.0, constants::PLAYER_RADIUS + 10.0))),
        MeshMaterial2d(materials.add(Color::srgba(
            constants::PLAYER_COLOR[0],
            constants::PLAYER_COLOR[1],
            constants::PLAYER_COLOR[2],
            constants::PLAYER_COLOR[3],
        ))),
        Transform::from_xyz(0.0, constants::PLAYER_RADIUS / 2.0 + 5.0, 0.0),
    )).set_parent_in_place(player_id);
}

pub fn player_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Velocity), With<Player>>,
) {
    let Ok((mut transform, mut velocity)) = query.single_mut() else { return };

    let mut direction = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) { direction.y += 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { direction.y -= 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { direction.x += 1.0; }

    let direction = direction.normalize_or_zero();
    velocity.0 = direction * constants::PLAYER_SPEED;
    transform.translation += direction.extend(0.0) * constants::PLAYER_SPEED * time.delta_secs();
}

pub fn player_aim(
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut transform) = query.single_mut() else { return };

    let Some(cursor) = window.cursor_position() else { return };
    let Ok(world_pos) = camera.0.viewport_to_world_2d(camera.1, cursor) else { return };

    let delta = world_pos - transform.translation.xy();
    if delta.length_squared() > 0.001 {
        transform.rotation = Quat::from_rotation_z(delta.y.atan2(delta.x) - std::f32::consts::FRAC_PI_2);
    }
}
