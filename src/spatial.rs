use crate::constants;
use crate::{
    enemy_bot::{EnemyBot, EnemyBotHealth},
    player::{Player, PlayerHealth},
    projectile::Projectile,
    shape::{Health, Shape},
};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialKind {
    Tank,
    Shape,
    Projectile,
}

#[derive(Clone, Copy, Debug)]
pub struct SpatialEntry {
    pub entity: Entity,
    pub position: Vec2,
    pub kind: SpatialKind,
}

#[derive(Resource, Default)]
pub struct SpatialGrid {
    cells: HashMap<IVec2, Vec<SpatialEntry>>,
}

impl SpatialGrid {
    pub fn clear(&mut self) {
        for entries in self.cells.values_mut() {
            entries.clear();
        }
    }
    pub fn insert(&mut self, entry: SpatialEntry) {
        self.cells
            .entry(cell(entry.position))
            .or_default()
            .push(entry);
    }
    pub fn finish(&mut self) {
        for entries in self.cells.values_mut() {
            entries.sort_by_key(|entry| entry.entity.to_bits());
        }
    }
    #[cfg(test)]
    pub fn nearby(&self, position: Vec2, radius: f32) -> Vec<SpatialEntry> {
        let mut result = Vec::new();
        self.nearby_into(position, radius, &mut result);
        result
    }
    pub fn nearby_into(&self, position: Vec2, radius: f32, result: &mut Vec<SpatialEntry>) {
        self.nearby_filtered_into(position, radius, None, result);
    }
    pub fn nearby_kind_into(
        &self,
        position: Vec2,
        radius: f32,
        kind: SpatialKind,
        result: &mut Vec<SpatialEntry>,
    ) {
        self.nearby_filtered_into(position, radius, Some(kind), result);
    }
    fn nearby_filtered_into(
        &self,
        position: Vec2,
        radius: f32,
        kind: Option<SpatialKind>,
        result: &mut Vec<SpatialEntry>,
    ) {
        let min = cell(position - Vec2::splat(radius));
        let max = cell(position + Vec2::splat(radius));
        let radius_sq = radius * radius;
        result.clear();
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                if let Some(entries) = self.cells.get(&IVec2::new(x, y)) {
                    result.extend(entries.iter().copied().filter(|entry| {
                        kind.is_none_or(|kind| entry.kind == kind)
                            && entry.position.distance_squared(position) <= radius_sq
                    }));
                }
            }
        }
        result.sort_by_key(|entry| entry.entity.to_bits());
    }
    #[cfg(test)]
    pub fn unique_pairs(&self, radius: f32) -> Vec<(Entity, Entity)> {
        self.unique_pairs_matching(radius, None)
    }
    #[cfg(test)]
    pub fn unique_pairs_of_kind(&self, radius: f32, kind: SpatialKind) -> Vec<(Entity, Entity)> {
        self.unique_pairs_matching(radius, Some(kind))
    }
    pub fn unique_pairs_of_kind_into(
        &self,
        radius: f32,
        kind: SpatialKind,
        pairs: &mut Vec<(Entity, Entity)>,
    ) {
        self.unique_pairs_matching_into(radius, Some(kind), pairs);
    }
    #[cfg(test)]
    fn unique_pairs_matching(
        &self,
        radius: f32,
        kind: Option<SpatialKind>,
    ) -> Vec<(Entity, Entity)> {
        let mut pairs = Vec::new();
        self.unique_pairs_matching_into(radius, kind, &mut pairs);
        pairs
    }
    fn unique_pairs_matching_into(
        &self,
        radius: f32,
        kind: Option<SpatialKind>,
        pairs: &mut Vec<(Entity, Entity)>,
    ) {
        pairs.clear();
        let radius_sq = radius * radius;
        for entries in self.cells.values() {
            for entry in entries {
                if kind.is_some_and(|kind| entry.kind != kind) {
                    continue;
                }
                let min = cell(entry.position - Vec2::splat(radius));
                let max = cell(entry.position + Vec2::splat(radius));
                for y in min.y..=max.y {
                    for x in min.x..=max.x {
                        let Some(others) = self.cells.get(&IVec2::new(x, y)) else {
                            continue;
                        };
                        for other in others {
                            if entry.entity.to_bits() >= other.entity.to_bits()
                                || kind.is_some_and(|kind| other.kind != kind)
                                || entry.position.distance_squared(other.position) > radius_sq
                            {
                                continue;
                            }
                            pairs.push((entry.entity, other.entity));
                        }
                    }
                }
            }
        }
        pairs.sort_by_key(|(a, b)| (a.to_bits(), b.to_bits()));
    }
}

