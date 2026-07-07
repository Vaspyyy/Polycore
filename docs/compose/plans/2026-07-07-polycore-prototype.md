# Polycore Prototype Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use compose:subagent (recommended) or compose:execute to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a minimal playable Polycore prototype — native window, WASD player, mouse aim/shoot, destructible XP polygons, collision, HUD.

**Architecture:** Bevy 0.19 app with 7 modules. Mouse input + aiming in `Update`, movement/collision/spawning in `FixedUpdate`. Simple bounding-radius collision. Deterministic LCG RNG resource.

**Tech Stack:** Rust 1.96.0, Bevy 0.19 (no other crates)

## Global Constraints

- Bevy 0.19 only, no other dependencies
- Mouse aiming and shooting fire in `Update` for responsiveness
- Movement, projectile travel, shape spawning, collision in `FixedUpdate`
- Shapes (XP polygons) in `shape.rs`, not `enemy.rs`
- Collision: distance-based bounding radii only
- Deterministic RNG as a resource (LCG), no `rand` crate
- Modular from the start: `src/{main, constants, rng, player, projectile, shape, collision, hud}.rs`
- `cargo run` must launch a playable prototype

---

### Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/constants.rs`

**Interfaces:**
- Produces: `Cargo.toml` with Bevy 0.19 dependency; `src/main.rs` with app skeleton; `src/constants.rs` with all shared numeric constants

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "polycore"
version = "0.1.0"
edition = "2024"

[dependencies]
bevy = "0.19"
```

- [ ] **Step 2: Create src/constants.rs**

```rust
// Window
pub const WINDOW_WIDTH: f32 = 1024.0;
pub const WINDOW_HEIGHT: f32 = 768.0;
pub const WINDOW_TITLE: &str = "Polycore";

// Player
pub const PLAYER_RADIUS: f32 = 20.0;
pub const PLAYER_SPEED: f32 = 300.0;
pub const PLAYER_COLOR: [f32; 4] = [0.2, 0.6, 1.0, 1.0];

// Shooting
pub const SHOOT_COOLDOWN: f32 = 0.3;
pub const PROJECTILE_RADIUS: f32 = 4.0;
pub const PROJECTILE_SPEED: f32 = 600.0;
pub const PROJECTILE_LIFETIME: f32 = 2.0;

// Shapes (XP polygons)
pub const SHAPE_RADIUS: f32 = 25.0;
pub const SHAPE_SPAWN_INTERVAL: f32 = 1.5;
pub const SHAPE_MAX_COUNT: usize = 20;

// XP
pub const XP_PER_KILL: u32 = 10;
pub const XP_PER_LEVEL: u32 = 100;

// Colors
pub const PROJECTILE_COLOR: [f32; 4] = [1.0, 0.8, 0.2, 1.0];
pub const ENEMY_COLOR: [f32; 4] = [0.9, 0.2, 0.2, 1.0];
pub const BG_COLOR: [f32; 4] = [0.05, 0.05, 0.08, 1.0];
```

- [ ] **Step 3: Create src/main.rs (skeleton)**

```rust
mod constants;
mod rng;
mod player;
mod projectile;
mod shape;
mod collision;
mod hud;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: constants::WINDOW_TITLE.into(),
                resolution: (constants::WINDOW_WIDTH, constants::WINDOW_HEIGHT).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::from(constants::BG_COLOR)))
        .run();
}
```

- [ ] **Step 4: Verify scaffold builds**

Run: `cargo check`
Expected: Compiles successfully (unused warnings OK)

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/main.rs src/constants.rs
git commit -m "feat: scaffold project with Bevy 0.19 and constants"
```

---

### Task 2: RNG Resource

**Files:**
- Create: `src/rng.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Produces: `Rng` resource — `new(seed: u64) -> Self`, `Rng::next(&mut self, max: u32) -> u32`

- [ ] **Step 1: Write src/rng.rs**

```rust
use bevy::prelude::*;

/// Simple LCG random number generator.
#[derive(Resource)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns a value in [0, max).
    pub fn next(&mut self, max: u32) -> u32 {
        // LCG parameters from Numerical Recipes
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.state >> 33) as u32) % max
    }
}
```

- [ ] **Step 2: Register Rng in main.rs**

In `src/main.rs`, add after `.insert_resource(ClearColor(...))`:

```rust
        .insert_resource(rng::Rng::new(12345))
```

- [ ] **Step 3: Verify builds**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add src/rng.rs src/main.rs
git commit -m "feat: add deterministic LCG RNG resource"
```

---

### Task 3: Player — Movement, Aiming, Visual

