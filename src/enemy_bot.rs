use crate::{
    combat::{CombatDeathQueue, CombatStats, CombatantId},
    constants,
    evolution::{self, BarrelSpec, EvolutionKind, EvolutionState},
    hud::{UpgradeKind, UpgradeState},
    menu::GamePhase,
    player::Player,
    projectile::ShootCooldown,
    rng::Rng,
};
use bevy::prelude::*;

pub const BOT_COUNT: usize = 5;
pub const BOT_BARREL_OVERLAP: f32 = 2.0;
pub const BOT_RESPAWN_DELAY: f32 = 4.0;
const BOT_NAME_OFFSET_Y: f32 = 38.0;
const BOT_SPAWN_MIN_DISTANCE: f32 = 220.0;

const BOT_NAMES: [&str; 12] = [
    "Scrapjaw", "Hex", "Rivet", "Bishop", "Torque", "Mako", "Vex", "Bolt", "Kilo", "Nyx", "Axle",
    "Cipher",
];

const BOT_PLAYSTYLES: [EnemyBotPlaystyle; BOT_COUNT] = [
    EnemyBotPlaystyle::Brawler,
    EnemyBotPlaystyle::Sharpshooter,
    EnemyBotPlaystyle::Juggernaut,
    EnemyBotPlaystyle::Sentinel,
    EnemyBotPlaystyle::Skirmisher,
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
    pub(crate) slot: usize,
    pub(crate) outline: bool,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyBotPlaystyle {
    Brawler,
    Sharpshooter,
    Juggernaut,
    Sentinel,
    Skirmisher,
}

impl EnemyBotPlaystyle {
    pub(crate) fn upgrade_weights(self) -> &'static [u32; 8] {
        match self {
            Self::Brawler => &[1, 3, 2, 4, 4, 10, 10, 5],
            Self::Sharpshooter => &[2, 3, 1, 10, 5, 9, 5, 5],
            Self::Juggernaut => &[4, 10, 10, 1, 2, 1, 2, 7],
            Self::Sentinel => &[10, 10, 2, 3, 4, 5, 5, 4],
            Self::Skirmisher => &[3, 4, 2, 6, 5, 7, 8, 10],
        }
    }

    pub(crate) fn preferred_evolution(self) -> EvolutionKind {
        match self {
            Self::Brawler => EvolutionKind::Sprayer,
            Self::Sharpshooter => EvolutionKind::Sniper,
            Self::Juggernaut => EvolutionKind::RamCore,
            Self::Sentinel => EvolutionKind::Guard,
            Self::Skirmisher => EvolutionKind::Flanker,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EnemyBotTargetKind {
    Combatant,
    Shape,
}

#[derive(Component, Debug)]
pub struct EnemyBotBrain {
    pub(crate) target: Option<Entity>,
    pub(crate) target_kind: EnemyBotTargetKind,
    pub(crate) decision_timer: f32,
    pub(crate) strafe_timer: f32,
    pub(crate) strafe_direction: f32,
    pub(crate) last_attacker: Option<Entity>,
    pub(crate) retaliation_timer: f32,
    pub(crate) engagement_timer: f32,
    pub(crate) truce_timer: f32,
    pub(crate) fleeing: bool,
    pub(crate) aim_angle: f32,
}

impl Default for EnemyBotBrain {
    fn default() -> Self {
        Self {
            target: None,
            target_kind: EnemyBotTargetKind::Combatant,
            decision_timer: 0.0,
            strafe_timer: 0.0,
            strafe_direction: 1.0,
            last_attacker: None,
            retaliation_timer: 0.0,
            engagement_timer: 0.0,
            truce_timer: 5.0,
            fleeing: false,
            aim_angle: std::f32::consts::FRAC_PI_2,
        }
    }
}

impl EnemyBotBrain {
    pub fn note_attacker(&mut self, attacker: Entity) {
        self.last_attacker = Some(attacker);
        self.retaliation_timer = 6.0;
        self.decision_timer = 0.0;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Component, Default)]
pub struct EnemyBotRespawnTimer(pub f32);

#[derive(Resource)]
pub struct EnemyBotResetPending(pub bool);

impl Default for EnemyBotResetPending {
    fn default() -> Self {
        Self(true)
    }
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
    let barrel_mesh = meshes.add(Rectangle::new(1.0, 1.0));
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

    for (bot_index, playstyle) in BOT_PLAYSTYLES.iter().copied().enumerate() {
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
                playstyle,
                EnemyBotBrain {
                    strafe_direction: if bot_index % 2 == 0 { 1.0 } else { -1.0 },
                    ..default()
                },
                EnemyBotRespawnTimer::default(),
                CombatStats::default(),
            ))
            .with_children(|bot| {
                bot.spawn((
                    Mesh2d(outline_mesh.clone()),
                    MeshMaterial2d(outline_material.clone()),
                    Transform::from_xyz(0.0, 0.0, -0.2),
                ));
                for slot in 0..evolution::MAX_BARRELS {
                    spawn_turret_part(
                        bot,
                        barrel_mesh.clone(),
                        outline_material.clone(),
                        slot,
                        true,
                    );
                    spawn_turret_part(
                        bot,
                        barrel_mesh.clone(),
                        barrel_material.clone(),
                        slot,
                        false,
                    );
                }
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
    let was_dead = health.current == 0;
    let missing_health = health.max.saturating_sub(health.current);
    health.max = enemy_bot_max_health(upgrades, evolution);
    health.current = if was_dead {
        0
    } else {
        health.max.saturating_sub(missing_health)
    };
}

#[allow(clippy::too_many_arguments)]
pub fn award_enemy_bot_xp(
    xp_value: u32,
    xp: &mut EnemyBotXp,
    level: &mut EnemyBotLevel,
    upgrades: &mut EnemyBotUpgrades,
    evolution: &mut EnemyBotEvolution,
    health: &mut EnemyBotHealth,
    playstyle: &EnemyBotPlaystyle,
    stats: &mut CombatStats,
    rng: &mut Rng,
) {
    xp.0 += xp_value;
    stats.score = stats.score.saturating_add(xp_value);
    while xp.0 >= constants::XP_PER_LEVEL {
        xp.0 -= constants::XP_PER_LEVEL;
        level.0 += 1;
        upgrades.0.add_points(1);
    }

    spend_adaptive_upgrade_points(upgrades, health, playstyle, rng);
    evolution
        .0
        .choose_kind_for_level(level.0, playstyle.preferred_evolution());
    if health.max != enemy_bot_max_health(&upgrades.0, &evolution.0) {
        refresh_enemy_bot_max_health(health, &upgrades.0, &evolution.0);
    }
}

fn spend_adaptive_upgrade_points(
    upgrades: &mut EnemyBotUpgrades,
    health: &EnemyBotHealth,
    playstyle: &EnemyBotPlaystyle,
    rng: &mut Rng,
) {
    let health_fraction = if health.current == 0 {
        1.0
    } else {
        health.current as f32 / health.max.max(1) as f32
    };
    let target_regen_level = if health_fraction <= 0.25 {
        3
    } else if health_fraction <= 0.50 {
        2
    } else if health_fraction <= 0.70 {
        1
    } else {
        0
    };

    while upgrades.0.points > 0 {
        if upgrades.0.level_of(UpgradeKind::HealthRegen) < target_regen_level
            && upgrades.0.spend_point_on(UpgradeKind::HealthRegen)
        {
            continue;
        }

        let mut weights = *playstyle.upgrade_weights();
        if health_fraction <= 0.50 {
            let regen = UpgradeKind::HealthRegen.index();
            let max_health = UpgradeKind::MaxHealth.index();
            let movement = UpgradeKind::MovementSpeed.index();
            weights[regen] = weights[regen].saturating_mul(4).max(24);
            weights[max_health] = weights[max_health].saturating_mul(2).saturating_add(6);
            weights[movement] = weights[movement].saturating_add(4);
        }

        if !upgrades.0.spend_weighted_point(rng, &weights) {
            break;
        }
    }
}

pub fn apply_enemy_bot_damage(health: &mut EnemyBotHealth, damage: u32) -> bool {
    let was_alive = health.current > 0;
    health.current = health.current.saturating_sub(damage);
    was_alive && health.current == 0
}

pub fn finish_enemy_bot_death(
    bot_entity: Entity,
    visibility: &mut Visibility,
    respawn_timer: &mut EnemyBotRespawnTimer,
    deaths: &mut CombatDeathQueue,
    killer: Option<CombatantId>,
) {
    *visibility = Visibility::Hidden;
    respawn_timer.0 = BOT_RESPAWN_DELAY;
    deaths.record(CombatantId::EnemyBot(bot_entity), killer);
}

pub(crate) fn enemy_bot_barrel_transform(
    spec: BarrelSpec,
    outline: bool,
    aim_angle: f32,
) -> Transform {
    let direction = Vec2::from_angle(aim_angle + spec.angle_offset);
    let right = Vec2::new(direction.y, -direction.x);
    let center_distance = constants::PLAYER_RADIUS - BOT_BARREL_OVERLAP + spec.length / 2.0;
    let center = direction * center_distance + right * spec.lateral_offset;
    let outline_growth = if outline {
        constants::OUTLINE_THICKNESS * 2.0
    } else {
        0.0
    };

    Transform {
        translation: Vec3::new(center.x, center.y, if outline { -0.2 } else { -0.1 }),
        rotation: Quat::from_rotation_z(
            aim_angle + spec.angle_offset - std::f32::consts::FRAC_PI_2,
        ),
        scale: Vec3::new(
            spec.width + outline_growth,
            spec.length + outline_growth,
            1.0,
        ),
    }
}

fn spawn_turret_part(
    bot: &mut ChildSpawnerCommands,
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
    slot: usize,
    outline: bool,
) {
    let default_evolution = EvolutionState::default();
    let spec = default_evolution.barrel_specs()[0];
    bot.spawn((
        EnemyBotTurret { slot, outline },
        Mesh2d(mesh),
        MeshMaterial2d(material),
        enemy_bot_barrel_transform(spec, outline, std::f32::consts::FRAC_PI_2),
        if slot == 0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
    ));
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
        let health_fraction = (health.current as f32 / health.max.max(1) as f32).clamp(0.0, 1.0);

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

pub fn sync_enemy_bot_visibility(
    phase: Res<GamePhase>,
    mut reset_pending: ResMut<EnemyBotResetPending>,
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
    mut bot_state: Query<
        (
            &mut EnemyBotRespawnTimer,
            &mut EnemyBotBrain,
            &mut CombatStats,
        ),
        With<EnemyBotSceneEntity>,
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
    let should_reset = *phase == GamePhase::Playing && reset_pending.0;

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
        if should_reset {
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
            transform.rotation = Quat::IDENTITY;
            velocity.0 = Vec2::ZERO;
            move_velocity.0 = Vec2::ZERO;
            damage_cooldown.0 = 0.0;
            heal_progress.0 = 0.0;
            shoot_cooldown.0 = 0.0;
            *bot_visibility = Visibility::Visible;
        } else if *phase == GamePhase::Playing {
            *bot_visibility = if health.current > 0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        } else {
            *bot_visibility = Visibility::Hidden;
        }
    }

    if should_reset {
        for (mut respawn_timer, mut brain, mut stats) in bot_state.iter_mut() {
            respawn_timer.0 = 0.0;
            brain.reset();
            *stats = CombatStats::default();
        }
    }
    if *phase == GamePhase::Playing {
        reset_pending.0 = false;
    }
}

pub(crate) fn random_enemy_bot_spawn_position(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bot_profiles_choose_distinct_evolutions() {
        let mut evolutions = BOT_PLAYSTYLES
            .iter()
            .copied()
            .map(EnemyBotPlaystyle::preferred_evolution)
            .collect::<Vec<_>>();
        evolutions.sort_by_key(|kind| *kind as u8);
        evolutions.dedup();

        assert_eq!(evolutions.len(), BOT_COUNT);
    }

    #[test]
    fn max_health_refresh_does_not_revive_dead_bot() {
        let mut health = EnemyBotHealth {
            current: 0,
            max: constants::PLAYER_MAX_HEALTH,
        };
        let mut upgrades = UpgradeState::default();
        upgrades.levels[1] = 2;

        refresh_enemy_bot_max_health(&mut health, &upgrades, &EvolutionState::default());

        assert_eq!(health.current, 0);
        assert_eq!(health.max, constants::PLAYER_MAX_HEALTH + 40);
    }

    #[test]
    fn low_health_bot_spends_its_next_point_on_regeneration() {
        let mut xp = EnemyBotXp::default();
        let mut level = EnemyBotLevel(1);
        let mut upgrades = EnemyBotUpgrades(UpgradeState::default());
        let mut evolution = EnemyBotEvolution(EvolutionState::default());
        let mut health = EnemyBotHealth {
            current: 10,
            max: constants::PLAYER_MAX_HEALTH,
        };
        let mut stats = CombatStats::default();
        let mut rng = Rng::new(17);

        award_enemy_bot_xp(
            constants::XP_PER_LEVEL,
            &mut xp,
            &mut level,
            &mut upgrades,
            &mut evolution,
            &mut health,
            &EnemyBotPlaystyle::Brawler,
            &mut stats,
            &mut rng,
        );

        assert_eq!(level.0, 2);
        assert_eq!(upgrades.0.level_of(UpgradeKind::HealthRegen), 1);
    }
}
