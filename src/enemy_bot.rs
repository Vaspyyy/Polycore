use crate::{
    constants,
    evolution::EvolutionState,
    hud::UpgradeState,
    menu::GamePhase,
    player::{self, Player, PlayerHealth, Velocity},
    projectile::{
        Lifetime, Projectile, ProjectileDamage, ProjectileKnockback, ProjectileOwner,
        ProjectilePenetration, ShootCooldown,
    },
    rng::Rng,
    shape::{Health, Shape},
};
use bevy::prelude::*;

const BOT_COUNT: usize = 5;
const BOT_BARREL_LENGTH: f32 = 30.0;
const BOT_BARREL_WIDTH: f32 = 7.0;
const BOT_BARREL_OVERLAP: f32 = 2.0;
const BOT_TURRET_SPIN_SPEED: f32 = 0.45;
const BOT_NAME_OFFSET_Y: f32 = 38.0;
const BOT_APPROACH_DISTANCE: f32 = 270.0;
const BOT_RETREAT_DISTANCE: f32 = 150.0;
const BOT_FIRE_RANGE: f32 = 520.0;
const BOT_VIEW_RANGE: f32 = 620.0;
const BOT_LOW_HEALTH_FRACTION: f32 = 0.35;
const BOT_SPAWN_MIN_DISTANCE: f32 = 220.0;

const BOT_NAMES: [&str; 12] = [
    "Scrapjaw", "Hex", "Rivet", "Bishop", "Torque", "Mako", "Vex", "Bolt", "Kilo", "Nyx", "Axle",
    "Cipher",
];

#[derive(Component)]
pub struct EnemyBot;

#[derive(Component)]
pub struct EnemyBotSceneEntity;

#[derive(Component)]
pub struct EnemyBotName(pub String);

#[derive(Component)]
pub struct EnemyBotUpgrades(pub UpgradeState);

#[derive(Component)]
pub struct EnemyBotEvolution(pub EvolutionState);

#[derive(Component)]
pub struct EnemyBotHealth {
    pub current: u32,
    pub max: u32,
}

#[derive(Component, Default)]
pub struct EnemyBotXp(pub u32);

#[derive(Component, Default)]
pub struct EnemyBotLevel(pub u32);

#[derive(Component, Default)]
pub struct EnemyBotVelocity(pub Vec2);

#[derive(Component, Default)]
pub struct EnemyBotMoveVelocity(pub Vec2);

#[derive(Component, Default)]
pub struct EnemyBotDamageCooldown(pub f32);

#[derive(Component, Default)]
pub struct EnemyBotHealProgress(pub f32);

#[derive(Component)]
pub struct EnemyBotSpawnPosition(pub Vec2);

#[derive(Component)]
pub struct EnemyBotTurret {
    center_distance: f32,
}

#[derive(Component)]
pub struct EnemyBotHealthBarBack;

#[derive(Component)]
pub struct EnemyBotHealthBarFill;

#[derive(Clone, Copy)]
struct CombatTarget {
    entity: Entity,
    position: Vec2,
}

