use crate::{
    ability::Construct, feedback::EffectParticle, menu::GamePhase, profile::Profile,
    projectile::Projectile, shape::Shape,
};
use bevy::{app::AppExit, prelude::*, time::Real};
use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const FRAME_HISTORY_CAPACITY: usize = 240;
const HITCH_HISTORY_CAPACITY: usize = 64;
const HITCH_THRESHOLD_MS: f32 = 50.0;
const HITCH_RATE_LIMIT_SECS: f64 = 5.0;
const OVERLAY_REFRESH_SECS: f32 = 0.25;
const MAX_LOG_BYTES: u64 = 512 * 1024;

#[derive(Component)]
pub struct PerformanceOverlay;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct EntityCounts {
    projectiles: usize,
    shapes: usize,
    constructs: usize,
    active_particles: usize,
    total: usize,
}

#[derive(Clone, Debug)]
struct HitchSnapshot {
    sequence: u64,
    elapsed_secs: f64,
    frame_ms: f32,
    average_fps: f32,
    p95_fps: f32,
    worst_frame_ms: f32,
    fixed_steps: u32,
    counts: EntityCounts,
    phase: GamePhase,
    width: u32,
    height: u32,
    fullscreen: bool,
    low_power: bool,
}

#[derive(Resource)]
pub struct PerformanceTelemetry {
    overlay_visible: bool,
    overlay_elapsed: f32,
    fixed_steps: u32,
    frame_times_ms: VecDeque<f32>,
    stats_scratch: Vec<f32>,
    hitches: VecDeque<HitchSnapshot>,
    next_sequence: u64,
    last_hitch_elapsed: Option<f64>,
    last_flushed_sequence: u64,
}

impl Default for PerformanceTelemetry {
    fn default() -> Self {
        Self {
            overlay_visible: false,
            overlay_elapsed: OVERLAY_REFRESH_SECS,
            fixed_steps: 0,
            frame_times_ms: VecDeque::with_capacity(FRAME_HISTORY_CAPACITY),
            stats_scratch: Vec::with_capacity(FRAME_HISTORY_CAPACITY),
            hitches: VecDeque::with_capacity(HITCH_HISTORY_CAPACITY),
            next_sequence: 1,
            last_hitch_elapsed: None,
            last_flushed_sequence: 0,
        }
    }
}

impl PerformanceTelemetry {
    fn record_frame(&mut self, frame_ms: f32) {
        if self.frame_times_ms.len() == FRAME_HISTORY_CAPACITY {
            self.frame_times_ms.pop_front();
        }
        self.frame_times_ms.push_back(frame_ms);
    }

    fn stats(&mut self) -> (f32, f32, f32) {
        if self.frame_times_ms.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        let average_ms = self.frame_times_ms.iter().sum::<f32>() / self.frame_times_ms.len() as f32;
        self.stats_scratch.clear();
        self.stats_scratch
            .extend(self.frame_times_ms.iter().copied());
        self.stats_scratch.sort_by(f32::total_cmp);
        let p95_index = ((self.stats_scratch.len() - 1) as f32 * 0.95).round() as usize;
        let p95_ms = self.stats_scratch[p95_index];
        let worst_ms = *self.stats_scratch.last().unwrap_or(&0.0);
        (fps_from_ms(average_ms), fps_from_ms(p95_ms), worst_ms)
    }

    fn can_capture_hitch(&self, elapsed_secs: f64, frame_ms: f32) -> bool {
        frame_ms > HITCH_THRESHOLD_MS
            && self
                .last_hitch_elapsed
                .is_none_or(|last| elapsed_secs - last >= HITCH_RATE_LIMIT_SECS)
    }

    fn push_hitch(&mut self, mut snapshot: HitchSnapshot) {
        snapshot.sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1).max(1);
        self.last_hitch_elapsed = Some(snapshot.elapsed_secs);
        if self.hitches.len() == HITCH_HISTORY_CAPACITY {
            self.hitches.pop_front();
        }
        self.hitches.push_back(snapshot);
    }
}

fn fps_from_ms(frame_ms: f32) -> f32 {
    if frame_ms > f32::EPSILON {
        1_000.0 / frame_ms
    } else {
        0.0
    }
}

pub fn setup_performance_overlay(mut commands: Commands) {
    commands.spawn((
        PerformanceOverlay,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgba(0.82, 0.94, 1.0, 0.96)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.018, 0.025, 0.04, 0.86)),
        GlobalZIndex(200),
        Pickable::IGNORE,
        Visibility::Hidden,
    ));
}

pub fn count_fixed_step(mut telemetry: ResMut<PerformanceTelemetry>) {
    telemetry.fixed_steps = telemetry.fixed_steps.saturating_add(1);
}