**Files:**
- Create: `src/player.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Produces: `Player` marker component; `setup_player` startup system; `player_movement` (FixedUpdate); `player_aim` (Update); `Velocity` component
- Consumes: `constants::{PLAYER_RADIUS, PLAYER_SPEED, PLAYER_COLOR}`

- [ ] **Step 1: Write src/player.rs (skeleton)**

```rust
use bevy::prelude::*;
use crate::constants;

#[derive(Component)]
pub struct Player;

#[derive(Component, Default)]
pub struct Velocity(pub Vec2);

pub fn setup_player(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands.spawn((
        Player,
        Velocity::default(),
        Mesh2d(meshes.add(Circle::new(constants::PLAYER_RADIUS))),
        MeshMaterial2d(materials.add(Color::from(constants::PLAYER_COLOR))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}
```

- [ ] **Step 2: Implement player_movement (FixedUpdate)**

Add to `src/player.rs`:

```rust
pub fn player_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Velocity), With<Player>>,
) {
    let Ok((mut transform, mut velocity)) = query.get_single_mut() else { return };

    let mut direction = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) { direction.y += 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { direction.y -= 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { direction.x += 1.0; }

    let direction = direction.normalize_or_zero();
    velocity.0 = direction * constants::PLAYER_SPEED;
    transform.translation += direction.extend(0.0) * constants::PLAYER_SPEED * time.delta_secs();
}
```

- [ ] **Step 3: Implement player_aim (Update)**

Add to `src/player.rs`:

```rust
pub fn player_aim(
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut transform) = query.get_single_mut() else { return };

    let Some(cursor) = window.cursor_position() else { return };
    let Ok(world_pos) = camera.0.viewport_to_world_2d(camera.1, cursor) else { return };

    let delta = world_pos - transform.translation.xy();
    if delta.length_squared() > 0.001 {
        transform.rotation = Quat::from_rotation_z(delta.y.atan2(delta.x));
    }
}
```

- [ ] **Step 4: Add player visual directional indicator (turret)**

Add to `setup_player` — spawn a child rectangle as a turret barrel:

```rust
pub fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let player_id = commands.spawn((
        Player,
        Velocity::default(),
        Mesh2d(meshes.add(Circle::new(constants::PLAYER_RADIUS))),
        MeshMaterial2d(materials.add(Color::from(constants::PLAYER_COLOR))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    )).id();

    // Turret barrel (rectangle indicating aim direction)
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(4.0, constants::PLAYER_RADIUS + 10.0))),
        MeshMaterial2d(materials.add(Color::from(constants::PLAYER_COLOR))),
        Transform::from_xyz(0.0, constants::PLAYER_RADIUS / 2.0 + 5.0, 0.0),
    )).set_parent(player_id);
}
```

- [ ] **Step 5: Register player systems + camera in main.rs**

In `src/main.rs`, update main():

```rust
mod constants;
mod rng;
mod player;
mod projectile;
mod shape;
mod collision;
mod hud;

use bevy::prelude::*;
use bevy::window::WindowResolution;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: constants::WINDOW_TITLE.into(),
                resolution: WindowResolution::new(constants::WINDOW_WIDTH, constants::WINDOW_HEIGHT),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::from(constants::BG_COLOR)))
        .insert_resource(rng::Rng::new(12345))
        .add_systems(Startup, (setup_camera, player::setup_player))
        .add_systems(Update, player::player_aim)
        .add_systems(FixedUpdate, player::player_movement)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
```

- [ ] **Step 6: Verify builds and cursor lock**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 7: Commit**

```bash
git add src/player.rs src/main.rs
git commit -m "feat: add player with WASD movement and mouse aiming"
```

---

### Task 4: Projectiles — Shooting and Movement

**Files:**
- Create: `src/projectile.rs`
- Modify: `src/main.rs`, `src/player.rs`

**Interfaces:**
- Produces: `Projectile` marker; `ShootCooldown` component; `shoot_projectile` (Update); `projectile_update` (FixedUpdate)
- Consumes: `constants::{SHOOT_COOLDOWN, PROJECTILE_RADIUS, PROJECTILE_SPEED, PROJECTILE_LIFETIME, PROJECTILE_COLOR}`

- [ ] **Step 1: Write src/projectile.rs**

```rust
use bevy::prelude::*;
use crate::constants;

#[derive(Component)]
pub struct Projectile;

#[derive(Component)]
pub struct Lifetime(pub f32);

#[derive(Component)]
pub struct ShootCooldown(pub f32);

