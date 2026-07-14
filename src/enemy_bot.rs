use crate::{
    combat::{CombatDeathQueue, CombatStats, CombatantId},
    constants,
    evolution::{self, BarrelSpec, EvolutionKind, EvolutionState},
    hud::{UpgradeKind, UpgradeState},
    menu::GamePhase,
    palette::{BotPalette, PaletteMaterials},
    player::Player,
    projectile::ShootCooldown,
    rng::Rng,
    tank::{RecentDamage, SpawnProtection, TankOutline},
};
use bevy::prelude::*;

pub const BOT_COUNT: usize = 5;
pub const BOT_BARREL_OVERLAP: f32 = 2.0;
pub const BOT_RESPAWN_DELAY: f32 = 4.0;
const BOT_NAME_OFFSET_Y: f32 = 38.0;

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
    pub current: f32,
    pub max: f32,
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
    pub(crate) owner: Entity,
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
    pub fn hotspot_interest(self) -> f32 {
        match self {
            Self::Brawler => 1.15,
            Self::Sharpshooter => 0.90,
            Self::Juggernaut => 0.85,
            Self::Sentinel => 1.00,
            Self::Skirmisher => 1.10,
        }
    }

    pub(crate) fn upgrade_weights(self) -> &'static [u32; 8] {
        match self {
            Self::Brawler => &[1, 3, 2, 4, 4, 10, 10, 5],
            Self::Sharpshooter => &[2, 3, 1, 10, 5, 9, 5, 5],
            Self::Juggernaut => &[4, 10, 10, 1, 2, 1, 2, 7],
            Self::Sentinel => &[10, 10, 2, 3, 4, 5, 5, 4],
            Self::Skirmisher => &[3, 4, 2, 6, 5, 7, 8, 10],
        }
    }

    pub(crate) fn weighted_evolution(
        self,
        adaptation: Option<evolution::EvolutionTag>,
        rng: &mut Rng,
    ) -> EvolutionKind {
        if let Some(adaptation) = adaptation
            && rng.next(100) < 40
        {
            return match adaptation {
                evolution::EvolutionTag::Defense => EvolutionKind::Guard,
                evolution::EvolutionTag::Durability => EvolutionKind::RamCore,
                evolution::EvolutionTag::Mobility => EvolutionKind::Flanker,
                evolution::EvolutionTag::Precision => EvolutionKind::Sniper,
                evolution::EvolutionTag::AreaControl => EvolutionKind::Sprayer,
                evolution::EvolutionTag::Sustained => EvolutionKind::Gunner,
                evolution::EvolutionTag::Burst => EvolutionKind::Cannon,
            };
        }
        let (options, base_weights) = match self {
            Self::Brawler => (
                [
                    EvolutionKind::Sprayer,
                    EvolutionKind::TwinBarrel,
                    EvolutionKind::Cannon,
                ],
                [60, 25, 15],
            ),
            Self::Sharpshooter => (
                [
                    EvolutionKind::Sniper,
                    EvolutionKind::Cannon,
                    EvolutionKind::Gunner,
                ],
                [65, 25, 10],
            ),
            Self::Juggernaut => (
                [
                    EvolutionKind::RamCore,
                    EvolutionKind::Guard,
                    EvolutionKind::Cannon,
                ],
                [70, 20, 10],
            ),
            Self::Sentinel => (
                [
                    EvolutionKind::Guard,
                    EvolutionKind::Gunner,
                    EvolutionKind::Sniper,
                ],
                [65, 20, 15],
            ),
            Self::Skirmisher => (
                [
                    EvolutionKind::Flanker,
                    EvolutionKind::TwinBarrel,
                    EvolutionKind::Sprayer,
                ],
                [65, 25, 10],
            ),
        };
        let weights: [u32; 3] =
            std::array::from_fn(|index| {
                base_weights[index]
                    + 45 * u32::from(adaptation.is_some_and(|tag| {
                        evolution::definition(options[index]).tags.contains(&tag)
                    }))
            });
        let mut roll = rng.next(weights.iter().sum());
        for (kind, weight) in options.iter().copied().zip(weights) {
            if roll < weight {
                return kind;
            }
            roll -= weight;
        }
        options[0]
    }

    pub(crate) fn weighted_advanced_evolution(
        self,
        parent: EvolutionKind,
        adaptation: Option<evolution::EvolutionTag>,
        rng: &mut Rng,
    ) -> Option<EvolutionKind> {
        let options = evolution::advanced_kinds(parent)?;
        let preferred: &[evolution::EvolutionTag] = match self {
            Self::Brawler => &[
                evolution::EvolutionTag::AreaControl,
                evolution::EvolutionTag::Burst,
                evolution::EvolutionTag::Sustained,
            ],
            Self::Sharpshooter => &[
                evolution::EvolutionTag::Precision,
                evolution::EvolutionTag::Burst,
            ],
            Self::Juggernaut => &[
                evolution::EvolutionTag::Durability,
                evolution::EvolutionTag::Defense,
            ],
            Self::Sentinel => &[
                evolution::EvolutionTag::Defense,
                evolution::EvolutionTag::Precision,
            ],
            Self::Skirmisher => &[
                evolution::EvolutionTag::Mobility,
                evolution::EvolutionTag::Sustained,
            ],
        };
        let weights = options.map(|kind| {
            let definition = evolution::definition(kind);
            debug_assert!(kind.is_advanced());
            25 + 50 * u32::from(definition.tags.iter().any(|tag| preferred.contains(tag)))
                + 60 * u32::from(adaptation.is_some_and(|tag| definition.tags.contains(&tag)))
        });
        let roll = rng.next(weights[0] + weights[1]);
        Some(if roll < weights[0] {
            options[0]
        } else {
            options[1]
        })
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
    pub(crate) close_range_pressure: f32,
    pub(crate) long_range_pressure: f32,
    pub(crate) capstone_confirmation: f32,
    pub(crate) capstone_pending: bool,
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
            close_range_pressure: 0.0,
            long_range_pressure: 0.0,
            capstone_confirmation: 0.0,
            capstone_pending: false,
        }
    }
}

