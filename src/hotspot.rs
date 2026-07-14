use crate::{
    constants,
    dominance::{AchievementUnlocked, DominanceState},
    menu::GamePhase,
    player::{Player, PlayerHealth},
    profile::{AchievementId, Profile},
    rng::Rng,
    shape::{Health, MaxHealth, Shape, ShapeAssets, ShapeKind},
};
use bevy::prelude::*;

pub const HOTSPOT_RADIUS: f32 = 350.0;
const FIRST_DELAY: f32 = 45.0;
const ACTIVE_DURATION: f32 = 60.0;
const QUIET_DURATION: f32 = 45.0;
const RESERVE_LIMIT: usize = 20;
const RESERVE_SPAWN_INTERVAL: f32 = 0.15;
const MINIMAP_SIZE: f32 = 168.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HotspotPhase {
    #[default]
    Quiet,
    Active,
}

#[derive(Resource, Debug)]
pub struct HotspotState {
    pub phase: HotspotPhase,
    pub center: Vec2,
    pub timer: f32,
    pub generation: u32,
    spawn_timer: f32,
    zone_candidate: bool,
    zone_failed: bool,
}

impl Default for HotspotState {
    fn default() -> Self {
        Self {
            phase: HotspotPhase::Quiet,
            center: Vec2::ZERO,
            timer: FIRST_DELAY,
            generation: 0,
            spawn_timer: 0.0,
            zone_candidate: false,
            zone_failed: false,
        }
    }
}

impl HotspotState {
    pub fn active(&self) -> bool {
        self.phase == HotspotPhase::Active
    }