pub fn shoot_projectile(
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut player_query: Query<(&Transform, &mut ShootCooldown), With<super::player::Player>>,
    window: Single<&Window>,
) {
    let Ok((transform, mut cooldown)) = player_query.get_single_mut() else { return };

    cooldown.0 -= time.delta_secs();
    if cooldown.0 > 0.0 {
        return;
    }

    if !mouse.pressed(MouseButton::Left) {
        return;
    }

    cooldown.0 = constants::SHOOT_COOLDOWN;
    if cooldown.0 < 0.0 {
        return; // shouldn't happen, but belt-and-suspenders
    }

    // The player's rotation gives us the aim direction.
    let direction = transform.rotation * Vec3::Y;
    let spawn_pos = transform.translation + direction * (constants::PLAYER_RADIUS + constants::PROJECTILE_RADIUS + 4.0);

    commands.spawn((
        Projectile,
        Lifetime(constants::PROJECTILE_LIFETIME),
        Mesh2d(meshes.add(Circle::new(constants::PROJECTILE_RADIUS))),
        MeshMaterial2d(materials.add(Color::from(constants::PROJECTILE_COLOR))),
        Transform::from_translation(spawn_pos).with_rotation(transform.rotation),
        super::player::Velocity(direction.xy() * constants::PROJECTILE_SPEED),
    ));
}

pub fn projectile_update(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &super::player::Velocity, &mut Lifetime), With<Projectile>>,
) {
    for (entity, mut transform, velocity, mut lifetime) in query.iter_mut() {
        let dt = time.delta_secs();
        lifetime.0 -= dt;
        if lifetime.0 <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation += velocity.0.extend(0.0) * dt;
    }
}
```

- [ ] **Step 2: Add ShootCooldown to player spawn**

In `src/player.rs`, update `setup_player` spawn to include `ShootCooldown`:

```rust
use crate::projectile::ShootCooldown;

// In the player spawn, add:
        ShootCooldown(0.0),