fn cell(position: Vec2) -> IVec2 {
    (position / constants::SPATIAL_CELL_SIZE).floor().as_ivec2()
}

pub fn rebuild_spatial_grid(
    mut grid: ResMut<SpatialGrid>,
    player: Query<(Entity, &Transform, &PlayerHealth), With<Player>>,
    bots: Query<(Entity, &Transform, &EnemyBotHealth), With<EnemyBot>>,
    shapes: Query<(Entity, &Transform, &Health), With<Shape>>,
    projectiles: Query<(Entity, &Transform), With<Projectile>>,
) {
    grid.clear();
    if let Ok((entity, transform, health)) = player.single()
        && health.current > 0.0
    {
        grid.insert(SpatialEntry {
            entity,
            position: transform.translation.xy(),
            kind: SpatialKind::Tank,
        });
    }
    for (entity, transform, health) in &bots {
        if health.current > 0.0 {
            grid.insert(SpatialEntry {
                entity,
                position: transform.translation.xy(),
                kind: SpatialKind::Tank,
            });
        }
    }
    for (entity, transform, health) in &shapes {
        if health.0 > 0.0 {
            grid.insert(SpatialEntry {
                entity,
                position: transform.translation.xy(),
                kind: SpatialKind::Shape,
            });
        }
    }
    for (entity, transform) in &projectiles {
        grid.insert(SpatialEntry {
            entity,
            position: transform.translation.xy(),
            kind: SpatialKind::Projectile,
        });
    }
    grid.finish();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn mutate_profile_during_stress(mut profile: ResMut<crate::profile::Profile>) {
        profile.data.records.shapes_destroyed =
            profile.data.records.shapes_destroyed.wrapping_add(1);
        profile.mark_dirty();
    }
    #[test]
    fn neighbor_queries_and_pairs_are_unique() {
        let mut grid = SpatialGrid::default();
        let a = Entity::from_bits(1);
        let b = Entity::from_bits(2);
        let c = Entity::from_bits(3);
        for (entity, position) in [
            (a, Vec2::ZERO),
            (b, Vec2::new(50.0, 0.0)),
            (c, Vec2::new(500.0, 0.0)),
        ] {
            grid.insert(SpatialEntry {
                entity,
                position,
                kind: SpatialKind::Tank,
            });
        }
        grid.finish();
        assert_eq!(grid.nearby(Vec2::ZERO, 100.0).len(), 2);
        assert_eq!(grid.unique_pairs(100.0), vec![(a, b)]);
    }

    #[test]
    fn typed_pairs_do_not_include_mixed_spatial_kinds() {
        let mut grid = SpatialGrid::default();
        let tank_a = Entity::from_bits(10);
        let tank_b = Entity::from_bits(11);
        let shape = Entity::from_bits(12);
        for (entity, kind) in [
            (tank_a, SpatialKind::Tank),
            (tank_b, SpatialKind::Tank),
            (shape, SpatialKind::Shape),
        ] {
            grid.insert(SpatialEntry {
                entity,
                position: Vec2::ZERO,
                kind,
            });
        }
        grid.finish();
        assert_eq!(
            grid.unique_pairs_of_kind(100.0, SpatialKind::Tank),
            vec![(tank_a, tank_b)]
        );
        assert!(
            grid.unique_pairs_of_kind(100.0, SpatialKind::Shape)
                .is_empty()
        );
    }

    #[test]
    fn clear_and_queries_reuse_allocated_capacity() {
        let mut grid = SpatialGrid::default();
        let cell_position = Vec2::new(10.0, 10.0);
        for index in 0..32 {
            grid.insert(SpatialEntry {
                entity: Entity::from_bits(index + 1),
                position: cell_position,
                kind: SpatialKind::Tank,
            });
        }
        let cell_key = cell(cell_position);
        let capacity = grid.cells[&cell_key].capacity();
        grid.clear();
        assert_eq!(grid.cells[&cell_key].capacity(), capacity);
        assert!(grid.cells[&cell_key].is_empty());

        let mut pairs = Vec::with_capacity(64);
        let pair_capacity = pairs.capacity();
        grid.unique_pairs_of_kind_into(100.0, SpatialKind::Tank, &mut pairs);
        assert_eq!(pairs.capacity(), pair_capacity);
    }

    #[test]
    #[ignore = "run with cargo test --release -- --ignored stress_harness"]
    fn stress_harness_capstones_hotspot_constructs_and_1000_projectiles() {
        const WARMUP: usize = 60;
        const MEASURED: usize = 600;
        let half = constants::arena_half_extent();
        let tanks = (0..20)
            .map(|i| Vec2::new(i as f32 * 37.0 - 350.0, i as f32 * 19.0 - 180.0))
            .collect::<Vec<_>>();
        let shapes = (0..120)
            .map(|i| {
                Vec2::new(
                    (i % 10) as f32 * 140.0 - 630.0,
                    (i / 10) as f32 * 140.0 - 630.0,
                )
            })
            .collect::<Vec<_>>();
        let capstones = [
            crate::evolution::EvolutionKind::Sentry,
            crate::evolution::EvolutionKind::Fusillade,
            crate::evolution::EvolutionKind::Bombardier,
            crate::evolution::EvolutionKind::Guardian,
            crate::evolution::EvolutionKind::Afterburner,
            crate::evolution::EvolutionKind::Ace,
        ];
        assert!(capstones.into_iter().all(|kind| kind.is_capstone()));
        assert_eq!(
            crate::evolution::EvolutionState {
                current_kind: crate::evolution::EvolutionKind::Fusillade,
                ..default()
            }
            .barrel_specs()
            .len(),
            8
        );
        let constructs = [
            Vec2::new(-180.0, 40.0),
            Vec2::new(120.0, -80.0),
            Vec2::new(260.0, 160.0),
            Vec2::new(-300.0, -210.0),
        ];
        let mut projectiles = (0..1_000)
            .map(|i| {
                let x = (i % 40) as f32 * 90.0 - 1_755.0;
                let y = (i / 40) as f32 * 90.0 - 1_080.0;
                (
                    Vec2::new(x, y),
                    Vec2::new(70.0 + (i % 7) as f32, 35.0 - (i % 5) as f32),
                )
            })
            .collect::<Vec<_>>();
        let mut samples = Vec::with_capacity(MEASURED);
        let mut scratch = Vec::new();
        let mut pair_scratch = Vec::new();
        let mut grid = SpatialGrid::default();

        for step in 0..(WARMUP + MEASURED) {
            let started = Instant::now();
            grid.clear();
            for (index, position) in tanks.iter().enumerate() {
                grid.insert(SpatialEntry {
                    entity: Entity::from_bits(index as u64 + 1),
                    position: *position,
                    kind: SpatialKind::Tank,
                });
            }
            for (index, position) in shapes.iter().enumerate() {
                grid.insert(SpatialEntry {
                    entity: Entity::from_bits(index as u64 + 101),
                    position: *position,
                    kind: SpatialKind::Shape,
                });
            }
            for (index, (position, velocity)) in projectiles.iter_mut().enumerate() {
                *position += *velocity / 60.0;
                if position.x.abs() > half {
                    position.x = -position.x.signum() * half;
                }
                if position.y.abs() > half {
                    position.y = -position.y.signum() * half;
                }
                assert!(position.is_finite());
                assert!(position.x.abs() <= half && position.y.abs() <= half);
                grid.insert(SpatialEntry {
                    entity: Entity::from_bits(index as u64 + 1_001),
                    position: *position,
                    kind: SpatialKind::Projectile,
                });
            }
            grid.finish();
            for tank in &tanks {
                grid.nearby_into(*tank, 1_000.0, &mut scratch);
                std::hint::black_box(scratch.len());
            }
            for construct in constructs {
                grid.nearby_kind_into(construct, 550.0, SpatialKind::Tank, &mut scratch);
                std::hint::black_box(scratch.len());
                grid.nearby_kind_into(construct, 150.0, SpatialKind::Projectile, &mut scratch);
                std::hint::black_box(scratch.len());
            }
            for (position, _) in &projectiles {
                grid.nearby_kind_into(
                    *position,
                    constants::PROJECTILE_RADIUS + constants::SHAPE_RADIUS,
                    SpatialKind::Shape,
                    &mut scratch,
                );
                std::hint::black_box(scratch.len());
                grid.nearby_kind_into(
                    *position,
                    constants::PROJECTILE_RADIUS + 25.0,
                    SpatialKind::Tank,
                    &mut scratch,
                );
                std::hint::black_box(scratch.len());
            }
            grid.unique_pairs_of_kind_into(150.0, SpatialKind::Tank, &mut pair_scratch);
            std::hint::black_box(pair_scratch.len());
            grid.unique_pairs_of_kind_into(
                constants::SHAPE_RADIUS * 2.0,
                SpatialKind::Shape,
                &mut pair_scratch,
            );
            std::hint::black_box(pair_scratch.len());
            if step >= WARMUP {
                samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            }
        }

        samples.sort_by(f64::total_cmp);
        let average = samples.iter().sum::<f64>() / samples.len() as f64;
        let p95 = samples[(samples.len() as f32 * 0.95) as usize];
        eprintln!("stress average={average:.3}ms p95={p95:.3}ms");
        assert!(p95 < 15.625, "p95 {p95:.3}ms exceeded fixed-step budget");

        let mut history = crate::projectile::ProjectileHitHistory::default();
        let target = Entity::from_bits(7_777);
        assert!(history.record(target));
        assert!(!history.record(target));
    }

    #[test]
    #[ignore = "run with cargo test --release -- --ignored stress_harness"]
    fn release_stress_harness_runs_fixed_schedule_and_cleanup() {
        const WARMUP: usize = 60;
        const MEASURED: usize = 360;
        let root =
            std::env::temp_dir().join(format!("polycore-fixed-stress-{}", std::process::id()));
        let profile_path = root.join("profile.json");
        let _ = std::fs::remove_dir_all(&root);

        let mut world = World::new();
        let mut fixed_clock = Time::<()>::default();
        fixed_clock.advance_by(Duration::from_micros(15_625));
        world.insert_resource(fixed_clock);
        world.insert_resource(SpatialGrid::default());
        world.insert_resource(crate::menu::GamePhase::Playing);
        world.insert_resource(crate::profile::Profile::test_with_path(Some(
            profile_path.clone(),
        )));
        world.insert_resource(crate::projectile::ProjectileAssets {
            mesh: Handle::default(),
        });
        world.insert_resource(crate::evolution::EvolutionState {
            current_kind: crate::evolution::EvolutionKind::Sentry,
            ..default()
        });

        world.spawn((
            Player,
            PlayerHealth {
                current: 100.0,
                max: 100.0,
            },
            crate::combat::LifeGeneration(1),
            Transform::from_xyz(-200.0, 0.0, 0.0),
        ));
        let capstones = [
            crate::evolution::EvolutionKind::Emplacement,
            crate::evolution::EvolutionKind::Siegebreaker,
            crate::evolution::EvolutionKind::Lancer,
            crate::evolution::EvolutionKind::Fusillade,
            crate::evolution::EvolutionKind::Guardian,
        ];
        for (index, capstone) in capstones.into_iter().enumerate() {
            world.spawn((
                EnemyBot,
                EnemyBotHealth {
                    current: 100.0,
                    max: 100.0,
                },
                crate::combat::LifeGeneration(1),
                crate::enemy_bot::EnemyBotEvolution(crate::evolution::EvolutionState {
                    current_kind: capstone,
                    ..default()
                }),
                Transform::from_xyz(index as f32 * 80.0 - 120.0, 140.0, 0.0),
            ));
        }
        for index in 0..120 {
            world.spawn((
                Shape,
                Health(100.0),
                Transform::from_xyz(
                    (index % 12) as f32 * 110.0 - 605.0,
                    (index / 12) as f32 * 110.0 - 495.0,
                    0.0,
                ),
            ));
        }
        for index in 0..1_000 {
            world.spawn((
                Projectile,
                crate::projectile::Lifetime(30.0),
                crate::projectile::ProjectileTravel::default(),
                crate::projectile::ProjectileRadius(constants::PROJECTILE_RADIUS),
                crate::player::Velocity(Vec2::new(
                    22.0 + (index % 7) as f32,
                    11.0 - (index % 5) as f32,
                )),
                crate::ability::ProjectileAbility {
                    reflected: index % 3 == 0,
                    ..default()
                },
                Transform::from_xyz(
                    (index % 40) as f32 * 70.0 - 1_365.0,
                    (index / 40) as f32 * 70.0 - 840.0,
                    0.0,
                ),
            ));
        }
        for index in 0..4 {
            let child = world
                .spawn((crate::ability::ConstructHealthFill, Transform::default()))
                .id();
            let construct = world
                .spawn((
                    crate::ability::Construct {
                        kind: crate::ability::ConstructKind::Fortification,
                        owner: crate::projectile::ProjectileOwner::Player,
                        generation: 1,
                        health: 150.0,
                        max_health: 150.0,
                        remaining: 30.0,
                        duration: 30.0,
                        damage: 0.0,
                        projectile_speed: 0.0,
                        range: 0.0,
                        fire_timer: 0.0,
                    },
                    Transform::from_xyz(index as f32 * 180.0 - 270.0, -220.0, 0.0),
                ))
                .id();
            world.entity_mut(construct).add_child(child);
        }

        let mut fixed_schedule = Schedule::new(FixedUpdate);
        fixed_schedule.add_systems(
            (
                crate::ability::update_constructs,
                crate::projectile::projectile_update,
                rebuild_spatial_grid,
                mutate_profile_during_stress,
                crate::profile::flush_profile,
            )
                .chain(),
        );
        let mut samples = Vec::with_capacity(MEASURED);
        for step in 0..(WARMUP + MEASURED) {
            let started = Instant::now();
            fixed_schedule.run(&mut world);
            if step >= WARMUP {
                samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            }
        }
        samples.sort_by(f64::total_cmp);
        let p95 = samples[((samples.len() - 1) as f32 * 0.95).round() as usize];
        eprintln!("fixed-schedule stress p95={p95:.3}ms");
        assert!(p95 < 15.625, "p95 {p95:.3}ms exceeded fixed-step budget");
        assert!(
            !profile_path.exists(),
            "profile was written during active play"
        );

        let mut projectile_lifetimes =
            world.query_filtered::<&mut crate::projectile::Lifetime, With<Projectile>>();
        for mut lifetime in projectile_lifetimes.iter_mut(&mut world) {
            lifetime.0 = 0.0;
        }
        let mut constructs = world.query::<&mut crate::ability::Construct>();
        for mut construct in constructs.iter_mut(&mut world) {
            construct.remaining = 0.0;
        }
        fixed_schedule.run(&mut world);
        assert_eq!(
            world
                .query_filtered::<Entity, With<Projectile>>()
                .iter(&world)
                .count(),
            0
        );
        assert_eq!(
            world
                .query_filtered::<Entity, With<crate::ability::Construct>>()
                .iter(&world)
                .count(),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
