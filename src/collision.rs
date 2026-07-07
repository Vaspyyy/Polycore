use bevy::prelude::*;
use crate::{constants, projectile::Projectile, shape::{Shape, XpValue, Xp}};

pub fn check_collisions(
    mut commands: Commands,
    projectiles: Query<(Entity, &Transform), With<Projectile>>,
    shapes: Query<(Entity, &Transform, &XpValue), With<Shape>>,
    mut xp: ResMut<Xp>,
) {
    let proj_data: Vec<(Entity, Vec2)> = projectiles.iter()
        .map(|(e, t)| (e, t.translation.xy()))
        .collect();

    let shape_data: Vec<(Entity, Vec2, u32)> = shapes.iter()
        .map(|(e, t, x)| (e, t.translation.xy(), x.0))
        .collect();

    let collision_dist = constants::PROJECTILE_RADIUS + constants::SHAPE_RADIUS;
    let collision_dist_sq = collision_dist * collision_dist;

    for (proj_entity, proj_pos) in &proj_data {
        for (shape_entity, shape_pos, xp_val) in &shape_data {
            let dist_sq = proj_pos.distance_squared(*shape_pos);
            if dist_sq < collision_dist_sq {
                commands.entity(*proj_entity).despawn();
                commands.entity(*shape_entity).despawn();
                xp.0 += xp_val;
                break;
            }
        }
    }
}