impl EnemyBotBrain {
    pub fn note_attacker(&mut self, attacker: Entity) {
        self.last_attacker = Some(attacker);
        self.retaliation_timer = 2.5;
        self.truce_timer = 0.0;
        self.decision_timer = 0.0;
        self.close_range_pressure = (self.close_range_pressure + 0.25).min(20.0);
    }

    pub fn note_projectile_attacker(&mut self, attacker: Entity, travel_distance: f32) {
        self.last_attacker = Some(attacker);
        self.retaliation_timer = 2.5;
        self.truce_timer = 0.0;
        self.decision_timer = 0.0;
        if travel_distance >= 350.0 {
            self.long_range_pressure = (self.long_range_pressure + 1.0).min(20.0);
        } else {
            self.close_range_pressure = (self.close_range_pressure + 1.0).min(20.0);
        }
    }

    pub fn adaptive_evolution_tag(&self) -> Option<evolution::EvolutionTag> {
        if self.long_range_pressure >= self.close_range_pressure + 1.0 {
            Some(evolution::EvolutionTag::Defense)
        } else if self.close_range_pressure >= self.long_range_pressure + 1.0 {
            Some(evolution::EvolutionTag::Durability)
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn reset_for_respawn(&mut self) {
        let close_range_pressure = self.close_range_pressure * 0.75;
        let long_range_pressure = self.long_range_pressure * 0.75;
        *self = Self {
            close_range_pressure,
            long_range_pressure,
            ..default()
        };
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
pub struct EnemyBotHealthBarBack {
    owner: Entity,
}

#[derive(Component)]
pub struct EnemyBotHealthBarFill {
    owner: Entity,
}

#[derive(Component)]
pub struct EnemyBotNameLabel {
    owner: Entity,
}

pub fn setup_enemy_bots(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut rng: ResMut<Rng>,
    palette_materials: Res<PaletteMaterials>,
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
    let outline_material = materials.add(Color::srgba(
        constants::OUTLINE_COLOR[0],
        constants::OUTLINE_COLOR[1],
        constants::OUTLINE_COLOR[2],
        constants::OUTLINE_COLOR[3],
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
    let mut occupied_positions = vec![Vec2::ZERO];

    for (bot_index, playstyle) in BOT_PLAYSTYLES.iter().copied().enumerate() {
        let bot_palette = palette_materials.bot(bot_index);
        let upgrades = UpgradeState::default();
        let evolution = EvolutionState::default();
        let max_health = enemy_bot_max_health(&upgrades, &evolution);
        let position = crate::tank::safe_spawn(&mut rng, &occupied_positions, &[], &[]);
        let health_bar_y = position.y + crate::tank::health_bar_offset(&evolution);
        occupied_positions.push(position);
        let name_index = rng.next(remaining_names as u32) as usize;
        let bot_name = names[name_index];
        remaining_names -= 1;
        names.swap(name_index, remaining_names);

        let bot_entity = commands
            .spawn((
                EnemyBot,
                EnemyBotSceneEntity,
                EnemyBotName(bot_name.to_string()),
                BotPalette(bot_index),
                Mesh2d(body_mesh.clone()),
                MeshMaterial2d(bot_palette.body.clone()),
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
                    decision_timer: bot_index as f32 * 0.025,
                    strafe_direction: if bot_index % 2 == 0 { 1.0 } else { -1.0 },
                    ..default()
                },
                EnemyBotRespawnTimer::default(),
                CombatStats::default(),
            ))
            .insert(crate::combat::LifeGeneration::default())
            .insert((SpawnProtection::default(), RecentDamage::default()))
            .insert(crate::passive::PassiveRuntime::default())
            .insert(crate::ability::ActiveAbilityState::default())
            .insert(crate::ability::Slowed::default())
            .id();

        commands.entity(bot_entity).with_children(|bot| {
            bot.spawn((
                TankOutline,
                Mesh2d(outline_mesh.clone()),
                MeshMaterial2d(outline_material.clone()),
                Transform::from_xyz(0.0, 0.0, -0.2),
            ));
        });

        commands.spawn((
            EnemyBotHealthBarBack { owner: bot_entity },
            Mesh2d(health_bar_back_mesh.clone()),
            MeshMaterial2d(health_bar_back_material.clone()),
            Transform::from_xyz(position.x, health_bar_y, 2.0),
            Visibility::Hidden,
        ));
        commands.spawn((
            EnemyBotHealthBarFill { owner: bot_entity },
            Mesh2d(health_bar_fill_mesh.clone()),
            MeshMaterial2d(health_bar_fill_material.clone()),
            Transform::from_xyz(position.x, health_bar_y, 3.0),
            Visibility::Hidden,
        ));

        for slot in 0..evolution::MAX_BARRELS {
            spawn_turret_part(
                &mut commands,
                bot_entity,
                position,
                barrel_mesh.clone(),
                outline_material.clone(),
                slot,
                true,
            );
            spawn_turret_part(
                &mut commands,
                bot_entity,
                position,
                barrel_mesh.clone(),
                bot_palette.barrel.clone(),
                slot,
                false,
            );
        }
        commands.spawn((
            EnemyBotNameLabel { owner: bot_entity },
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
            Transform::from_xyz(position.x, position.y + BOT_NAME_OFFSET_Y, 4.0),
            Visibility::Hidden,
        ));
    }
}

pub fn enemy_bot_max_health(upgrades: &UpgradeState, evolution: &EvolutionState) -> f32 {
    (upgrades.max_health() + evolution.max_health_bonus() as f32).max(40.0)
}

pub fn refresh_enemy_bot_max_health(
    health: &mut EnemyBotHealth,
    upgrades: &UpgradeState,
    evolution: &EvolutionState,
) {
    let was_dead = health.current <= 0.0;
    let missing_health = (health.max - health.current).max(0.0);
    health.max = enemy_bot_max_health(upgrades, evolution);
    health.current = if was_dead {
        0.0
    } else {
        (health.max - missing_health).max(1.0).min(health.max)
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
    adaptation: Option<evolution::EvolutionTag>,
    stats: &mut CombatStats,
    rng: &mut Rng,
) {
    award_enemy_bot_progress(
        xp_value, xp_value, xp, level, upgrades, evolution, health, playstyle, adaptation, stats,
        rng,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn award_enemy_bot_progress(
    xp_value: u32,
    score_value: u32,
    xp: &mut EnemyBotXp,
    level: &mut EnemyBotLevel,
    upgrades: &mut EnemyBotUpgrades,
    evolution: &mut EnemyBotEvolution,
    health: &mut EnemyBotHealth,
    playstyle: &EnemyBotPlaystyle,
    adaptation: Option<evolution::EvolutionTag>,
    stats: &mut CombatStats,
    rng: &mut Rng,
) {
    xp.0 += xp_value;
    stats.life_score = stats.life_score.saturating_add(score_value);
    let gained = constants::consume_level_ups(&mut xp.0, &mut level.0);
    upgrades.0.add_points(gained);

    spend_adaptive_upgrade_points(upgrades, health, playstyle, rng);
    if evolution.0.current_kind == EvolutionKind::Tank {
        evolution
            .0
            .choose_kind_for_level(level.0, playstyle.weighted_evolution(adaptation, rng));
    }
    if evolution.0.current_kind.is_level_five()
        && let Some(choice) =
            playstyle.weighted_advanced_evolution(evolution.0.current_kind, adaptation, rng)
    {
        evolution.0.choose_kind_for_level(level.0, choice);
    }
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
    let health_fraction = if health.current <= 0.0 {
        1.0
    } else {
        health.current / health.max.max(1.0)
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

pub fn apply_enemy_bot_damage(health: &mut EnemyBotHealth, damage: f32) -> bool {
    let was_alive = health.current > 0.0;
    health.current = (health.current - damage).max(0.0);
    was_alive && health.current <= 0.0
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
    evolution: &EvolutionState,
) -> Transform {
    let direction = Vec2::from_angle(aim_angle + spec.angle_offset);
    let right = Vec2::new(direction.y, -direction.x);
    let center_distance = crate::tank::radius(evolution) - BOT_BARREL_OVERLAP + spec.length / 2.0;
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
    commands: &mut Commands,
    owner: Entity,
    owner_position: Vec2,
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
    slot: usize,
    outline: bool,
) {
    let default_evolution = EvolutionState::default();
    let spec = default_evolution.barrel_specs()[0];
    let owner_transform = Transform::from_translation(owner_position.extend(0.0));
    commands.spawn((
        EnemyBotTurret {
            owner,
            slot,
            outline,
        },
        Mesh2d(mesh),
        MeshMaterial2d(material),
        owner_transform.mul_transform(enemy_bot_barrel_transform(
            spec,
            outline,
            std::f32::consts::FRAC_PI_2,
            &default_evolution,
        )),
        Visibility::Hidden,
    ));
}

pub fn update_enemy_bot_health_bars(
    phase: Res<GamePhase>,
    bots: Query<
        (&EnemyBotHealth, &EnemyBotEvolution, &Transform),
        (
            With<EnemyBot>,
            Without<EnemyBotHealthBarBack>,
            Without<EnemyBotHealthBarFill>,
        ),
    >,
    mut bars: Query<
        (
            &mut Transform,
            &mut Visibility,
            Option<&EnemyBotHealthBarBack>,
            Option<&EnemyBotHealthBarFill>,
        ),
        (
            Without<EnemyBot>,
            Or<(With<EnemyBotHealthBarBack>, With<EnemyBotHealthBarFill>)>,
        ),
    >,
) {
    let simulating = matches!(
        *phase,
        GamePhase::Playing | GamePhase::Paused | GamePhase::Dead
    );
    for (mut transform, mut visibility, back, fill) in &mut bars {
        let owner = back
            .map(|bar| bar.owner)
            .or_else(|| fill.map(|bar| bar.owner));
        let Some(owner) = owner else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let Ok((health, evolution, owner_transform)) = bots.get(owner) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let is_damaged = simulating && health.current > 0.0 && health.current < health.max;
        *visibility = if is_damaged {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let health_fraction = (health.current / health.max.max(1.0)).clamp(0.0, 1.0);
        let y = owner_transform.translation.y + crate::tank::health_bar_offset(&evolution.0);
        transform.rotation = Quat::IDENTITY;
        if back.is_some() {
            transform.translation = Vec3::new(owner_transform.translation.x, y, 2.0);
            transform.scale.x = 1.0;
        } else {
            transform.translation = Vec3::new(
                owner_transform.translation.x
                    - constants::HEALTH_BAR_WIDTH * (1.0 - health_fraction) / 2.0,
                y,
                3.0,
            );
            transform.scale.x = health_fraction;
        }
    }
}

pub fn sync_enemy_bot_visibility(
    phase: Res<GamePhase>,
    mut reset_pending: ResMut<EnemyBotResetPending>,
    mut shape_kills: ResMut<crate::dominance::PlayerShapeKills>,
    mut profile_tracker: ResMut<crate::dominance::ProfileProgressTracker>,
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
            &mut EnemyBotName,
            &mut SpawnProtection,
        ),
        (With<EnemyBotSceneEntity>, Without<Player>),
    >,
    mut bot_state: Query<
        (
            &mut EnemyBotRespawnTimer,
            &mut EnemyBotBrain,
            &mut CombatStats,
            &mut crate::passive::PassiveRuntime,
            &mut crate::combat::LifeGeneration,
        ),
        With<EnemyBotSceneEntity>,
    >,
) {
    if !phase.is_changed() && !reset_pending.0 {
        return;
    }

    let player_pos = player
        .single()
        .map(|transform| transform.translation.xy())
        .unwrap_or(Vec2::ZERO);
    let mut occupied_positions = vec![player_pos];
    let should_reset = reset_pending.0;
    let mut available_names = BOT_NAMES;
    let mut remaining_names = available_names.len();

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
        mut bot_name,
        mut protection,
    ) in bots.iter_mut()
    {
        if should_reset {
            let position = crate::tank::safe_spawn(&mut rng, &occupied_positions, &[], &[]);
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
            protection.remaining = constants::SPAWN_PROTECTION_SECS;
            let name_index = rng.next(remaining_names as u32) as usize;
            bot_name.0 = available_names[name_index].to_string();
            remaining_names -= 1;
            available_names.swap(name_index, remaining_names);
            *bot_visibility = if *phase == GamePhase::Playing {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        } else if matches!(
            *phase,
            GamePhase::Playing | GamePhase::Paused | GamePhase::Dead
        ) {
            *bot_visibility = if health.current > 0.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        } else {
            *bot_visibility = Visibility::Hidden;
        }
    }

    if should_reset {
        shape_kills.0 = 0;
        *profile_tracker = crate::dominance::ProfileProgressTracker::default();
        for (mut respawn_timer, mut brain, mut stats, mut passive_runtime, mut generation) in
            bot_state.iter_mut()
        {
            respawn_timer.0 = 0.0;
            brain.reset();
            *stats = CombatStats::default();
            passive_runtime.reset_for_life();
            generation.0 = generation.0.wrapping_add(1);
        }
    }
    if should_reset {
        reset_pending.0 = false;
    }
}

pub fn sync_enemy_bot_name_labels(
    phase: Res<GamePhase>,
    bots: Query<
        (
            &EnemyBotName,
            &Transform,
            &EnemyBotHealth,
            &EnemyBotEvolution,
        ),
        With<EnemyBot>,
    >,
    mut labels: Query<
        (
            &EnemyBotNameLabel,
            &mut Text2d,
            &mut Transform,
            &mut Visibility,
        ),
        (Without<EnemyBot>, Without<EnemyBotTurret>),
    >,
    mut turrets: Query<
        (&EnemyBotTurret, &mut Visibility),
        (Without<EnemyBot>, Without<EnemyBotNameLabel>),
    >,
) {
    let simulating = matches!(
        *phase,
        GamePhase::Playing | GamePhase::Paused | GamePhase::Dead
    );
    for (label, mut text, mut transform, mut visibility) in &mut labels {
        let Ok((name, owner_transform, health, evolution)) = bots.get(label.owner) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        if text.as_str() != name.0.as_str() {
            **text = name.0.clone();
        }
        transform.translation = Vec3::new(
            owner_transform.translation.x,
            owner_transform.translation.y + crate::tank::radius(&evolution.0) + 18.0,
            4.0,
        );
        transform.rotation = Quat::IDENTITY;
        *visibility = if simulating && health.current > 0.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (turret, mut visibility) in &mut turrets {
        if !simulating
            || bots
                .get(turret.owner)
                .map_or(true, |(_, _, health, _)| health.current <= 0.0)
        {
            *visibility = Visibility::Hidden;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bot_profiles_choose_distinct_evolutions() {
        let mut rng = Rng::new(5);
        for playstyle in BOT_PLAYSTYLES {
            let kind = playstyle.weighted_evolution(None, &mut rng);
            assert_ne!(kind, EvolutionKind::Tank);
        }
    }

    #[test]
    fn hotspot_interest_matches_playstyle_contract() {
        assert_eq!(EnemyBotPlaystyle::Brawler.hotspot_interest(), 1.15);
        assert_eq!(EnemyBotPlaystyle::Sharpshooter.hotspot_interest(), 0.90);
        assert_eq!(EnemyBotPlaystyle::Juggernaut.hotspot_interest(), 0.85);
        assert_eq!(EnemyBotPlaystyle::Sentinel.hotspot_interest(), 1.00);
        assert_eq!(EnemyBotPlaystyle::Skirmisher.hotspot_interest(), 1.10);
    }

    #[test]
    fn max_health_refresh_does_not_revive_dead_bot() {
        let mut health = EnemyBotHealth {
            current: 0.0,
            max: constants::PLAYER_MAX_HEALTH,
        };
        let mut upgrades = UpgradeState::default();
        upgrades.levels[1] = 2;

        refresh_enemy_bot_max_health(&mut health, &upgrades, &EvolutionState::default());

        assert_eq!(health.current, 0.0);
        assert_eq!(health.max, constants::PLAYER_MAX_HEALTH + 40.0);
    }

    #[test]
    fn max_health_reduction_keeps_a_living_bot_alive() {
        let mut health = EnemyBotHealth {
            current: 5.0,
            max: constants::PLAYER_MAX_HEALTH,
        };
        let evolution = EvolutionState {
            current_kind: EvolutionKind::Sniper,
            ..default()
        };

        refresh_enemy_bot_max_health(&mut health, &UpgradeState::default(), &evolution);

        assert_eq!(health.max, 40.0);
        assert_eq!(health.current, 1.0);
    }

    #[test]
    fn low_health_bot_spends_its_next_point_on_regeneration() {
        let mut xp = EnemyBotXp::default();
        let mut level = EnemyBotLevel(1);
        let mut upgrades = EnemyBotUpgrades(UpgradeState::default());
        let mut evolution = EnemyBotEvolution(EvolutionState::default());
        let mut health = EnemyBotHealth {
            current: 10.0,
            max: constants::PLAYER_MAX_HEALTH,
        };
        let mut stats = CombatStats::default();
        let mut rng = Rng::new(17);

        award_enemy_bot_xp(
            constants::xp_required_for_level(1),
            &mut xp,
            &mut level,
            &mut upgrades,
            &mut evolution,
            &mut health,
            &EnemyBotPlaystyle::Brawler,
            None,
            &mut stats,
            &mut rng,
        );

        assert_eq!(level.0, 2);
        assert_eq!(upgrades.0.level_of(UpgradeKind::HealthRegen), 1);
    }

    #[test]
    fn weighted_evolutions_stay_inside_each_playstyle_candidate_set() {
        let mut rng = Rng::new(77);
        for style in BOT_PLAYSTYLES {
            for _ in 0..200 {
                let choice = style.weighted_evolution(None, &mut rng);
                let allowed = match style {
                    EnemyBotPlaystyle::Brawler => [
                        EvolutionKind::Sprayer,
                        EvolutionKind::TwinBarrel,
                        EvolutionKind::Cannon,
                    ]
                    .contains(&choice),
                    EnemyBotPlaystyle::Sharpshooter => [
                        EvolutionKind::Sniper,
                        EvolutionKind::Cannon,
                        EvolutionKind::Gunner,
                    ]
                    .contains(&choice),
                    EnemyBotPlaystyle::Juggernaut => [
                        EvolutionKind::RamCore,
                        EvolutionKind::Guard,
                        EvolutionKind::Cannon,
                    ]
                    .contains(&choice),
                    EnemyBotPlaystyle::Sentinel => [
                        EvolutionKind::Guard,
                        EvolutionKind::Gunner,
                        EvolutionKind::Sniper,
                    ]
                    .contains(&choice),
                    EnemyBotPlaystyle::Skirmisher => [
                        EvolutionKind::Flanker,
                        EvolutionKind::TwinBarrel,
                        EvolutionKind::Sprayer,
                    ]
                    .contains(&choice),
                };
                assert!(allowed);
            }
        }
    }

    #[test]
    fn bot_adaptation_tracks_attack_range_across_respawns() {
        let attacker = Entity::from_bits(99);
        let mut brain = EnemyBotBrain::default();
        brain.note_projectile_attacker(attacker, 500.0);
        assert_eq!(
            brain.adaptive_evolution_tag(),
            Some(evolution::EvolutionTag::Defense)
        );
        brain.reset_for_respawn();
        assert!(brain.long_range_pressure > 0.0);

        for _ in 0..3 {
            brain.note_projectile_attacker(attacker, 50.0);
        }
        assert_eq!(
            brain.adaptive_evolution_tag(),
            Some(evolution::EvolutionTag::Durability)
        );
    }

    #[test]
    fn adaptive_counter_pick_can_override_static_playstyle_candidates() {
        let mut rng = Rng::new(31);
        let picked_guard = (0..100).any(|_| {
            EnemyBotPlaystyle::Brawler
                .weighted_evolution(Some(evolution::EvolutionTag::Defense), &mut rng)
                == EvolutionKind::Guard
        });
        assert!(picked_guard);
    }

    #[test]
    fn base_player_and_bot_stats_use_identical_rules() {
        let upgrades = UpgradeState::default();
        let evolution = EvolutionState::default();
        assert_eq!(
            enemy_bot_max_health(&upgrades, &evolution),
            upgrades.max_health()
        );
        assert_eq!(
            crate::tank::body_damage(upgrades.body_damage(), &evolution),
            constants::BASE_BODY_DAMAGE,
        );
        assert_eq!(upgrades.bullet_damage(), constants::BASE_PROJECTILE_DAMAGE);
    }

    #[test]
    fn turret_world_transform_follows_bot_owner() {
        let evolution = EvolutionState::default();
        let spec = evolution.barrel_specs()[0];
        let owner = Transform::from_xyz(-210.0, 95.0, 0.0);
        let local =
            enemy_bot_barrel_transform(spec, false, std::f32::consts::FRAC_PI_4, &evolution);
        let world = owner.mul_transform(local);

        assert!(
            world
                .translation
                .truncate()
                .distance(owner.transform_point(local.translation).truncate())
                < 0.001
        );
        assert!(
            world
                .translation
                .truncate()
                .distance(owner.translation.truncate())
                > constants::PLAYER_RADIUS
        );
    }

    #[test]
    fn bot_visual_system_queries_are_disjoint() {
        let mut app = App::new();
        app.insert_resource(GamePhase::Playing)
            .add_systems(Update, sync_enemy_bot_name_labels);

        let bot = app
            .world_mut()
            .spawn((
                EnemyBot,
                EnemyBotName("Bot".to_string()),
                EnemyBotHealth {
                    current: 50.0,
                    max: 50.0,
                },
                EnemyBotEvolution(EvolutionState::default()),
                Transform::default(),
            ))
            .id();
        app.world_mut().spawn((
            EnemyBotNameLabel { owner: bot },
            Text2d::default(),
            Transform::default(),
            Visibility::Hidden,
        ));
        app.world_mut().spawn((
            EnemyBotTurret {
                owner: bot,
                slot: 0,
                outline: false,
            },
            Visibility::Hidden,
        ));

        app.update();
    }

    #[test]
    fn damaged_bot_health_bars_follow_owner_and_hide_at_full_health() {
        let mut app = App::new();
        app.insert_resource(GamePhase::Playing)
            .add_systems(Update, update_enemy_bot_health_bars);

        let bot = app
            .world_mut()
            .spawn((
                EnemyBot,
                EnemyBotHealth {
                    current: 25.0,
                    max: 50.0,
                },
                EnemyBotEvolution(EvolutionState::default()),
                Transform::from_xyz(100.0, 50.0, 0.0),
            ))
            .id();
        let back = app
            .world_mut()
            .spawn((
                EnemyBotHealthBarBack { owner: bot },
                Transform::default(),
                Visibility::Hidden,
            ))
            .id();
        let fill = app
            .world_mut()
            .spawn((
                EnemyBotHealthBarFill { owner: bot },
                Transform::default(),
                Visibility::Hidden,
            ))
            .id();

        app.update();

        assert_eq!(
            app.world().entity(back).get::<Visibility>(),
            Some(&Visibility::Visible)
        );
        assert_eq!(
            app.world().entity(fill).get::<Visibility>(),
            Some(&Visibility::Visible)
        );
        assert_eq!(
            app.world().entity(fill).get::<Transform>().unwrap().scale.x,
            0.5
        );

        app.world_mut()
            .entity_mut(bot)
            .get_mut::<EnemyBotHealth>()
            .unwrap()
            .current = 50.0;
        app.update();

        assert_eq!(
            app.world().entity(back).get::<Visibility>(),
            Some(&Visibility::Hidden)
        );
        assert_eq!(
            app.world().entity(fill).get::<Visibility>(),
            Some(&Visibility::Hidden)
        );
    }
}