pub fn setup_enemy_bots(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut rng: ResMut<Rng>,
) {
    let body_mesh = meshes.add(Circle::new(constants::PLAYER_RADIUS));
    let outline_mesh = meshes.add(Circle::new(
        constants::PLAYER_RADIUS + constants::OUTLINE_THICKNESS,
    ));
    let barrel_mesh = meshes.add(Rectangle::new(BOT_BARREL_WIDTH, BOT_BARREL_LENGTH));
    let barrel_outline_mesh = meshes.add(Rectangle::new(
        BOT_BARREL_WIDTH + constants::OUTLINE_THICKNESS * 2.0,
        BOT_BARREL_LENGTH + constants::OUTLINE_THICKNESS * 2.0,
    ));
    let health_bar_back_mesh = meshes.add(Rectangle::new(
        constants::HEALTH_BAR_WIDTH,
        constants::HEALTH_BAR_HEIGHT,
    ));
    let health_bar_fill_mesh = meshes.add(Rectangle::new(
        constants::HEALTH_BAR_WIDTH,
        constants::HEALTH_BAR_HEIGHT,
    ));
    let body_material = materials.add(Color::srgba(
        constants::ENEMY_COLOR[0],
        constants::ENEMY_COLOR[1],
        constants::ENEMY_COLOR[2],
        constants::ENEMY_COLOR[3],
    ));
    let outline_material = materials.add(Color::srgba(
        constants::OUTLINE_COLOR[0],
        constants::OUTLINE_COLOR[1],
        constants::OUTLINE_COLOR[2],
        constants::OUTLINE_COLOR[3],
    ));
    let barrel_material = materials.add(Color::srgba(
        constants::BARREL_COLOR[0],
        constants::BARREL_COLOR[1],
        constants::BARREL_COLOR[2],
        constants::BARREL_COLOR[3],
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
    let mut names = BOT_NAMES;
    let mut remaining_names = names.len();
    let mut occupied_positions = Vec::new();

    for _ in 0..BOT_COUNT {
        let upgrades = UpgradeState::default();
        let evolution = EvolutionState::default();
        let max_health = enemy_bot_max_health(&upgrades, &evolution);
        let position = random_enemy_bot_spawn_position(&mut rng, &occupied_positions, Vec2::ZERO);
        occupied_positions.push(position);
        let name_index = rng.next(remaining_names as u32) as usize;
        let bot_name = names[name_index];
        remaining_names -= 1;
        names.swap(name_index, remaining_names);

        commands
            .spawn((
                EnemyBot,
                EnemyBotSceneEntity,
                EnemyBotName(bot_name.to_string()),
                Mesh2d(body_mesh.clone()),
                MeshMaterial2d(body_material.clone()),
                Transform::from_xyz(position.x, position.y, 0.0),
                Visibility::Hidden,
            ))
            .insert((
                EnemyBotUpgrades(upgrades),
                EnemyBotEvolution(evolution),
                EnemyBotHealth {
                    current: max_health,
                    max: max_health,
                },
                EnemyBotXp::default(),
                EnemyBotLevel(1),
                EnemyBotVelocity::default(),
                EnemyBotMoveVelocity::default(),
                EnemyBotDamageCooldown::default(),
                EnemyBotHealProgress::default(),
                ShootCooldown(0.0),
                EnemyBotSpawnPosition(position),
            ))
            .with_children(|bot| {
                bot.spawn((
                    Mesh2d(outline_mesh.clone()),
                    MeshMaterial2d(outline_material.clone()),
                    Transform::from_xyz(0.0, 0.0, -0.2),
                ));
                spawn_turret_part(
                    bot,
                    barrel_outline_mesh.clone(),
                    outline_material.clone(),
                    true,
                );
                spawn_turret_part(bot, barrel_mesh.clone(), barrel_material.clone(), false);
                bot.spawn((
                    EnemyBotHealthBarBack,
                    Mesh2d(health_bar_back_mesh.clone()),
                    MeshMaterial2d(health_bar_back_material.clone()),
                    Transform::from_xyz(0.0, constants::HEALTH_BAR_OFFSET_Y, 2.0),
                    Visibility::Hidden,
                ));
                bot.spawn((
                    EnemyBotHealthBarFill,
                    Mesh2d(health_bar_fill_mesh.clone()),
                    MeshMaterial2d(health_bar_fill_material.clone()),
                    Transform::from_xyz(0.0, constants::HEALTH_BAR_OFFSET_Y, 3.0),
                    Visibility::Hidden,
                ));
                bot.spawn((
                    Text2d::new(bot_name),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    TextShadow {
                        offset: Vec2::new(1.5, 1.5),
                        color: Color::BLACK,
                    },
                    Transform::from_xyz(0.0, BOT_NAME_OFFSET_Y, 3.0),
                ));
            });
    }
}

pub fn enemy_bot_max_health(upgrades: &UpgradeState, evolution: &EvolutionState) -> u32 {
    (upgrades.max_health() as i32 + evolution.max_health_bonus()).max(40) as u32
}

pub fn refresh_enemy_bot_max_health(
    health: &mut EnemyBotHealth,
    upgrades: &UpgradeState,
    evolution: &EvolutionState,
) {
    let missing_health = health.max.saturating_sub(health.current);
    health.max = enemy_bot_max_health(upgrades, evolution);
    health.current = health.max.saturating_sub(missing_health);
}

pub fn award_enemy_bot_xp(
    xp_value: u32,
    xp: &mut EnemyBotXp,
    level: &mut EnemyBotLevel,
    upgrades: &mut EnemyBotUpgrades,
    evolution: &EnemyBotEvolution,
    health: &mut EnemyBotHealth,
    rng: &mut Rng,
) {
    xp.0 += xp_value;
    while xp.0 >= constants::XP_PER_LEVEL {
        xp.0 -= constants::XP_PER_LEVEL;
        level.0 += 1;
        upgrades.0.add_points(1);
    }

    let old_max = health.max;
    while upgrades.0.spend_random_point(rng) {}
    if health.max != enemy_bot_max_health(&upgrades.0, &evolution.0) || health.max != old_max {
        refresh_enemy_bot_max_health(health, &upgrades.0, &evolution.0);
    }
}

pub fn apply_enemy_bot_damage(health: &mut EnemyBotHealth, damage: u32) -> bool {
    let was_alive = health.current > 0;
    health.current = health.current.saturating_sub(damage);
    was_alive && health.current == 0
}

pub fn enemy_bot_ai_update(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    mut rng: ResMut<Rng>,
    mut bots: ParamSet<(
        Query<(Entity, &Transform, &EnemyBotHealth), (With<EnemyBot>, Without<EnemyBotTurret>)>,
        Query<
            (
                Entity,
                &mut Transform,
                &mut EnemyBotMoveVelocity,
                &mut EnemyBotVelocity,
                &mut EnemyBotDamageCooldown,
                &mut EnemyBotHealth,
                &mut EnemyBotHealProgress,
                &EnemyBotUpgrades,
                &EnemyBotEvolution,
                &mut ShootCooldown,
                &Children,
            ),
            (With<EnemyBot>, Without<EnemyBotTurret>),
        >,
    )>,
    player: Query<
        (Entity, &Transform, &PlayerHealth),
        (With<Player>, Without<EnemyBot>, Without<EnemyBotTurret>),
    >,
    shapes: Query<(&Transform, &Health), (With<Shape>, Without<EnemyBot>, Without<EnemyBotTurret>)>,
    mut turrets: Query<(&mut Transform, &EnemyBotTurret), Without<EnemyBot>>,
) {
    let dt = time.delta_secs();
    let half = constants::arena_half_extent() - constants::PLAYER_RADIUS;
    let damping = (1.0 - constants::PLAYER_KNOCKBACK_DAMPING * dt).clamp(0.0, 1.0);
    let projectile_mesh = meshes.add(Circle::new(constants::PROJECTILE_RADIUS));
    let projectile_material = materials.add(Color::srgba(
        constants::PROJECTILE_COLOR[0],
        constants::PROJECTILE_COLOR[1],
        constants::PROJECTILE_COLOR[2],
        constants::PROJECTILE_COLOR[3],
    ));

    let mut combat_targets: Vec<CombatTarget> = Vec::new();
    if let Ok((player_entity, player_transform, player_health)) = player.single()
        && player_health.current > 0
    {
        combat_targets.push(CombatTarget {
            entity: player_entity,
            position: player_transform.translation.xy(),
        });
    }
    combat_targets.extend(
        bots.p0()
            .iter()
            .filter(|(_, _, health)| health.current > 0)
            .map(|(entity, transform, _)| CombatTarget {
                entity,
                position: transform.translation.xy(),
            }),
    );

    for (
        bot_entity,
        mut transform,
        mut move_velocity,
        mut knockback_velocity,
        mut damage_cooldown,
        mut health,
        mut heal_progress,
        upgrades,
        evolution,
        mut shoot_cooldown,
        children,
    ) in bots.p1().iter_mut()
    {
        if health.current == 0 {
            move_velocity.0 = Vec2::ZERO;
            continue;
        }

        regenerate_enemy_bot_health(&mut health, &mut heal_progress, upgrades, evolution, dt);
        shoot_cooldown.0 -= dt;
        let bot_pos = transform.translation.xy();
        let target = nearest_combat_target(bot_entity, bot_pos, &combat_targets)
            .or_else(|| nearest_shape(bot_pos, &shapes));
        if let Some((target_pos, target_distance)) = target {
            aim_enemy_bot_turrets(children, bot_pos, target_pos, &mut turrets);
            update_enemy_bot_move_velocity(
                &mut move_velocity,
                bot_pos,
                target_pos,
                target_distance,
                &health,
                upgrades,
                evolution,
                dt,
            );

            if target_distance <= BOT_FIRE_RANGE && shoot_cooldown.0 <= 0.0 {
                shoot_cooldown.0 = upgrades.0.reload_cooldown() * evolution.0.reload_multiplier();
                shoot_enemy_bot_projectiles(
                    &mut commands,
                    &projectile_mesh,
                    &projectile_material,
                    bot_entity,
                    transform.translation,
                    target_pos,
                    upgrades,
                    evolution,
                    &mut rng,
                );
            }
        } else {
            move_velocity.0 =
                approach_velocity(move_velocity.0, Vec2::ZERO, constants::PLAYER_SPEED * dt);
            spin_enemy_bot_turrets(children, &mut turrets, dt);
        }

        transform.translation += (move_velocity.0 + knockback_velocity.0).extend(0.0) * dt;
        transform.translation.x = transform.translation.x.clamp(-half, half);
        transform.translation.y = transform.translation.y.clamp(-half, half);
        knockback_velocity.0 *= damping;
        damage_cooldown.0 = (damage_cooldown.0 - dt).max(0.0);
    }
}

fn regenerate_enemy_bot_health(
    health: &mut EnemyBotHealth,
    heal_progress: &mut EnemyBotHealProgress,
    upgrades: &EnemyBotUpgrades,
    evolution: &EnemyBotEvolution,
    dt: f32,
) {
    let regen_per_second = upgrades.0.health_regen_per_second() + evolution.0.health_regen_bonus();
    if regen_per_second <= 0.0 || health.current >= health.max {
        heal_progress.0 = 0.0;
        return;
    }

    heal_progress.0 += regen_per_second * dt;
    let heal_amount = heal_progress.0.floor() as u32;
    if heal_amount == 0 {
        return;
    }

    health.current = (health.current + heal_amount).min(health.max);
    heal_progress.0 -= heal_amount as f32;
}

fn nearest_shape(
    bot_pos: Vec2,
    shapes: &Query<
        (&Transform, &Health),
        (With<Shape>, Without<EnemyBot>, Without<EnemyBotTurret>),
    >,
) -> Option<(Vec2, f32)> {
    shapes
        .iter()
        .filter(|(_, health)| health.0 > 0)
        .map(|(transform, _)| {
            let shape_pos = transform.translation.xy();
            (shape_pos, bot_pos.distance(shape_pos))
        })
        .min_by(|(_, distance_a), (_, distance_b)| distance_a.total_cmp(distance_b))
}

fn nearest_combat_target(
    bot_entity: Entity,
    bot_pos: Vec2,
    targets: &[CombatTarget],
) -> Option<(Vec2, f32)> {
    targets
        .iter()
        .filter(|target| target.entity != bot_entity)
        .map(|target| (target.position, bot_pos.distance(target.position)))
        .filter(|(_, distance)| *distance <= BOT_VIEW_RANGE)
        .min_by(|(_, distance_a), (_, distance_b)| distance_a.total_cmp(distance_b))
}

fn update_enemy_bot_move_velocity(
    move_velocity: &mut EnemyBotMoveVelocity,
    bot_pos: Vec2,
    target_pos: Vec2,
    target_distance: f32,
    health: &EnemyBotHealth,
    upgrades: &EnemyBotUpgrades,
    evolution: &EnemyBotEvolution,
    dt: f32,
) {
    let direction = (target_pos - bot_pos).normalize_or_zero();
    let movement_speed = upgrades.0.movement_speed() * evolution.0.movement_multiplier();
    let health_fraction = health.current as f32 / health.max.max(1) as f32;
    let target_velocity = if health_fraction <= BOT_LOW_HEALTH_FRACTION {
        -direction * movement_speed
    } else if target_distance > BOT_APPROACH_DISTANCE {
        direction * movement_speed
    } else if target_distance < BOT_RETREAT_DISTANCE {
        -direction * movement_speed
    } else {
        Vec2::ZERO
    };
    let acceleration = movement_speed / constants::PLAYER_ACCEL_TIME;
    move_velocity.0 = approach_velocity(move_velocity.0, target_velocity, acceleration * dt);
}

fn approach_velocity(current: Vec2, target: Vec2, max_delta: f32) -> Vec2 {
    let delta = target - current;
    if delta.length_squared() <= max_delta * max_delta {
        target
    } else {
        current + delta.normalize_or_zero() * max_delta
    }
}

fn aim_enemy_bot_turrets(
    children: &Children,
    bot_pos: Vec2,
    target_pos: Vec2,
    turrets: &mut Query<(&mut Transform, &EnemyBotTurret), Without<EnemyBot>>,
) {
    let direction = (target_pos - bot_pos).normalize_or_zero();
    if direction.length_squared() <= 0.001 {
        return;
    }

    let angle = direction.y.atan2(direction.x);
    for child in children.iter() {
        let Ok((mut transform, turret)) = turrets.get_mut(child) else {
            continue;
        };
        transform.translation.x = direction.x * turret.center_distance;
        transform.translation.y = direction.y * turret.center_distance;
        transform.rotation = Quat::from_rotation_z(angle - std::f32::consts::FRAC_PI_2);
    }
}

fn spin_enemy_bot_turrets(
    children: &Children,
    turrets: &mut Query<(&mut Transform, &EnemyBotTurret), Without<EnemyBot>>,
    dt: f32,
) {
    let delta = BOT_TURRET_SPIN_SPEED * dt;
    for child in children.iter() {
        let Ok((mut transform, turret)) = turrets.get_mut(child) else {
            continue;
        };
        let current_angle = transform.translation.y.atan2(transform.translation.x) + delta;
        let direction = Vec2::from_angle(current_angle);
        transform.translation.x = direction.x * turret.center_distance;
        transform.translation.y = direction.y * turret.center_distance;
        transform.rotation = Quat::from_rotation_z(current_angle - std::f32::consts::FRAC_PI_2);
    }
}

fn shoot_enemy_bot_projectiles(
    commands: &mut Commands,
    projectile_mesh: &Handle<Mesh>,
    projectile_material: &Handle<ColorMaterial>,
    bot_entity: Entity,
    bot_translation: Vec3,
    target_pos: Vec2,
    upgrades: &EnemyBotUpgrades,
    evolution: &EnemyBotEvolution,
    rng: &mut Rng,
) {
    let base_angle = (target_pos - bot_translation.xy())
        .normalize_or_zero()
        .to_angle();
    let spread = evolution.0.spread_radians();
    let base_damage = upgrades.0.bullet_damage() as f32 * evolution.0.bullet_damage_multiplier();
    let bullet_speed = upgrades.0.bullet_speed() * evolution.0.bullet_speed_multiplier();
    let lifetime = constants::PROJECTILE_LIFETIME * evolution.0.projectile_lifetime_multiplier();
    let knockback = evolution.0.bullet_knockback_multiplier();

    for spec in evolution.0.barrel_specs() {
        let jitter = if spread > 0.0 {
            let roll = rng.next(10_000) as f32 / 9_999.0;
            (roll * 2.0 - 1.0) * spread
        } else {
            0.0
        };
        let shot_angle = base_angle + spec.angle_offset + jitter;
        let direction = Vec2::from_angle(shot_angle);
        let right = Vec2::new(direction.y, -direction.x);
        let spawn_pos = bot_translation
            + (direction * player::muzzle_projectile_distance(spec.length)
                + right * spec.lateral_offset)
                .extend(1.0);
        let damage = (base_damage * spec.damage_multiplier).round().max(1.0) as u32;

        commands.spawn((
            Projectile,
            ProjectileOwner::EnemyBot(bot_entity),
            Lifetime(lifetime),
            ProjectileDamage(damage),
            ProjectilePenetration(upgrades.0.bullet_penetration()),
            ProjectileKnockback(knockback),
            Mesh2d(projectile_mesh.clone()),
            MeshMaterial2d(projectile_material.clone()),
            Transform::from_translation(spawn_pos),
            Velocity(direction * bullet_speed),
        ));
    }
}

pub fn update_enemy_bot_health_bars(
    bots: Query<(&EnemyBotHealth, &Children), With<EnemyBot>>,
    mut bars: Query<
        (
            &mut Transform,
            &mut Visibility,
            Option<&EnemyBotHealthBarBack>,
            Option<&EnemyBotHealthBarFill>,
        ),
        Or<(With<EnemyBotHealthBarBack>, With<EnemyBotHealthBarFill>)>,
    >,
) {
    for (health, children) in bots.iter() {
        let is_damaged = health.current > 0 && health.current < health.max;
        let visibility = if is_damaged {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let health_fraction = (health.current as f32 / health.max as f32).clamp(0.0, 1.0);

        for child in children.iter() {
            let Ok((mut transform, mut bar_visibility, back, fill)) = bars.get_mut(child) else {
                continue;
            };

            *bar_visibility = visibility;
            if back.is_some() {
                transform.translation = Vec3::new(0.0, constants::HEALTH_BAR_OFFSET_Y, 2.0);
                transform.scale.x = 1.0;
            } else if fill.is_some() {
                transform.translation = Vec3::new(
                    -constants::HEALTH_BAR_WIDTH * (1.0 - health_fraction) / 2.0,
                    constants::HEALTH_BAR_OFFSET_Y,
                    3.0,
                );
                transform.scale.x = health_fraction;
            }
        }
    }
}

fn spawn_turret_part(
    bot: &mut ChildSpawnerCommands,
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
    outline: bool,
) {
    let center_distance = constants::PLAYER_RADIUS - BOT_BARREL_OVERLAP + BOT_BARREL_LENGTH / 2.0;
    let z = if outline { 0.2 } else { 0.4 };

    bot.spawn((
        EnemyBotTurret { center_distance },
        Mesh2d(mesh),
        MeshMaterial2d(material),
        Transform::from_xyz(0.0, center_distance, z),
    ));
}

pub fn sync_enemy_bot_visibility(
    phase: Res<GamePhase>,
    mut rng: ResMut<Rng>,
    player: Query<&Transform, (With<Player>, Without<EnemyBot>)>,
    mut bots: Query<
        (
            &mut Visibility,
            &mut Transform,
            &mut EnemyBotHealth,
            &mut EnemyBotVelocity,
            &mut EnemyBotMoveVelocity,
            &mut EnemyBotDamageCooldown,
            &mut EnemyBotHealProgress,
            &mut EnemyBotXp,
            &mut EnemyBotLevel,
            &mut EnemyBotUpgrades,
            &mut EnemyBotEvolution,
            &mut ShootCooldown,
            &mut EnemyBotSpawnPosition,
        ),
        (With<EnemyBotSceneEntity>, Without<Player>),
    >,
) {
    if !phase.is_changed() {
        return;
    }

    let player_pos = player
        .single()
        .map(|transform| transform.translation.xy())
        .unwrap_or(Vec2::ZERO);
    let mut occupied_positions = Vec::new();

    for (
        mut bot_visibility,
        mut transform,
        mut health,
        mut velocity,
        mut move_velocity,
        mut damage_cooldown,
        mut heal_progress,
        mut xp,
        mut level,
        mut upgrades,
        mut evolution,
        mut shoot_cooldown,
        mut spawn_position,
    ) in bots.iter_mut()
    {
        if *phase == GamePhase::Playing {
            let position =
                random_enemy_bot_spawn_position(&mut rng, &occupied_positions, player_pos);
            occupied_positions.push(position);
            spawn_position.0 = position;
            upgrades.0.reset();
            evolution.0.reset();
            health.max = enemy_bot_max_health(&upgrades.0, &evolution.0);
            health.current = health.max;
            xp.0 = 0;
            level.0 = 1;
            transform.translation = position.extend(0.0);
            velocity.0 = Vec2::ZERO;
            move_velocity.0 = Vec2::ZERO;
            damage_cooldown.0 = 0.0;
            heal_progress.0 = 0.0;
            shoot_cooldown.0 = 0.0;
            *bot_visibility = Visibility::Visible;
        } else {
            *bot_visibility = Visibility::Hidden;
        }
    }
}

fn random_enemy_bot_spawn_position(
    rng: &mut Rng,
    occupied_positions: &[Vec2],
    player_pos: Vec2,
) -> Vec2 {
    let half = constants::arena_half_extent() - constants::PLAYER_RADIUS;
    let min_distance_sq = BOT_SPAWN_MIN_DISTANCE * BOT_SPAWN_MIN_DISTANCE;

    for _ in 0..32 {
        let candidate = Vec2::new(random_arena_axis(rng, half), random_arena_axis(rng, half));
        if candidate.distance_squared(player_pos) < min_distance_sq {
            continue;
        }
        if occupied_positions
            .iter()
            .any(|position| candidate.distance_squared(*position) < min_distance_sq)
        {
            continue;
        }
        return candidate;
    }

    Vec2::new(random_arena_axis(rng, half), random_arena_axis(rng, half))
}

fn random_arena_axis(rng: &mut Rng, half: f32) -> f32 {
    rng.next((half * 2.0) as u32) as f32 - half
}