    pub fn contains(&self, point: Vec2) -> bool {
        self.active() && point.distance_squared(self.center) <= HOTSPOT_RADIUS * HOTSPOT_RADIUS
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct HotspotShape {
    pub generation: u32,
    pub expired: bool,
}

#[derive(Resource, Default)]
pub struct HotspotShapeKillProgress(pub u32);

#[derive(Component)]
pub struct HotspotField;

#[derive(Component)]
pub struct MinimapRoot;

#[derive(Component)]
pub struct MinimapPlayer;

#[derive(Component)]
pub struct MinimapLeader;

#[derive(Component)]
pub struct MinimapHotspot;

pub fn setup_hotspot(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(HotspotState::default());
    commands.insert_resource(HotspotShapeKillProgress::default());
    commands.spawn((
        HotspotField,
        Mesh2d(meshes.add(Circle::new(HOTSPOT_RADIUS))),
        MeshMaterial2d(materials.add(Color::srgba(0.96, 0.56, 0.12, 0.095))),
        Transform::from_xyz(0.0, 0.0, -0.4),
        Visibility::Hidden,
    ));

    commands
        .spawn((
            MinimapRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(18.0),
                bottom: Val::Px(18.0),
                width: Val::Px(MINIMAP_SIZE),
                height: Val::Px(MINIMAP_SIZE),
                border: UiRect::all(Val::Px(2.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.035, 0.055, 0.88)),
            BorderColor::all(Color::srgba(0.36, 0.49, 0.66, 0.82)),
            GlobalZIndex(40),
            Visibility::Hidden,
        ))
        .with_children(|map| {
            map.spawn((
                MinimapHotspot,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(minimap_hotspot_diameter()),
                    height: Val::Px(minimap_hotspot_diameter()),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Percent(50.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 0.55, 0.10, 0.20)),
                BorderColor::all(Color::srgba(1.0, 0.65, 0.18, 0.82)),
                Visibility::Hidden,
            ));
            map.spawn((
                MinimapLeader,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(9.0),
                    height: Val::Px(9.0),
                    border_radius: BorderRadius::all(Val::Percent(18.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 0.80, 0.18, 1.0)),
                Visibility::Hidden,
            ));
            map.spawn((
                MinimapPlayer,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(9.0),
                    height: Val::Px(9.0),
                    border_radius: BorderRadius::all(Val::Percent(50.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.18, 0.92, 1.0, 1.0)),
            ));
        });
}

#[allow(clippy::too_many_arguments)]
pub fn update_hotspot(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut state: ResMut<HotspotState>,
    mut rng: ResMut<Rng>,
    assets: Res<ShapeAssets>,
    mut hotspot_shapes: Query<
        (
            Entity,
            &Transform,
            &Health,
            &MaxHealth,
            &ShapeKind,
            &mut HotspotShape,
        ),
        With<Shape>,
    >,
    player: Query<(&Transform, &PlayerHealth), With<Player>>,
    mut profile: ResMut<Profile>,
    mut achievements: MessageWriter<AchievementUnlocked>,
    mut kill_progress: ResMut<HotspotShapeKillProgress>,
) {
    let dt = time.delta_secs();
    state.timer -= dt;

    if state.active() {
        let elapsed = ACTIVE_DURATION - state.timer;
        if let Ok((transform, health)) = player.single() {
            let inside = state.contains(transform.translation.xy());
            if elapsed <= 10.0 && inside && health.current > 0.0 {
                state.zone_candidate = true;
            }
            if (state.zone_candidate && (!inside || health.current <= 0.0))
                || (elapsed > 10.0 && !state.zone_candidate)
            {
                state.zone_failed = true;
            }
        } else {
            state.zone_failed = true;
        }

        state.spawn_timer -= dt;
        let live_reserve = hotspot_shapes
            .iter()
            .filter(|(_, _, health, _, _, marker)| {
                !marker.expired && marker.generation == state.generation && health.0 > 0.0
            })
            .count();
        if state.spawn_timer <= 0.0 && live_reserve < RESERVE_LIMIT {
            state.spawn_timer = RESERVE_SPAWN_INTERVAL;
            let angle = rng.range_f32(0.0, std::f32::consts::TAU);
            let distance = HOTSPOT_RADIUS * rng.range_f32(0.12, 0.88).sqrt();
            let position = state.center + Vec2::from_angle(angle) * distance;
            let sides = hotspot_sides_for_roll(rng.next(100));
            let entity = crate::shape::spawn_shape(&mut commands, &assets, sides, position);
            commands.entity(entity).insert(HotspotShape {
                generation: state.generation,
                expired: false,
            });
        }
    }

    if state.timer <= 0.0 {
        match state.phase {
            HotspotPhase::Quiet => {
                for (entity, _, _, _, _, marker) in &mut hotspot_shapes {
                    if marker.expired {
                        commands.entity(entity).despawn();
                    }
                }
                state.phase = HotspotPhase::Active;
                state.timer = ACTIVE_DURATION;
                state.generation = state.generation.wrapping_add(1);
                state.spawn_timer = 0.0;
                let limit = constants::arena_half_extent() - HOTSPOT_RADIUS - 40.0;
                state.center =
                    Vec2::new(rng.range_f32(-limit, limit), rng.range_f32(-limit, limit));
                state.zone_candidate = false;
                state.zone_failed = false;
            }
            HotspotPhase::Active => {
                if state.zone_candidate && !state.zone_failed {
                    profile.data.records.hotspots_survived =
                        profile.data.records.hotspots_survived.saturating_add(1);
                    profile.mark_dirty();
                    if profile.unlock(AchievementId::HoldTheZone) {
                        achievements.write(AchievementUnlocked(AchievementId::HoldTheZone));
                    }
                }
                let mut downgraded = Vec::new();
                for (entity, transform, health, max_health, kind, mut marker) in &mut hotspot_shapes
                {
                    marker.expired = true;
                    let Some(sides) = downgraded_shape_sides(kind.sides) else {
                        continue;
                    };
                    let fraction = (health.0 / max_health.0.max(1.0)).clamp(0.0, 1.0);
                    downgraded.push((entity, transform.translation.xy(), sides, fraction, *marker));
                }
                for (entity, position, sides, fraction, marker) in downgraded {
                    commands.entity(entity).despawn();
                    let replacement =
                        crate::shape::spawn_shape(&mut commands, &assets, sides, position);
                    commands
                        .entity(replacement)
                        .insert((marker, Health(constants::shape_health(sides) * fraction)));
                }
                state.phase = HotspotPhase::Quiet;
                state.timer = QUIET_DURATION;
            }
        }
    }

    if kill_progress.0 > 0 {
        profile.data.records.hotspot_high_tier_shapes_destroyed = profile
            .data
            .records
            .hotspot_high_tier_shapes_destroyed
            .saturating_add(kill_progress.0);
        kill_progress.0 = 0;
        profile.mark_dirty();
        if profile.data.records.hotspot_high_tier_shapes_destroyed >= 100
            && profile.unlock(AchievementId::RichVein)
        {
            achievements.write(AchievementUnlocked(AchievementId::RichVein));
        }
    }
}

pub fn update_hotspot_presentation(
    phase: Res<GamePhase>,
    state: Res<HotspotState>,
    dominance: Res<DominanceState>,
    player: Query<(Entity, &Transform), (With<Player>, Without<HotspotField>)>,
    transforms: Query<&Transform, Without<HotspotField>>,
    mut field: Query<(&mut Transform, &mut Visibility), With<HotspotField>>,
    mut root: Query<
        &mut Visibility,
        (
            With<MinimapRoot>,
            Without<HotspotField>,
            Without<MinimapPlayer>,
            Without<MinimapLeader>,
            Without<MinimapHotspot>,
        ),
    >,
    mut player_marker: Query<
        (&mut Node, &mut Visibility, &mut BackgroundColor),
        (
            With<MinimapPlayer>,
            Without<HotspotField>,
            Without<MinimapRoot>,
            Without<MinimapLeader>,
            Without<MinimapHotspot>,
        ),
    >,
    mut leader_marker: Query<
        (&mut Node, &mut Visibility),
        (
            With<MinimapLeader>,
            Without<HotspotField>,
            Without<MinimapRoot>,
            Without<MinimapPlayer>,
            Without<MinimapHotspot>,
        ),
    >,
    mut hotspot_marker: Query<
        (&mut Node, &mut Visibility),
        (
            With<MinimapHotspot>,
            Without<HotspotField>,
            Without<MinimapRoot>,
            Without<MinimapPlayer>,
            Without<MinimapLeader>,
        ),
    >,
) {
    let visible = *phase != GamePhase::Menu;
    for mut visibility in &mut root {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut transform, mut visibility) in &mut field {
        transform.translation.x = state.center.x;
        transform.translation.y = state.center.y;
        *visibility = if state.active() && visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let Ok((player_entity, player_transform)) = player.single() else {
        return;
    };
    if let Ok((mut node, mut visibility, mut color)) = player_marker.single_mut() {
        place_marker(&mut node, player_transform.translation.xy(), 9.0);
        *visibility = Visibility::Visible;
        *color = if dominance.leader == Some(player_entity) {
            BackgroundColor(Color::srgba(0.42, 0.95, 1.0, 1.0))
        } else {
            BackgroundColor(Color::srgba(0.18, 0.92, 1.0, 1.0))
        };
    }
    if let Ok((mut node, mut visibility)) = leader_marker.single_mut() {
        if let Some(leader) = dominance.leader.filter(|leader| *leader != player_entity)
            && let Ok(transform) = transforms.get(leader)
        {
            place_marker(&mut node, transform.translation.xy(), 9.0);
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
    if let Ok((mut node, mut visibility)) = hotspot_marker.single_mut() {
        place_marker(&mut node, state.center, minimap_hotspot_diameter());
        *visibility = if state.active() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn place_marker(node: &mut Node, position: Vec2, diameter: f32) {
    let half = constants::arena_half_extent();
    node.left = Val::Px(
        ((position.x + half) / (half * 2.0) * MINIMAP_SIZE - diameter * 0.5)
            .clamp(0.0, MINIMAP_SIZE - diameter),
    );
    node.top = Val::Px(
        ((half - position.y) / (half * 2.0) * MINIMAP_SIZE - diameter * 0.5)
            .clamp(0.0, MINIMAP_SIZE - diameter),
    );
}

fn minimap_hotspot_diameter() -> f32 {
    HOTSPOT_RADIUS * 2.0 / (constants::arena_half_extent() * 2.0) * MINIMAP_SIZE
}

fn downgraded_shape_sides(sides: u32) -> Option<u32> {
    match sides {
        5 => Some(3),
        6 => Some(4),
        _ => None,
    }
}

pub fn hotspot_sides_for_roll(roll: u32) -> u32 {
    match roll {
        0..=9 => 3,
        10..=29 => 4,
        30..=74 => 5,
        _ => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotspot_distribution_is_exact() {
        let counts = (0..100).fold([0; 4], |mut counts, roll| {
            counts[(hotspot_sides_for_roll(roll) - 3) as usize] += 1;
            counts
        });
        assert_eq!(counts, [10, 20, 45, 25]);
    }

    #[test]
    fn cadence_defaults_to_first_delay_and_uses_fixed_windows() {
        let state = HotspotState::default();
        assert_eq!(state.phase, HotspotPhase::Quiet);
        assert_eq!(state.timer, 45.0);
        assert_eq!((ACTIVE_DURATION, QUIET_DURATION), (60.0, 45.0));
        assert_eq!(RESERVE_LIMIT, 20);
    }

    #[test]
    fn arena_mapping_is_north_up() {
        let mut node = Node::default();
        place_marker(&mut node, Vec2::new(-2_000.0, 2_000.0), 8.0);
        assert_eq!(node.left, Val::Px(0.0));
        assert_eq!(node.top, Val::Px(0.0));
    }

    #[test]
    fn high_tier_reserve_downgrades_without_touching_lower_tiers() {
        assert_eq!(downgraded_shape_sides(5), Some(3));
        assert_eq!(downgraded_shape_sides(6), Some(4));
        assert_eq!(downgraded_shape_sides(3), None);
        assert_eq!(downgraded_shape_sides(4), None);
    }
}
