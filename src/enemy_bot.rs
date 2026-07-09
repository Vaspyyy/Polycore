use crate::{constants, evolution::EvolutionState, hud::UpgradeState, menu::GamePhase, rng::Rng};
use bevy::prelude::*;

const BOT_COUNT: usize = 5;
const BOT_BARREL_LENGTH: f32 = 30.0;
const BOT_BARREL_WIDTH: f32 = 7.0;
const BOT_BARREL_OVERLAP: f32 = 2.0;
const BOT_TURRET_SPIN_SPEED: f32 = 0.45;
const BOT_NAME_OFFSET_Y: f32 = 38.0;

const BOT_NAMES: [&str; 12] = [
    "Scrapjaw", "Hex", "Rivet", "Bishop", "Torque", "Mako", "Vex", "Bolt", "Kilo", "Nyx", "Axle",
    "Cipher",
];

const BOT_POSITIONS: [Vec2; BOT_COUNT] = [
    Vec2::new(-310.0, 220.0),
    Vec2::new(320.0, 190.0),
    Vec2::new(-280.0, -280.0),
    Vec2::new(270.0, -300.0),
    Vec2::new(20.0, 350.0),
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
pub struct EnemyBotVelocity(pub Vec2);

#[derive(Component, Default)]
pub struct EnemyBotDamageCooldown(pub f32);

#[derive(Component)]
pub struct EnemyBotSpawnPosition(pub Vec2);

#[derive(Component)]
pub struct EnemyBotTurret {
    angle: f32,
    center_distance: f32,
}

#[derive(Component)]
pub struct EnemyBotHealthBarBack;

#[derive(Component)]
pub struct EnemyBotHealthBarFill;

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

    for position in BOT_POSITIONS {
        let upgrades = UpgradeState::default();
        let evolution = EvolutionState::default();
        let max_health = enemy_bot_max_health(&upgrades, &evolution);
        let name_index = rng.next(remaining_names as u32) as usize;
        let bot_name = names[name_index];
        remaining_names -= 1;
        names.swap(name_index, remaining_names);

        commands
            .spawn((
                EnemyBot,
                EnemyBotSceneEntity,
                EnemyBotName(bot_name.to_string()),
                EnemyBotUpgrades(upgrades),
                EnemyBotEvolution(evolution),
                EnemyBotHealth {
                    current: max_health,
                    max: max_health,
                },
                EnemyBotVelocity::default(),
                EnemyBotDamageCooldown::default(),
                EnemyBotSpawnPosition(position),
                Mesh2d(body_mesh.clone()),
                MeshMaterial2d(body_material.clone()),
                Transform::from_xyz(position.x, position.y, 0.0),
                Visibility::Hidden,
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

pub fn apply_enemy_bot_damage(health: &mut EnemyBotHealth, damage: u32) -> bool {
    let was_alive = health.current > 0;
    health.current = health.current.saturating_sub(damage);
    was_alive && health.current == 0
}

pub fn enemy_bot_knockback_update(
    time: Res<Time>,
    mut bots: Query<
        (
            &mut Transform,
            &mut EnemyBotVelocity,
            &mut EnemyBotDamageCooldown,
        ),
        With<EnemyBot>,
    >,
) {
    let dt = time.delta_secs();
    let half = constants::arena_half_extent() - constants::PLAYER_RADIUS;
    let damping = (1.0 - constants::PLAYER_KNOCKBACK_DAMPING * dt).clamp(0.0, 1.0);

    for (mut transform, mut velocity, mut damage_cooldown) in bots.iter_mut() {
        transform.translation += velocity.0.extend(0.0) * dt;
        transform.translation.x = transform.translation.x.clamp(-half, half);
        transform.translation.y = transform.translation.y.clamp(-half, half);
        velocity.0 *= damping;
        damage_cooldown.0 = (damage_cooldown.0 - dt).max(0.0);
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
        EnemyBotTurret {
            angle: 0.0,
            center_distance,
        },
        Mesh2d(mesh),
        MeshMaterial2d(material),
        Transform::from_xyz(0.0, center_distance, z),
    ));
}

pub fn spin_enemy_bot_turrets(
    time: Res<Time>,
    mut turrets: Query<(&mut Transform, &mut EnemyBotTurret)>,
) {
    let delta = BOT_TURRET_SPIN_SPEED * time.delta_secs();
    for (mut transform, mut turret) in turrets.iter_mut() {
        turret.angle += delta;
        let direction = Vec2::from_angle(turret.angle);
        transform.translation.x = direction.x * turret.center_distance;
        transform.translation.y = direction.y * turret.center_distance;
        transform.rotation = Quat::from_rotation_z(turret.angle - std::f32::consts::FRAC_PI_2);
    }
}

pub fn sync_enemy_bot_visibility(
    phase: Res<GamePhase>,
    mut bots: Query<
        (
            &mut Visibility,
            &mut Transform,
            &mut EnemyBotHealth,
            &mut EnemyBotVelocity,
            &mut EnemyBotDamageCooldown,
            &mut EnemyBotUpgrades,
            &mut EnemyBotEvolution,
            &EnemyBotSpawnPosition,
        ),
        With<EnemyBotSceneEntity>,
    >,
) {
    if !phase.is_changed() {
        return;
    }

    for (
        mut bot_visibility,
        mut transform,
        mut health,
        mut velocity,
        mut damage_cooldown,
        mut upgrades,
        mut evolution,
        spawn_position,
    ) in bots.iter_mut()
    {
        if *phase == GamePhase::Playing {
            upgrades.0.reset();
            evolution.0.reset();
            health.max = enemy_bot_max_health(&upgrades.0, &evolution.0);
            health.current = health.max;
            transform.translation = spawn_position.0.extend(0.0);
            velocity.0 = Vec2::ZERO;
            damage_cooldown.0 = 0.0;
            *bot_visibility = Visibility::Visible;
        } else {
            *bot_visibility = Visibility::Hidden;
        }
    }
}
