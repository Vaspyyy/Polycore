use bevy::prelude::*;
use crate::{constants, projectile::Projectile, shape::{Shape, Health, XpValue, Xp}};

pub fn check_collisions(
    mut commands: Commands,
    projectiles: Query<(Entity, &Transform), With<Projectile>>,
    mut shapes: Query<(Entity, &Transform, &mut Health, &XpValue), With<Shape>>,
    mut xp: ResMut<Xp>,
) {
    let proj_data: Vec<(Entity, Vec2)> = projectiles.iter()
        .map(|(e, t)| (e, t.translation.xy()))
        .collect();

    let collision_dist = constants::PROJECTILE_RADIUS + constants::SHAPE_RADIUS;
    let collision_dist_sq = collision_dist * collision_dist;

    for (proj_entity, proj_pos) in &proj_data {
        for (shape_entity, shape_pos, mut health, xp_val) in shapes.iter_mut() {
            let dist_sq = proj_pos.distance_squared(shape_pos.translation.xy());
            if dist_sq < collision_dist_sq {
                commands.entity(*proj_entity).despawn();
                health.0 -= 1;
                if health.0 == 0 {
                    commands.entity(shape_entity).despawn();
                    xp.0 += xp_val.0;
                }
                break;
            }
        }
    }
}