pub fn toggle_performance_overlay(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut telemetry: ResMut<PerformanceTelemetry>,
    mut overlay: Query<&mut Visibility, With<PerformanceOverlay>>,
) {
    if !keyboard.just_pressed(KeyCode::F3) {
        return;
    }
    telemetry.overlay_visible = !telemetry.overlay_visible;
    telemetry.overlay_elapsed = OVERLAY_REFRESH_SECS;
    for mut visibility in &mut overlay {
        *visibility = if telemetry.overlay_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sample_performance(
    real_time: Res<Time<Real>>,
    phase: Res<GamePhase>,
    profile: Res<Profile>,
    window: Single<&Window>,
    projectiles: Query<Entity, With<Projectile>>,
    shapes: Query<Entity, With<Shape>>,
    constructs: Query<Entity, With<Construct>>,
    particles: Query<&Visibility, With<EffectParticle>>,
    entities: Query<Entity>,
    mut telemetry: ResMut<PerformanceTelemetry>,
    mut overlay: Query<&mut Text, With<PerformanceOverlay>>,
) {
    let frame_ms = real_time.delta_secs() * 1_000.0;
    let elapsed_secs = real_time.elapsed_secs_f64();
    telemetry.record_frame(frame_ms);
    telemetry.overlay_elapsed += real_time.delta_secs();

    let capture_hitch = telemetry.can_capture_hitch(elapsed_secs, frame_ms);
    let refresh_overlay =
        telemetry.overlay_visible && telemetry.overlay_elapsed >= OVERLAY_REFRESH_SECS;
    if capture_hitch || refresh_overlay {
        let counts = EntityCounts {
            projectiles: projectiles.iter().count(),
            shapes: shapes.iter().count(),
            constructs: constructs.iter().count(),
            active_particles: particles
                .iter()
                .filter(|visibility| **visibility != Visibility::Hidden)
                .count(),
            total: entities.iter().count(),
        };
        let fixed_steps = telemetry.fixed_steps;
        let (average_fps, p95_fps, worst_frame_ms) = telemetry.stats();
        if refresh_overlay {
            telemetry.overlay_elapsed %= OVERLAY_REFRESH_SECS;
            for mut text in &mut overlay {
                **text = format!(
                    "FPS avg {average_fps:.0}  p95 {p95_fps:.0}  worst {worst_frame_ms:.1} ms\nFixed steps {}  Projectiles {}  Shapes {}\nConstructs {}  Particles {}  Entities {}",
                    fixed_steps,
                    counts.projectiles,
                    counts.shapes,
                    counts.constructs,
                    counts.active_particles,
                    counts.total,
                );
            }
        }
        if capture_hitch {
            telemetry.push_hitch(HitchSnapshot {
                sequence: 0,
                elapsed_secs,
                frame_ms,
                average_fps,
                p95_fps,
                worst_frame_ms,
                fixed_steps,
                counts,
                phase: *phase,
                width: window.resolution.physical_width(),
                height: window.resolution.physical_height(),
                fullscreen: profile.data.settings.fullscreen,
                low_power: profile.data.settings.low_power_mode,
            });
        }
    }
    telemetry.fixed_steps = 0;
}

pub fn flush_performance_log(
    phase: Res<GamePhase>,
    mut exits: MessageReader<AppExit>,
    mut telemetry: ResMut<PerformanceTelemetry>,
) {
    let exiting = exits.read().next().is_some();
    if !may_flush(*phase, exiting) {
        return;
    }
    let pending = telemetry
        .hitches
        .iter()
        .filter(|snapshot| snapshot.sequence > telemetry.last_flushed_sequence)
        .map(format_snapshot)
        .collect::<String>();
    if pending.is_empty() {
        return;
    }
    let Some(path) = performance_log_path() else {
        return;
    };
    if append_rotating_log(&path, pending.as_bytes(), MAX_LOG_BYTES).is_ok() {
        telemetry.last_flushed_sequence = telemetry
            .hitches
            .back()
            .map_or(telemetry.last_flushed_sequence, |snapshot| {
                snapshot.sequence
            });
    }
}

fn may_flush(phase: GamePhase, exiting: bool) -> bool {
    phase != GamePhase::Playing || exiting
}

fn format_snapshot(snapshot: &HitchSnapshot) -> String {
    format!(
        "t={:.3} frame_ms={:.3} avg_fps={:.1} p95_fps={:.1} worst_ms={:.3} fixed_steps={} projectiles={} shapes={} constructs={} particles={} entities={} phase={} resolution={}x{} fullscreen={} low_power={}\n",
        snapshot.elapsed_secs,
        snapshot.frame_ms,
        snapshot.average_fps,
        snapshot.p95_fps,
        snapshot.worst_frame_ms,
        snapshot.fixed_steps,
        snapshot.counts.projectiles,
        snapshot.counts.shapes,
        snapshot.counts.constructs,
        snapshot.counts.active_particles,
        snapshot.counts.total,
        phase_name(snapshot.phase),
        snapshot.width,
        snapshot.height,
        snapshot.fullscreen,
        snapshot.low_power,
    )
}

fn phase_name(phase: GamePhase) -> &'static str {
    match phase {
        GamePhase::Menu => "menu",
        GamePhase::Playing => "playing",
        GamePhase::Paused => "paused",
        GamePhase::Dead => "dead",
    }
}

fn performance_log_path() -> Option<PathBuf> {
    crate::profile::config_dir().map(|directory| directory.join("performance.log"))
}

fn append_rotating_log(path: &Path, bytes: &[u8], max_bytes: u64) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing_bytes = fs::metadata(path).map_or(0, |metadata| metadata.len());
    if existing_bytes.saturating_add(bytes.len() as u64) > max_bytes && existing_bytes > 0 {
        let previous = path.with_file_name("performance.previous.log");
        match fs::remove_file(&previous) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        fs::rename(path, previous)?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("polycore-{name}-{}", std::process::id()))
    }

    #[test]
    fn frame_statistics_and_hitch_rate_limit_are_stable() {
        let mut telemetry = PerformanceTelemetry::default();
        for frame_ms in [10.0, 12.0, 14.0, 16.0, 100.0] {
            telemetry.record_frame(frame_ms);
        }
        let (average, p95, worst) = telemetry.stats();
        assert!((average - 32.89).abs() < 0.1);
        assert!((p95 - 10.0).abs() < 0.1);
        assert_eq!(worst, 100.0);
        assert!(telemetry.can_capture_hitch(1.0, 50.1));
        telemetry.last_hitch_elapsed = Some(1.0);
        assert!(!telemetry.can_capture_hitch(5.9, 75.0));
        assert!(telemetry.can_capture_hitch(6.0, 75.0));
        assert!(!telemetry.can_capture_hitch(7.0, 50.0));
    }

    #[test]
    fn rotating_log_keeps_one_previous_file() {
        let root = test_path("performance-log");
        let path = root.join("performance.log");
        let previous = root.join("performance.previous.log");
        let _ = fs::remove_dir_all(&root);
        append_rotating_log(&path, b"first\n", 12).unwrap();
        append_rotating_log(&path, b"second\n", 12).unwrap();
        assert_eq!(fs::read_to_string(&previous).unwrap(), "first\n");
        assert_eq!(fs::read_to_string(&path).unwrap(), "second\n");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn overlay_toggle_is_session_only() {
        let mut world = World::new();
        let mut keyboard = ButtonInput::<KeyCode>::default();
        keyboard.press(KeyCode::F3);
        world.insert_resource(keyboard);
        world.insert_resource(PerformanceTelemetry::default());
        let overlay = world.spawn((PerformanceOverlay, Visibility::Hidden)).id();
        let mut schedule = Schedule::default();
        schedule.add_systems(toggle_performance_overlay);
        schedule.run(&mut world);
        assert!(world.resource::<PerformanceTelemetry>().overlay_visible);
        assert_eq!(
            *world.get::<Visibility>(overlay).unwrap(),
            Visibility::Visible
        );
    }

    #[test]
    fn performance_logs_flush_only_in_safe_states() {
        assert!(!may_flush(GamePhase::Playing, false));
        assert!(may_flush(GamePhase::Paused, false));
        assert!(may_flush(GamePhase::Dead, false));
        assert!(may_flush(GamePhase::Menu, false));
        assert!(may_flush(GamePhase::Playing, true));
    }

    #[test]
    fn hitch_snapshot_captures_live_entity_counters() {
        let mut world = World::new();
        let mut real_time = Time::<Real>::default();
        real_time.update_with_duration(std::time::Duration::ZERO);
        real_time.update_with_duration(std::time::Duration::from_millis(60));
        world.insert_resource(real_time);
        world.insert_resource(GamePhase::Playing);
        world.insert_resource(Profile::test_with_path(None));
        world.insert_resource(PerformanceTelemetry::default());
        world.spawn(Window::default());
        world.spawn((PerformanceOverlay, Text::new("")));
        world.spawn(Projectile);
        world.spawn(Projectile);
        world.spawn(Shape);
        world.spawn(Construct {
            kind: crate::ability::ConstructKind::Mine,
            owner: crate::projectile::ProjectileOwner::Player,
            generation: 1,
            health: 1.0,
            max_health: 1.0,
            remaining: 1.0,
            duration: 1.0,
            damage: 1.0,
            projectile_speed: 0.0,
            range: 0.0,
            fire_timer: 0.0,
        });
        world.spawn((
            EffectParticle {
                velocity: Vec2::ZERO,
                remaining: 1.0,
            },
            Visibility::Visible,
        ));
        let mut schedule = Schedule::default();
        schedule.add_systems(sample_performance);
        schedule.run(&mut world);
        let telemetry = world.resource::<PerformanceTelemetry>();
        let snapshot = telemetry.hitches.back().unwrap();
        assert_eq!(snapshot.counts.projectiles, 2);
        assert_eq!(snapshot.counts.shapes, 1);
        assert_eq!(snapshot.counts.constructs, 1);
        assert_eq!(snapshot.counts.active_particles, 1);
        assert_eq!(snapshot.frame_ms, 60.0);
    }
}
