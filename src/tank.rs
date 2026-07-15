use crate::{
    constants,
    evolution::{EvolutionKind, EvolutionState},
    rng::Rng,
};
use bevy::prelude::*;

#[derive(Component, Clone, Copy, Debug)]
pub struct SpawnProtection {
    pub remaining: f32,
}

impl Default for SpawnProtection {
    fn default() -> Self {
        Self {
            remaining: constants::SPAWN_PROTECTION_SECS,
        }
    }
}

impl SpawnProtection {
    pub fn active(self) -> bool {
        self.remaining > 0.0
    }
    pub fn cancel(&mut self) {
        self.remaining = 0.0;
    }
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct RecentDamage {
    pub amount: f32,
    pub remaining: f32,
    pub direction: Vec2,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct TankOutline {
    pub(crate) owner: Entity,
}

pub(crate) const BARREL_OUTLINE_Z_OFFSET: f32 = -0.4;
pub(crate) const BARREL_FILL_Z_OFFSET: f32 = -0.3;
pub(crate) const TANK_OUTLINE_Z_OFFSET: f32 = -0.2;

#[derive(Resource, Clone)]
pub struct TankBodyAssets {
    circles: [Handle<Mesh>; 6],
    ram: Handle<Mesh>,
    capstones: [Handle<Mesh>; 16],
    outlines: [Handle<Mesh>; 7],
    ram_outline: Handle<Mesh>,
    capstone_outlines: [Handle<Mesh>; 16],
    outline_material: Handle<ColorMaterial>,
    guard_material: Handle<ColorMaterial>,
}

pub fn setup_tank_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let radii = [20.0, 20.8, 18.4, 19.2, 19.0, 24.6];
    let circles = std::array::from_fn(|index| meshes.add(Circle::new(radii[index])));
    let outline_radii = [23.0, 23.8, 21.4, 22.2, 22.0, 27.6, 24.6];
    let outlines = std::array::from_fn(|index| meshes.add(Circle::new(outline_radii[index])));
    let capstone_radii = [
        20.0, 20.0, 20.8, 20.8, 20.0, 20.0, 18.4, 18.4, 27.5, 24.6, 19.2, 19.2, 24.6, 24.6, 19.0,
        19.0,
    ];
    let capstones = std::array::from_fn(|index| {
        let radius = capstone_radii[index];
        meshes.add(RegularPolygon::new(radius, 5 + (index % 4) as u32))
    });
    let capstone_outlines = std::array::from_fn(|index| {
        let radius = capstone_radii[index] + 3.0;
        meshes.add(RegularPolygon::new(radius, 5 + (index % 4) as u32))
    });
    commands.insert_resource(TankBodyAssets {
        circles,
        ram: meshes.add(RegularPolygon::new(24.6, 6)),
        capstones,
        outlines,
        ram_outline: meshes.add(RegularPolygon::new(27.6, 6)),
        capstone_outlines,
        outline_material: materials.add(Color::BLACK),
        guard_material: materials.add(Color::srgb(0.38, 0.22, 0.55)),
    });
}

impl TankBodyAssets {
    fn body(&self, kind: EvolutionKind) -> Handle<Mesh> {
        if let Some(index) = capstone_index(kind) {
            return self.capstones[index].clone();
        }
        match kind.base() {
            EvolutionKind::Cannon => self.circles[1].clone(),
            EvolutionKind::Sniper => self.circles[2].clone(),
            EvolutionKind::Sprayer => self.circles[3].clone(),
            EvolutionKind::Flanker => self.circles[4].clone(),
            EvolutionKind::RamCore => self.ram.clone(),
            EvolutionKind::Guard => self.circles[0].clone(),
            _ => self.circles[0].clone(),
        }
    }
    fn outline(&self, kind: EvolutionKind) -> Handle<Mesh> {
        if let Some(index) = capstone_index(kind) {
            return self.capstone_outlines[index].clone();
        }
        match kind.base() {
            EvolutionKind::Cannon => self.outlines[1].clone(),
            EvolutionKind::Sniper => self.outlines[2].clone(),
            EvolutionKind::Sprayer => self.outlines[3].clone(),
            EvolutionKind::Flanker => self.outlines[4].clone(),
            EvolutionKind::RamCore => self.ram_outline.clone(),
            EvolutionKind::Guard => self.outlines[6].clone(),
            _ => self.outlines[0].clone(),
        }
    }
}

fn capstone_index(kind: EvolutionKind) -> Option<usize> {
    Some(match kind {
        EvolutionKind::Sentry => 0,
        EvolutionKind::Emplacement => 1,
        EvolutionKind::Siegebreaker => 2,
        EvolutionKind::Lancer => 3,
        EvolutionKind::Fusillade => 4,
        EvolutionKind::Rearguard => 5,
        EvolutionKind::Deadeye => 6,
        EvolutionKind::Pursuer => 7,
        EvolutionKind::Dreadnought => 8,
        EvolutionKind::Vanguard => 9,
        EvolutionKind::Bombardier => 10,
        EvolutionKind::Impaler => 11,
        EvolutionKind::Stronghold => 12,
        EvolutionKind::Guardian => 13,
        EvolutionKind::Afterburner => 14,
        EvolutionKind::Ace => 15,
        _ => return None,
    })
}

pub fn update_tank_bodies(
    assets: Res<TankBodyAssets>,
    player_evolution: Res<EvolutionState>,
    mut players: Query<
        (&Transform, &Visibility, &mut Mesh2d),
        (
            With<crate::player::Player>,
            Without<crate::enemy_bot::EnemyBot>,
            Without<TankOutline>,
        ),
    >,
    mut bots: Query<
        (
            &crate::enemy_bot::EnemyBotEvolution,
            &Transform,
            &Visibility,
            &mut Mesh2d,
        ),
        (
            With<crate::enemy_bot::EnemyBot>,
            Without<crate::player::Player>,
            Without<TankOutline>,
        ),
    >,
    mut outlines: Query<
        (
            &TankOutline,
            &mut Mesh2d,
            &mut MeshMaterial2d<ColorMaterial>,
            &mut Transform,
            &mut Visibility,
        ),
        (
            Without<crate::player::Player>,
            Without<crate::enemy_bot::EnemyBot>,
        ),
    >,
) {
    for (outline, mut mesh, mut material, mut transform, mut visibility) in &mut outlines {
        let owner_state = if let Ok((owner_transform, owner_visibility, mut body)) =
            players.get_mut(outline.owner)
        {
            let kind = player_evolution.current_kind;
            set_mesh_if_changed(&mut body, assets.body(kind));
            Some((kind, *owner_transform, *owner_visibility))
        } else if let Ok((evolution, owner_transform, owner_visibility, mut body)) =
            bots.get_mut(outline.owner)
        {
            let kind = evolution.0.current_kind;
            set_mesh_if_changed(&mut body, assets.body(kind));
            Some((kind, *owner_transform, *owner_visibility))
        } else {
            None
        };

        let Some((kind, owner_transform, owner_visibility)) = owner_state else {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };

        set_mesh_if_changed(&mut mesh, assets.outline(kind));
        let next_material = if kind.base() == EvolutionKind::Guard {
            assets.guard_material.clone()
        } else {
            assets.outline_material.clone()
        };
        if material.0 != next_material {
            material.0 = next_material;
        }

        let next_transform = tank_outline_transform(&owner_transform);
        if *transform != next_transform {
            *transform = next_transform;
        }
        if *visibility != owner_visibility {
            *visibility = owner_visibility;
        }
    }
}

fn set_mesh_if_changed(mesh: &mut Mesh2d, next: Handle<Mesh>) {
    if mesh.0 != next {
        mesh.0 = next;
    }
}

fn tank_outline_transform(owner: &Transform) -> Transform {
    let mut outline = *owner;
    outline.translation.z += TANK_OUTLINE_Z_OFFSET;
    outline
}

impl RecentDamage {
    pub fn record_from(&mut self, amount: f32, source_direction: Vec2) {
        self.amount = self.amount.mul_add(0.65, amount);
        self.remaining = 2.5;
        if source_direction.length_squared() > f32::EPSILON {
            self.direction = source_direction.normalize();
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ProjectileCorridor {
    pub start: Vec2,
    pub end: Vec2,
}

pub fn radius(evolution: &EvolutionState) -> f32 {
    if evolution.current_kind == EvolutionKind::Dreadnought {
        return 27.5;
    }
    match evolution.current_kind.base() {
        EvolutionKind::Tank | EvolutionKind::Gunner | EvolutionKind::TwinBarrel => 20.0,
        EvolutionKind::Cannon => 20.8,
        EvolutionKind::Sniper => 18.4,
        EvolutionKind::Sprayer => 19.2,
        EvolutionKind::Flanker => 19.0,
        EvolutionKind::RamCore | EvolutionKind::Guard => 24.6,
        _ => constants::PLAYER_RADIUS,
    }
}

pub fn body_damage(upgrade_damage: f32, evolution: &EvolutionState) -> f32 {
    constants::BASE_BODY_DAMAGE + upgrade_damage + evolution.body_damage_bonus() as f32
}

pub fn contact_damage_for_step(body_damage: f32, fixed_delta: f32) -> f32 {
    body_damage / constants::PLAYER_DAMAGE_COOLDOWN * fixed_delta
}

pub fn health_bar_offset(evolution: &EvolutionState) -> f32 {
    -(radius(evolution) + 15.0)
}

pub fn safe_spawn(
    rng: &mut Rng,
    tanks: &[Vec2],
    shapes: &[Vec2],
    projectiles: &[ProjectileCorridor],
) -> Vec2 {
    let half = constants::arena_half_extent() - 25.0;
    let mut best = Vec2::ZERO;
    let mut best_clearance = f32::NEG_INFINITY;
    for _ in 0..32 {
        let candidate = Vec2::new(rng.range_f32(-half, half), rng.range_f32(-half, half));
        let tank_clearance = nearest_clearance(candidate, tanks).unwrap_or(f32::INFINITY);
        let shape_clearance = nearest_clearance(candidate, shapes).unwrap_or(f32::INFINITY);
        let projectile_clearance = projectiles
            .iter()
            .map(|corridor| point_segment_distance(candidate, corridor.start, corridor.end))
            .fold(f32::INFINITY, f32::min);
        let normalized = (tank_clearance / 300.0_f32)
            .min(shape_clearance / 80.0_f32)
            .min(projectile_clearance / 100.0_f32);
        if normalized > best_clearance {
            best = candidate;
            best_clearance = normalized;
        }
        if tank_clearance >= 300.0 && shape_clearance >= 80.0 && projectile_clearance >= 100.0 {
            return candidate;
        }
    }
    best
}

fn nearest_clearance(point: Vec2, others: &[Vec2]) -> Option<f32> {
    others
        .iter()
        .map(|other| point.distance(*other))
        .reduce(f32::min)
}

fn point_segment_distance(point: Vec2, start: Vec2, end: Vec2) -> f32 {
    let segment = end - start;
    if segment.length_squared() <= f32::EPSILON {
        return point.distance(start);
    }
    let t = ((point - start).dot(segment) / segment.length_squared()).clamp(0.0, 1.0);
    point.distance(start + segment * t)
}

pub fn tick_protection_and_damage(
    time: Res<Time<Fixed>>,
    mut tanks: Query<(&mut SpawnProtection, &mut RecentDamage)>,
) {
    let dt = time.delta_secs();
    for (mut protection, mut damage) in &mut tanks {
        protection.remaining = (protection.remaining - dt).max(0.0);
        damage.remaining = (damage.remaining - dt).max(0.0);
        if damage.remaining == 0.0 {
            damage.amount = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evolution_radii_match_combat_contract() {
        let mut state = EvolutionState::default();
        for (kind, expected) in [
            (EvolutionKind::Tank, 20.0),
            (EvolutionKind::Cannon, 20.8),
            (EvolutionKind::Sniper, 18.4),
            (EvolutionKind::RamCore, 24.6),
            (EvolutionKind::Guard, 24.6),
        ] {
            state.current_kind = kind;
            assert_eq!(radius(&state), expected);
        }
    }

    #[test]
    fn safe_spawn_avoids_known_hazards() {
        let mut rng = Rng::new(9);
        let tanks = [Vec2::ZERO];
        let shapes = [Vec2::new(1000.0, 1000.0)];
        let corridors = [ProjectileCorridor {
            start: Vec2::new(-2000.0, 0.0),
            end: Vec2::new(2000.0, 0.0),
        }];
        let spawn = safe_spawn(&mut rng, &tanks, &shapes, &corridors);
        assert!(spawn.distance(tanks[0]) >= 300.0);
        assert!(spawn.distance(shapes[0]) >= 80.0);
        assert!(point_segment_distance(spawn, corridors[0].start, corridors[0].end) >= 100.0);
    }

    #[test]
    fn contact_damage_is_timestep_independent() {
        let sixty_hz = (0..60)
            .map(|_| contact_damage_for_step(9.0, 1.0 / 60.0))
            .sum::<f32>();
        let twenty_hz = (0..20)
            .map(|_| contact_damage_for_step(9.0, 1.0 / 20.0))
            .sum::<f32>();
        assert!((sixty_hz - twenty_hz).abs() < 0.0001);
    }

    #[test]
    fn firing_can_cancel_spawn_protection() {
        let mut protection = SpawnProtection::default();
        assert!(protection.active());
        protection.cancel();
        assert!(!protection.active());
    }

    #[test]
    fn recent_damage_tracks_the_source_direction() {
        let mut damage = RecentDamage::default();

        damage.record_from(6.0, Vec2::new(3.0, 4.0));

        assert_eq!(damage.amount, 6.0);
        assert_eq!(damage.remaining, 2.5);
        assert!(damage.direction.distance(Vec2::new(0.6, 0.8)) < 0.0001);
    }

    #[test]
    fn standalone_outline_tracks_the_complete_owner_transform() {
        let owner = Transform::from_xyz(42.0, -18.0, 3.0)
            .with_rotation(Quat::from_rotation_z(0.75))
            .with_scale(Vec3::new(1.2, 0.9, 1.0));

        let outline = tank_outline_transform(&owner);

        assert_eq!(outline.translation.xy(), owner.translation.xy());
        assert_eq!(outline.translation.z, owner.translation.z - 0.2);
        assert_eq!(outline.rotation, owner.rotation);
        assert_eq!(outline.scale, owner.scale);
    }
}