```

So the player spawn becomes:

```rust
    commands.spawn((
        Player,
        Velocity::default(),
        ShootCooldown(0.0),
        Mesh2d(meshes.add(Circle::new(constants::PLAYER_RADIUS))),
        MeshMaterial2d(materials.add(Color::from(constants::PLAYER_COLOR))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
```

- [ ] **Step 3: Register projectile systems in main.rs**

In `src/main.rs`, add to systems:

```rust
        .add_systems(Update, projectile::shoot_projectile.after(player::player_aim))
        .add_systems(FixedUpdate, projectile::projectile_update)
```

- [ ] **Step 4: Verify builds**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 5: Commit**

```bash
git add src/projectile.rs src/player.rs src/main.rs
git commit -m "feat: add projectile shooting and movement"
```

---

### Task 5: XP Shapes — Spawning and Visuals

**Files:**
- Create: `src/shape.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Produces: `Shape` marker; `XpValue(u32)`; `Level` resource; `shape_spawn` (FixedUpdate)
- Consumes: `constants::{SHAPE_RADIUS, SHAPE_SPAWN_INTERVAL, SHAPE_MAX_COUNT, XP_PER_KILL, XP_PER_LEVEL, ENEMY_COLOR, WINDOW_WIDTH, WINDOW_HEIGHT}`; `rng::Rng`

- [ ] **Step 1: Write src/shape.rs**

```rust
use bevy::prelude::*;
use crate::{constants, rng::Rng};

#[derive(Component)]
pub struct Shape;

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

    // Random polygon: 3 (triangle) to 6 (hexagon)
    let sides = 3 + rng.next(4) as usize;

    commands.spawn((
        Shape,
        XpValue(constants::XP_PER_KILL),
        Mesh2d(meshes.add(RegularPolygon::new(constants::SHAPE_RADIUS, sides))),
        MeshMaterial2d(materials.add(Color::from(constants::ENEMY_COLOR))),
        Transform::from_xyz(x, y, 0.0),
    ));
}
```

- [ ] **Step 2: Register shape systems in main.rs**

In `src/main.rs`, add:

```rust
        .add_systems(Startup, shape::setup_xp)
        .add_systems(FixedUpdate, shape::shape_spawn)
```

- [ ] **Step 3: Verify builds**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add src/shape.rs src/main.rs
git commit -m "feat: add XP polygon shape spawning"
```

---

### Task 6: Collision Detection

**Files:**
- Create: `src/collision.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Produces: `check_collisions` (FixedUpdate) — reads Projectile + Shape entities, despawns both on overlap, adds XP
- Consumes: `projectile::Projectile`, `shape::{Shape, XpValue, Xp}`, `constants::PROJECTILE_RADIUS`, `constants::SHAPE_RADIUS`

- [ ] **Step 1: Write src/collision.rs**

```rust
use bevy::prelude::*;
use crate::{constants, projectile::Projectile, shape::{Shape, XpValue, Xp}};

pub fn check_collisions(
    mut commands: Commands,
    projectiles: Query<(Entity, &Transform), With<Projectile>>,
    shapes: Query<(Entity, &Transform, &XpValue), With<Shape>>,
    mut xp: ResMut<Xp>,
) {
    // Collect positions to avoid borrow conflicts
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
                break; // one projectile only hits one shape
            }
        }
    }
}
```

- [ ] **Step 2: Register collision system in main.rs**

In `src/main.rs`, add:

```rust
        .add_systems(FixedUpdate, collision::check_collisions.after(projectile::projectile_update))
```

- [ ] **Step 3: Verify builds**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add src/collision.rs src/main.rs
git commit -m "feat: add projectile-shape collision detection"
```

---

### Task 7: HUD — Level and XP Display

**Files:**
- Create: `src/hud.rs`
- Modify: `src/main.rs`, `src/shape.rs`

**Interfaces:**
- Produces: `setup_hud` (Startup); `update_hud` (Update) — reads Xp/Level resources, updates UI text
- Consumes: `shape::{Xp, Level}`, `constants::XP_PER_LEVEL`

- [ ] **Step 1: Add level-up logic to shape.rs**

Add to `src/shape.rs`:

```rust
use crate::constants::XP_PER_LEVEL;

pub fn check_level_up(mut xp: ResMut<Xp>, mut level: ResMut<Level>) {
    while xp.0 >= XP_PER_LEVEL {
        xp.0 -= XP_PER_LEVEL;
        level.0 += 1;
    }
}
```

Register it in main.rs after collision:

```rust
        .add_systems(FixedUpdate, shape::check_level_up.after(collision::check_collisions))
```

- [ ] **Step 2: Write src/hud.rs**

```rust
use bevy::prelude::*;
use crate::shape::{Xp, Level};

#[derive(Component)]
struct HudText;

pub fn setup_hud(mut commands: Commands) {
    commands.spawn((
        Text::new("Level: 1 | XP: 0"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        HudText,
    ));
}

pub fn update_hud(
    xp: Res<Xp>,
    level: Res<Level>,
    mut query: Query<&mut Text, With<HudText>>,
) {
    if xp.is_changed() || level.is_changed() {
        for mut text in query.iter_mut() {
            **text = format!("Level: {} | XP: {}", level.0, xp.0);
        }
    }
}
```

- [ ] **Step 3: Register HUD systems in main.rs**

In `src/main.rs`, add:

```rust
        .add_systems(Startup, hud::setup_hud)
        .add_systems(Update, hud::update_hud)
```

And also register `check_level_up`:

```rust
        .add_systems(FixedUpdate, (player::player_movement, projectile::projectile_update, shape::shape_spawn, collision::check_collisions, shape::check_level_up).chain())
```

- [ ] **Step 4: Verify builds**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 5: Commit**

```bash
git add src/hud.rs src/shape.rs src/main.rs
git commit -m "feat: add HUD with level and XP display"
```

---

### Task 8: Run, Test, and Polish

**Files:**
- Modify: `src/main.rs`

**Description:** Full integration check. Launch the game, verify playability, fix any issues.

- [ ] **Step 1: Final main.rs clean-up — combine all registrations**

```rust
mod constants;
mod rng;
mod player;
mod projectile;
mod shape;
mod collision;
mod hud;

use bevy::prelude::*;
use bevy::window::WindowResolution;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: constants::WINDOW_TITLE.into(),
                resolution: WindowResolution::new(constants::WINDOW_WIDTH, constants::WINDOW_HEIGHT),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::from(constants::BG_COLOR)))
        .insert_resource(rng::Rng::new(12345))
        .add_systems(Startup, (
            setup_camera,
            player::setup_player,
            shape::setup_xp,
            hud::setup_hud,
        ))
        .add_systems(Update, (
            player::player_aim,
            projectile::shoot_projectile,
            hud::update_hud,
        ).chain())
        .add_systems(FixedUpdate, (
            player::player_movement,
            projectile::projectile_update,
            shape::shape_spawn,
            collision::check_collisions,
            shape::check_level_up,
        ).chain())
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
```

- [ ] **Step 2: Build and fix compilation errors**

Run: `cargo build 2>&1`
Expected: Builds successfully with no errors

- [ ] **Step 3: Run the prototype**

Run: `cargo run 2>&1`
Expected: Window opens, player circle with turret visible, WASD movement, mouse aiming, click to shoot, shapes spawn, collision works, HUD updates

- [ ] **Step 4: Fix any runtime issues**

Check for common Bevy 0.19 API issues:
- `Color::from([r, g, b, a])` → may need `Color::srgba(r, g, b, a)` or `Color::linear_rgba(r, g, b, a)`
- `Mesh2d` / `MeshMaterial2d` → verify these exist in 0.19
- `WindowResolution::new(w, h)` → verify this API

Fix any compilation errors, re-run `cargo build`, then `cargo run`.

- [ ] **Step 5: Final commit**

```bash
git add src/main.rs
git commit -m "feat: final integration, all systems wired"
```
