use crate::{
    combat::{CombatDeathQueue, CombatantId, LifeGeneration},
    constants,
    enemy_bot::{
        EnemyBot, EnemyBotBrain, EnemyBotEvolution, EnemyBotHealth, EnemyBotMoveVelocity,
        EnemyBotUpgrades, EnemyBotVelocity,
    },
    evolution::{EvolutionKind, EvolutionState},
    hud::UpgradeState,
    palette::PaletteMaterials,
    player::{MoveVelocity, Player, PlayerHealth, Velocity},
    profile::Profile,
    projectile::{
        Lifetime, Projectile, ProjectileAssets, ProjectileDamage, ProjectileEvolution,
        ProjectileGeneration, ProjectileHitHistory, ProjectileKnockback, ProjectileOwner,
        ProjectilePenetration, ProjectileRadius, ProjectileRear, ProjectileSplashReady,
        ProjectileTravel,
    },
    rng::Rng,
};
use bevy::prelude::*;

const CONSTRUCT_BAR_WIDTH: f32 = 52.0;
const CONSTRUCT_BAR_HEIGHT: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveAbilityKind {
    GunPod,
    Brace,
    Airburst,
    PiercingLine,
    FullBattery,
    Counterburst,
    Rangefinder,
    HunterMine,
    RammingSpeed,
    Intercept,
    Saturation,
    PinningBurst,
    Fortify,
    ShieldWall,
    Burnout,
    CombatRoll,
}

impl ActiveAbilityKind {
    pub fn from_evolution(kind: EvolutionKind) -> Option<Self> {
        Some(match kind {
            EvolutionKind::Sentry => Self::GunPod,
            EvolutionKind::Emplacement => Self::Brace,
            EvolutionKind::Siegebreaker => Self::Airburst,
            EvolutionKind::Lancer => Self::PiercingLine,
            EvolutionKind::Fusillade => Self::FullBattery,
            EvolutionKind::Rearguard => Self::Counterburst,
            EvolutionKind::Deadeye => Self::Rangefinder,
            EvolutionKind::Pursuer => Self::HunterMine,
            EvolutionKind::Dreadnought => Self::RammingSpeed,
            EvolutionKind::Vanguard => Self::Intercept,
            EvolutionKind::Bombardier => Self::Saturation,
            EvolutionKind::Impaler => Self::PinningBurst,
            EvolutionKind::Stronghold => Self::Fortify,
            EvolutionKind::Guardian => Self::ShieldWall,
            EvolutionKind::Afterburner => Self::Burnout,
            EvolutionKind::Ace => Self::CombatRoll,
            _ => return None,
        })
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::GunPod => "Gun Pod",
            Self::Brace => "Brace",
            Self::Airburst => "Airburst",
            Self::PiercingLine => "Piercing Line",
            Self::FullBattery => "Full Battery",
            Self::Counterburst => "Counterburst",
            Self::Rangefinder => "Rangefinder",
            Self::HunterMine => "Hunter Mine",
            Self::RammingSpeed => "Ramming Speed",
            Self::Intercept => "Intercept",
            Self::Saturation => "Saturation",
            Self::PinningBurst => "Pinning Burst",
            Self::Fortify => "Fortify",
            Self::ShieldWall => "Shield Wall",
            Self::Burnout => "Burnout",
            Self::CombatRoll => "Combat Roll",
        }
    }

    pub const fn cooldown(self) -> f32 {
        match self {
            Self::GunPod => 18.0,
            Self::Brace => 14.0,
            Self::Airburst => 14.0,
            Self::PiercingLine => 14.0,
            Self::FullBattery => 14.0,
            Self::Counterburst => 16.0,
            Self::Rangefinder => 12.0,
            Self::HunterMine => 10.0,
            Self::RammingSpeed => 16.0,
            Self::Intercept => 12.0,
            Self::Saturation => 15.0,
            Self::PinningBurst => 12.0,
            Self::Fortify => 18.0,
            Self::ShieldWall => 16.0,
            Self::Burnout | Self::CombatRoll => 10.0,
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct ActiveAbilityState {
    pub kind: Option<ActiveAbilityKind>,
    pub cooldown_remaining: f32,
    pub active_remaining: f32,
    pub prime_remaining: f32,
    pub charges: u8,
    pub decision_delay: f32,
    pub decision_armed: bool,
    pub bonus_shield: f32,
}

impl Default for ActiveAbilityState {
    fn default() -> Self {
        Self {
            kind: None,
            cooldown_remaining: 0.0,
            active_remaining: 0.0,
            prime_remaining: 0.0,
            charges: 0,
            decision_delay: 0.0,
            decision_armed: false,
            bonus_shield: 0.0,
        }
    }
}

impl ActiveAbilityState {
    pub fn reset_for_life(&mut self) {
        *self = Self::default();
    }

    pub fn sync_evolution(&mut self, evolution: EvolutionKind) {
        let next = ActiveAbilityKind::from_evolution(evolution);
        if self.kind != next {
            *self = Self {
                kind: next,
                ..default()
            };
        }
    }

    pub fn movement_multiplier(&self) -> f32 {
        match (self.kind, self.active_remaining > 0.0) {
            (Some(ActiveAbilityKind::Brace), true) => 0.0,
            (Some(ActiveAbilityKind::RammingSpeed), true) => 1.70,
            (Some(ActiveAbilityKind::Burnout), true) => 2.0,
            (Some(ActiveAbilityKind::CombatRoll), true) => 1.25,
            _ => 1.0,
        }
    }

    pub fn damage_multiplier(&self) -> f32 {
        match (self.kind, self.active_remaining > 0.0) {
            (Some(ActiveAbilityKind::Brace), true) => 0.75,
            (Some(ActiveAbilityKind::RammingSpeed), true) => 0.70,
            _ => 1.0,
        }
    }

    pub fn firing_disabled(&self) -> bool {
        self.kind == Some(ActiveAbilityKind::RammingSpeed) && self.active_remaining > 0.0
    }

    pub fn forces_forward_movement(&self) -> bool {
        self.active_remaining > 0.0
            && matches!(
                self.kind,
                Some(ActiveAbilityKind::RammingSpeed) | Some(ActiveAbilityKind::Burnout)
            )
    }

    pub fn limited_turning(&self) -> bool {
        self.kind == Some(ActiveAbilityKind::RammingSpeed) && self.active_remaining > 0.0
    }

    pub fn reload_multiplier(&self) -> f32 {
        if self.kind == Some(ActiveAbilityKind::Brace) && self.active_remaining > 0.0 {
            0.80
        } else {
            1.0
        }
    }

    pub fn body_damage_multiplier(&self) -> f32 {
        if self.kind == Some(ActiveAbilityKind::RammingSpeed) && self.active_remaining > 0.0 {
            1.35
        } else {
            1.0
        }
    }

    pub fn braced(&self) -> bool {
        self.kind == Some(ActiveAbilityKind::Brace) && self.active_remaining > 0.0
    }

    pub fn absorb_shield_wall(&mut self, damage: f32, frontal: bool) -> f32 {
        if !frontal
            || self.kind != Some(ActiveAbilityKind::ShieldWall)
            || self.active_remaining <= 0.0
        {
            return damage;
        }
        let absorbed = damage.min(self.bonus_shield);
        self.bonus_shield -= absorbed;
        damage - absorbed
    }

    pub fn shield_wall_active(&self) -> bool {
        self.kind == Some(ActiveAbilityKind::ShieldWall) && self.active_remaining > 0.0
    }

    pub fn full_battery(&mut self) -> bool {
        if self.kind == Some(ActiveAbilityKind::FullBattery)
            && self.prime_remaining > 0.0
            && self.charges > 0
        {
            self.charges -= 1;
            if self.charges == 0 {
                self.prime_remaining = 0.0;
            }
            true
        } else {
            false
        }
    }

    pub fn primed_shot(&mut self) -> PrimedShot {
        if self.prime_remaining <= 0.0 || self.charges == 0 {
            return PrimedShot::default();
        }
        let modifier = match self.kind {
            Some(ActiveAbilityKind::PiercingLine) => PrimedShot {
                damage: 1.15,
                speed: 1.50,
                lifetime: 1.30,
                penetration: 4,
                clears_projectiles: true,
                pinning: false,
            },
            Some(ActiveAbilityKind::Rangefinder) => PrimedShot {
                damage: 1.15,
                speed: 1.50,
                lifetime: 1.50,
                penetration: 2,
                clears_projectiles: false,
                pinning: false,
            },
            Some(ActiveAbilityKind::PinningBurst) => PrimedShot {
                pinning: true,
                ..default()
            },
            _ => return PrimedShot::default(),
        };
        if !modifier.pinning {
            self.charges -= 1;
            if self.charges == 0 {
                self.prime_remaining = 0.0;
            }
        }
        modifier
    }

    pub fn consume_pinning_hit(&mut self) -> bool {
        if self.kind != Some(ActiveAbilityKind::PinningBurst)
            || self.prime_remaining <= 0.0
            || self.charges == 0
        {
            return false;
        }
        self.charges -= 1;
        if self.charges == 0 {
            self.prime_remaining = 0.0;
        }
        true
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PrimedShot {
    pub damage: f32,
    pub speed: f32,
    pub lifetime: f32,
    pub penetration: u32,
    pub clears_projectiles: bool,
    pub pinning: bool,
}

impl Default for PrimedShot {
    fn default() -> Self {
        Self {
            damage: 1.0,
            speed: 1.0,
            lifetime: 1.0,
            penetration: 0,
            clears_projectiles: false,
            pinning: false,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ProjectileAbility {
    pub clears_projectiles: bool,
    pub pinning: bool,
    pub reflected: bool,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Slowed {
    pub amount: f32,
    pub remaining: f32,
}

impl Slowed {
    pub fn movement_multiplier(&self) -> f32 {
        1.0 - self.amount.clamp(0.0, 0.30)
    }
}

#[derive(Message, Clone, Copy, Debug)]
pub struct AbilityCast {
    pub actor: Entity,
    pub aim: Vec2,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstructKind {
    Turret,
    Mine,
    Fortification,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Construct {
    pub kind: ConstructKind,
    pub owner: ProjectileOwner,
    pub generation: u32,
    pub health: f32,
    pub max_health: f32,
    pub remaining: f32,
    pub duration: f32,
    pub damage: f32,
    pub projectile_speed: f32,
    pub range: f32,
    pub fire_timer: f32,
}

#[derive(Component)]
pub struct ConstructHealthFill;

#[derive(Component)]
pub struct AbilityCooldownRing {
    owner: Entity,
}

#[derive(Component)]
pub struct AbilityAimLine {
    owner: Entity,
}

#[derive(Component)]
pub struct AbilityPanel;

#[derive(Component)]
pub struct AbilityPanelText;

#[derive(Resource, Clone)]
pub struct AbilityAssets {
    ring_mesh: Handle<Mesh>,
    turret_mesh: Handle<Mesh>,
    mine_mesh: Handle<Mesh>,
    wall_mesh: Handle<Mesh>,
    telegraph_mesh: Handle<Mesh>,
    aim_line_mesh: Handle<Mesh>,
    accent: Handle<ColorMaterial>,
    hostile_accent: Handle<ColorMaterial>,
    bar_mesh: Handle<Mesh>,
    bar_background: Handle<ColorMaterial>,
}

#[derive(Component, Clone)]
pub(crate) struct PendingAbilityEffect {
    actor: Entity,
    owner: ProjectileOwner,
    generation: u32,
    kind: ActiveAbilityKind,
    remaining: f32,
    position: Vec2,
    direction: Vec2,
    damage: f32,
    projectile_speed: f32,
    projectile_lifetime: f32,
    projectile_radius: f32,
    projectile_material: Handle<ColorMaterial>,
    repeats: u8,
}

pub fn setup_abilities(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let accent = materials.add(Color::srgba(0.18, 0.92, 1.0, 0.82));
    let hostile_accent = materials.add(Color::srgba(1.0, 0.30, 0.42, 0.82));
    commands.insert_resource(AbilityAssets {
        ring_mesh: meshes.add(Annulus::new(32.0, 35.0)),
        turret_mesh: meshes.add(RegularPolygon::new(18.0, 6)),
        mine_mesh: meshes.add(RegularPolygon::new(14.0, 8)),
        wall_mesh: meshes.add(Rectangle::new(110.0, 18.0)),
        telegraph_mesh: meshes.add(Annulus::new(145.0, 150.0)),
        aim_line_mesh: meshes.add(Rectangle::new(1.5, 900.0)),
        accent: accent.clone(),
        hostile_accent,
        bar_mesh: meshes.add(Rectangle::new(CONSTRUCT_BAR_WIDTH, CONSTRUCT_BAR_HEIGHT)),
        bar_background: materials.add(Color::srgba(0.06, 0.07, 0.09, 0.92)),
    });

    commands
        .spawn((
            AbilityPanel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                bottom: Val::Px(132.0),
                width: Val::Px(230.0),
                height: Val::Px(38.0),
                margin: UiRect::left(Val::Px(-115.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.07, 0.11, 0.88)),
            BorderColor::all(Color::srgba(0.18, 0.92, 1.0, 0.72)),
            Visibility::Hidden,
        ))
        .with_children(|panel| {
            panel.spawn((
                AbilityPanelText,
                Text::new("Ability ready"),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

pub fn player_ability_input(
    mouse: Res<ButtonInput<MouseButton>>,
    player: Query<(Entity, &Transform, &PlayerHealth), With<Player>>,
    mut casts: MessageWriter<AbilityCast>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok((actor, transform, health)) = player.single() else {
        return;
    };
    if health.current <= 0.0 {
        return;
    }
    casts.write(AbilityCast {
        actor,
        aim: (transform.rotation * Vec3::Y).xy().normalize_or_zero(),
    });
}

pub fn bot_ability_decisions(
    time: Res<Time>,
    mut rng: ResMut<Rng>,
    mut bots: Query<(
        Entity,
        &Transform,
        &EnemyBotHealth,
        &EnemyBotBrain,
        &EnemyBotEvolution,
        &mut ActiveAbilityState,
    )>,
    targets: Query<&Transform>,
    projectiles: Query<(&Transform, &ProjectileOwner, &ProjectileEvolution), With<Projectile>>,
    mut casts: MessageWriter<AbilityCast>,
) {
    let dt = time.delta_secs();
    for (entity, transform, health, brain, evolution, mut state) in &mut bots {
        state.sync_evolution(evolution.0.current_kind);
        if state.kind.is_none() || state.cooldown_remaining > 0.0 || health.current <= 0.0 {
            continue;
        }
        state.decision_delay -= dt;
        let target = brain.target.and_then(|target| targets.get(target).ok());
        let target_distance = target.map_or(f32::INFINITY, |target| {
            transform.translation.distance(target.translation)
        });
        let threatened = projectiles.iter().any(|(shot, owner, _)| {
            *owner != ProjectileOwner::EnemyBot(entity)
                && shot.translation.distance(transform.translation) <= 180.0
        });
        let has_live_shell = projectiles.iter().any(|(_, owner, evolution)| {
            *owner == ProjectileOwner::EnemyBot(entity)
                && matches!(
                    evolution.0,
                    EvolutionKind::Annihilator | EvolutionKind::Siegebreaker
                )
        });
        let health_fraction = health.current / health.max.max(1.0);
        let should_cast = bot_should_cast(
            state.kind.unwrap(),
            target_distance,
            threatened,
            health_fraction,
            has_live_shell,
        );
        if !should_cast {
            state.decision_armed = false;
            continue;
        }
        if !state.decision_armed {
            state.decision_armed = true;
            state.decision_delay = 0.2 + rng.next(401) as f32 / 1_000.0;
            continue;
        }
        if state.decision_delay > 0.0 {
            continue;
        }
        let aim = (transform.rotation * Vec3::Y)
            .xy()
            .normalize_or(Vec2::from_angle(brain.aim_angle));
        casts.write(AbilityCast { actor: entity, aim });
        state.decision_delay = 0.0;
        state.decision_armed = false;
    }
}

fn bot_should_cast(
    kind: ActiveAbilityKind,
    target_distance: f32,
    threatened: bool,
    health_fraction: f32,
    has_live_shell: bool,
) -> bool {
    match kind {
        ActiveAbilityKind::GunPod | ActiveAbilityKind::Fortify => target_distance < 700.0,
        ActiveAbilityKind::HunterMine => target_distance < 260.0,
        ActiveAbilityKind::Brace => target_distance < 650.0 && health_fraction < 0.82,
        ActiveAbilityKind::Airburst => has_live_shell && target_distance < 650.0,
        ActiveAbilityKind::PiercingLine
        | ActiveAbilityKind::Rangefinder
        | ActiveAbilityKind::FullBattery
        | ActiveAbilityKind::PinningBurst => target_distance < 900.0,
        ActiveAbilityKind::Counterburst
        | ActiveAbilityKind::Intercept
        | ActiveAbilityKind::ShieldWall => threatened || target_distance < 190.0,
        ActiveAbilityKind::RammingSpeed
        | ActiveAbilityKind::Saturation
        | ActiveAbilityKind::Burnout => target_distance < 340.0,
        ActiveAbilityKind::CombatRoll => threatened || target_distance < 260.0,
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn execute_ability_casts(
    mut commands: Commands,
    mut casts: MessageReader<AbilityCast>,
    assets: Res<AbilityAssets>,
    palettes: Res<PaletteMaterials>,
    profile: Res<Profile>,
    player_evolution: Res<EvolutionState>,
    player_upgrades: Res<UpgradeState>,
    constructs: Query<&Construct>,
    mut actors: Query<
        (
            Entity,
            &mut Transform,
            &mut ActiveAbilityState,
            &LifeGeneration,
            &MeshMaterial2d<ColorMaterial>,
            Option<&PlayerHealth>,
            Option<&EnemyBotHealth>,
            Option<&EnemyBotEvolution>,
            Option<&EnemyBotUpgrades>,
            Option<&MoveVelocity>,
            Option<&EnemyBotMoveVelocity>,
        ),
        Without<Projectile>,
    >,
    mut projectiles: Query<
        (
            Entity,
            &Transform,
            &mut Velocity,
            &mut ProjectileOwner,
            &mut ProjectileDamage,
            &mut ProjectileGeneration,
            &mut ProjectileAbility,
            &ProjectileEvolution,
        ),
        With<Projectile>,
    >,
) {
    for cast in casts.read() {
        let Ok((
            actor,
            mut transform,
            mut state,
            generation,
            body_material,
            player_health,
            bot_health,
            bot_evolution,
            bot_upgrades,
            player_velocity,
            bot_velocity,
        )) = actors.get_mut(cast.actor)
        else {
            continue;
        };
        let evolution = bot_evolution.map_or(&*player_evolution, |evolution| &evolution.0);
        state.sync_evolution(evolution.current_kind);
        let Some(kind) = state.kind else {
            continue;
        };
        let alive = player_health.map_or_else(
            || bot_health.is_some_and(|health| health.current > 0.0),
            |health| health.current > 0.0,
        );
        if !alive || state.cooldown_remaining > 0.0 {
            continue;
        }
        let owner = if bot_evolution.is_some() {
            ProjectileOwner::EnemyBot(actor)
        } else {
            ProjectileOwner::Player
        };
        let upgrades = bot_upgrades.map_or(&*player_upgrades, |upgrades| &upgrades.0);
        let max_health = player_health.map_or_else(
            || bot_health.map_or(1.0, |health| health.max),
            |health| health.max,
        );
        let direction = cast.aim.normalize_or(Vec2::Y);
        let position = transform.translation.xy();
        let projectile_material = if bot_evolution.is_some() {
            body_material.0.clone()
        } else {
            palettes
                .player(profile.data.selected_palette)
                .projectile
                .clone()
        };
        let damage = upgrades.bullet_damage() * evolution.bullet_damage_multiplier();
        let speed = upgrades.bullet_speed() * evolution.bullet_speed_multiplier();
        let lifetime = crate::projectile::projectile_lifetime(upgrades, evolution, 1.0);
        let radius = crate::projectile::projectile_radius(upgrades, evolution);

        let construct_count = |construct_kind| {
            constructs
                .iter()
                .filter(|construct| construct.owner == owner && construct.kind == construct_kind)
                .count()
        };
        let has_live_shell = || {
            projectiles
                .iter()
                .any(|(_, _, _, shot_owner, _, _, _, shot_evolution)| {
                    *shot_owner == owner
                        && matches!(
                            shot_evolution.0,
                            EvolutionKind::Annihilator | EvolutionKind::Siegebreaker
                        )
                })
        };
        let valid = cast_is_valid(
            kind,
            construct_count(ConstructKind::Turret),
            construct_count(ConstructKind::Mine),
            construct_count(ConstructKind::Fortification),
            has_live_shell(),
        );
        if !valid {
            continue;
        }

        state.cooldown_remaining = kind.cooldown();
        match kind {
            ActiveAbilityKind::GunPod => spawn_construct(
                &mut commands,
                &assets,
                ConstructKind::Turret,
                owner,
                generation.0,
                position + direction * 64.0,
                direction,
                max_health * 0.35,
                10.0,
                damage * 0.35,
                speed,
                550.0,
                body_material.0.clone(),
            ),
            ActiveAbilityKind::Brace => state.active_remaining = 4.0,
            ActiveAbilityKind::Airburst => {
                let shells = projectiles
                    .iter_mut()
                    .filter_map(
                        |(entity, shot_transform, _, shot_owner, shot_damage, _, _, shot_kind)| {
                            (*shot_owner == owner
                                && matches!(
                                    shot_kind.0,
                                    EvolutionKind::Annihilator | EvolutionKind::Siegebreaker
                                ))
                            .then_some((
                                entity,
                                shot_transform.translation.xy(),
                                shot_damage.0,
                            ))
                        },
                    )
                    .collect::<Vec<_>>();
                for (entity, shell_position, shell_damage) in shells {
                    commands.entity(entity).despawn();
                    commands.spawn(crate::collision::PendingSplash {
                        position: shell_position,
                        owner,
                        generation: generation.0,
                        direct_target: actor,
                        radius: 110.0,
                        damage: shell_damage * 0.70,
                        falloff_multiplier: 1.0,
                    });
                }
            }
            ActiveAbilityKind::PiercingLine | ActiveAbilityKind::Rangefinder => {
                state.prime_remaining = 5.0;
                state.charges = 1;
            }
            ActiveAbilityKind::FullBattery => {
                state.prime_remaining = f32::INFINITY;
                state.charges = 2;
            }
            ActiveAbilityKind::Counterburst => spawn_pending_effect(
                &mut commands,
                &assets,
                actor,
                owner,
                generation.0,
                kind,
                0.35,
                position,
                direction,
                damage,
                speed,
                lifetime,
                radius,
                projectile_material,
            ),
            ActiveAbilityKind::HunterMine => spawn_construct(
                &mut commands,
                &assets,
                ConstructKind::Mine,
                owner,
                generation.0,
                position + direction * 58.0,
                direction,
                max_health * 0.20,
                20.0,
                damage,
                speed,
                42.0,
                body_material.0.clone(),
            ),
            ActiveAbilityKind::RammingSpeed => spawn_pending_effect(
                &mut commands,
                &assets,
                actor,
                owner,
                generation.0,
                kind,
                0.25,
                position,
                direction,
                damage,
                speed,
                lifetime,
                radius,
                projectile_material,
            ),
            ActiveAbilityKind::Intercept => {
                transform.translation += (direction * 240.0).extend(0.0);
                reflect_projectiles(
                    actor,
                    owner,
                    position,
                    direction,
                    280.0,
                    0.75,
                    0.5,
                    generation.0,
                    &mut projectiles,
                );
                spawn_pending_effect(
                    &mut commands,
                    &assets,
                    actor,
                    owner,
                    generation.0,
                    kind,
                    0.01,
                    transform.translation.xy(),
                    direction,
                    damage,
                    speed,
                    lifetime,
                    radius,
                    projectile_material,
                );
            }
            ActiveAbilityKind::Saturation => spawn_pending_effect(
                &mut commands,
                &assets,
                actor,
                owner,
                generation.0,
                kind,
                0.45,
                position,
                direction,
                damage,
                speed,
                lifetime,
                radius,
                projectile_material,
            ),
            ActiveAbilityKind::PinningBurst => {
                state.prime_remaining = 5.0;
                state.charges = 6;
            }
            ActiveAbilityKind::Fortify => spawn_construct(
                &mut commands,
                &assets,
                ConstructKind::Fortification,
                owner,
                generation.0,
                position + direction * 70.0,
                direction,
                max_health * 1.50,
                12.0,
                0.0,
                0.0,
                65.0,
                body_material.0.clone(),
            ),
            ActiveAbilityKind::ShieldWall => {
                state.active_remaining = 3.0;
                state.bonus_shield = max_health * 0.30;
            }
            ActiveAbilityKind::Burnout => {
                state.active_remaining = 0.8;
                spawn_pending_effect(
                    &mut commands,
                    &assets,
                    actor,
                    owner,
                    generation.0,
                    kind,
                    0.02,
                    position,
                    direction,
                    damage,
                    speed,
                    lifetime,
                    radius,
                    projectile_material,
                );
            }
            ActiveAbilityKind::CombatRoll => {
                let movement = player_velocity
                    .map(|velocity| velocity.0)
                    .or_else(|| bot_velocity.map(|velocity| velocity.0))
                    .unwrap_or(Vec2::ZERO);
                let side = if movement.dot(Vec2::new(-direction.y, direction.x)) < 0.0 {
                    -1.0
                } else {
                    1.0
                };
                transform.translation +=
                    (Vec2::new(-direction.y, direction.x) * side * 100.0).extend(0.0);
                state.active_remaining = 1.8;
            }
        }
    }
}

fn cast_is_valid(
    kind: ActiveAbilityKind,
    turrets: usize,
    mines: usize,
    fortifications: usize,
    has_live_shell: bool,
) -> bool {
    match kind {
        ActiveAbilityKind::GunPod => turrets < 1,
        ActiveAbilityKind::HunterMine => mines < 2,
        ActiveAbilityKind::Fortify => fortifications < 1,
        ActiveAbilityKind::Airburst => has_live_shell,
        _ => true,
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_pending_effect(
    commands: &mut Commands,
    assets: &AbilityAssets,
    actor: Entity,
    owner: ProjectileOwner,
    generation: u32,
    kind: ActiveAbilityKind,
    delay: f32,
    position: Vec2,
    direction: Vec2,
    damage: f32,
    projectile_speed: f32,
    projectile_lifetime: f32,
    projectile_radius: f32,
    projectile_material: Handle<ColorMaterial>,
) {
    commands.spawn((
        PendingAbilityEffect {
            actor,
            owner,
            generation,
            kind,
            remaining: delay,
            position,
            direction,
            damage,
            projectile_speed,
            projectile_lifetime,
            projectile_radius,
            projectile_material,
            repeats: if kind == ActiveAbilityKind::Burnout {
                3
            } else {
                1
            },
        },
        Mesh2d(assets.telegraph_mesh.clone()),
        MeshMaterial2d(assets.accent.clone()),
        Transform::from_translation(position.extend(7.0)),
    ));
}

#[allow(clippy::too_many_arguments)]
fn spawn_construct(
    commands: &mut Commands,
    assets: &AbilityAssets,
    kind: ConstructKind,
    owner: ProjectileOwner,
    generation: u32,
    position: Vec2,
    direction: Vec2,
    health: f32,
    duration: f32,
    damage: f32,
    projectile_speed: f32,
    range: f32,
    material: Handle<ColorMaterial>,
) {
    let mesh = match kind {
        ConstructKind::Turret => assets.turret_mesh.clone(),
        ConstructKind::Mine => assets.mine_mesh.clone(),
        ConstructKind::Fortification => assets.wall_mesh.clone(),
    };
    let rotation = if kind == ConstructKind::Fortification {
        Quat::from_rotation_z(direction.to_angle() + std::f32::consts::FRAC_PI_2)
    } else {
        Quat::IDENTITY
    };
    commands
        .spawn((
            Construct {
                kind,
                owner,
                generation,
                health,
                max_health: health,
                remaining: duration,
                duration,
                damage,
                projectile_speed,
                range,
                fire_timer: 0.0,
            },
            Mesh2d(mesh),
            MeshMaterial2d(material),
            Transform::from_translation(position.extend(3.0)).with_rotation(rotation),
        ))
        .with_children(|construct| {
            construct.spawn((
                Mesh2d(assets.bar_mesh.clone()),
                MeshMaterial2d(assets.bar_background.clone()),
                Transform::from_xyz(0.0, 28.0, 1.0),
            ));
            construct.spawn((
                ConstructHealthFill,
                Mesh2d(assets.bar_mesh.clone()),
                MeshMaterial2d(assets.accent.clone()),
                Transform::from_xyz(0.0, 28.0, 2.0),
            ));
        });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn tick_abilities(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<AbilityAssets>,
    projectile_assets: Res<ProjectileAssets>,
    player_evolution: Res<EvolutionState>,
    mut actors: Query<
        (
            Entity,
            &Transform,
            &mut ActiveAbilityState,
            &LifeGeneration,
            Option<&EnemyBotEvolution>,
            Option<&mut Slowed>,
            Option<&mut Velocity>,
            Option<&mut EnemyBotVelocity>,
        ),
        Without<Projectile>,
    >,
    mut pending: Query<(Entity, &mut PendingAbilityEffect)>,
    mut projectiles: Query<
        (
            Entity,
            &Transform,
            &mut Velocity,
            &mut ProjectileOwner,
            &mut ProjectileDamage,
            &mut ProjectileGeneration,
            &mut ProjectileAbility,
            &ProjectileEvolution,
        ),
        With<Projectile>,
    >,
) {
    let dt = time.delta_secs();
    let mut shield_walls = Vec::new();
    for (entity, transform, mut state, generation, bot_evolution, slow, _, _) in &mut actors {
        let evolution = bot_evolution.map_or(player_evolution.current_kind, |e| e.0.current_kind);
        state.sync_evolution(evolution);
        state.cooldown_remaining = (state.cooldown_remaining - dt).max(0.0);
        state.active_remaining = (state.active_remaining - dt).max(0.0);
        if state.active_remaining <= 0.0 {
            state.bonus_shield = 0.0;
        }
        state.prime_remaining = (state.prime_remaining - dt).max(0.0);
        if state.prime_remaining <= 0.0 {
            state.charges = 0;
        }
        if let Some(mut slow) = slow {
            slow.remaining = (slow.remaining - dt).max(0.0);
            if slow.remaining <= 0.0 {
                slow.amount = 0.0;
            }
        }
        if state.kind == Some(ActiveAbilityKind::ShieldWall) && state.active_remaining > 0.0 {
            shield_walls.push((
                entity,
                if bot_evolution.is_some() {
                    ProjectileOwner::EnemyBot(entity)
                } else {
                    ProjectileOwner::Player
                },
                generation.0,
                transform.translation.xy(),
                (transform.rotation * Vec3::Y).xy(),
            ));
        }
    }
    for (actor, owner, generation, position, direction) in shield_walls {
        reflect_projectiles(
            actor,
            owner,
            position,
            direction,
            150.0,
            0.50,
            0.5,
            generation,
            &mut projectiles,
        );
    }

    for (effect_entity, mut effect) in &mut pending {
        effect.remaining -= dt;
        if effect.remaining > 0.0 {
            continue;
        }
        match effect.kind {
            ActiveAbilityKind::Counterburst => {
                reflect_projectiles(
                    effect.actor,
                    effect.owner,
                    effect.position,
                    Vec2::ZERO,
                    150.0,
                    0.75,
                    -1.0,
                    effect.generation,
                    &mut projectiles,
                );
                push_nearby_tanks(effect.actor, effect.position, 150.0, &mut actors);
            }
            ActiveAbilityKind::RammingSpeed => {
                if let Ok((_, _, mut state, _, _, _, _, _)) = actors.get_mut(effect.actor) {
                    state.active_remaining = 1.5;
                }
            }
            ActiveAbilityKind::Intercept => {
                push_nearby_tanks(effect.actor, effect.position, 125.0, &mut actors);
            }
            ActiveAbilityKind::Saturation => {
                for index in 0..16 {
                    let direction = Vec2::from_angle(index as f32 * std::f32::consts::TAU / 16.0);
                    spawn_ability_projectile(
                        &mut commands,
                        &projectile_assets,
                        effect.owner,
                        effect.generation,
                        effect.position,
                        direction,
                        effect.damage * 0.30,
                        effect.projectile_speed,
                        effect.projectile_lifetime * 0.65,
                        effect.projectile_radius,
                        effect.projectile_material.clone(),
                        EvolutionKind::Bombardier,
                    );
                }
            }
            ActiveAbilityKind::Burnout => {
                let rear = -effect.direction;
                for offset in [-0.18_f32, 0.0, 0.18] {
                    spawn_ability_projectile(
                        &mut commands,
                        &projectile_assets,
                        effect.owner,
                        effect.generation,
                        effect.position,
                        Vec2::from_angle(rear.to_angle() + offset),
                        effect.damage * 0.50,
                        effect.projectile_speed,
                        effect.projectile_lifetime,
                        effect.projectile_radius,
                        effect.projectile_material.clone(),
                        EvolutionKind::Afterburner,
                    );
                }
            }
            _ => {}
        }
        if effect.kind == ActiveAbilityKind::Burnout && effect.repeats > 1 {
            effect.repeats -= 1;
            effect.remaining = 0.25;
            if let Ok((_, transform, ..)) = actors.get(effect.actor) {
                effect.position = transform.translation.xy();
                effect.direction = (transform.rotation * Vec3::Y).xy().normalize_or(Vec2::Y);
            }
            continue;
        }
        commands.entity(effect_entity).despawn();
    }
    let _ = assets.hostile_accent.clone();
}

#[allow(clippy::too_many_arguments)]
fn spawn_ability_projectile(
    commands: &mut Commands,
    assets: &ProjectileAssets,
    owner: ProjectileOwner,
    generation: u32,
    position: Vec2,
    direction: Vec2,
    damage: f32,
    speed: f32,
    lifetime: f32,
    radius: f32,
    material: Handle<ColorMaterial>,
    evolution: EvolutionKind,
) {
    let scale = radius / constants::PROJECTILE_RADIUS;
    commands
        .spawn((
            Projectile,
            owner,
            Lifetime(lifetime),
            ProjectileDamage(damage),
            ProjectilePenetration(1),
            ProjectileKnockback(1.0),
            ProjectileEvolution(evolution),
            ProjectileTravel::default(),
            ProjectileRear(false),
            ProjectileSplashReady(false),
            ProjectileHitHistory::default(),
            ProjectileGeneration(generation),
            ProjectileRadius(radius),
            Mesh2d(assets.mesh.clone()),
            MeshMaterial2d(material),
        ))
        .insert((
            Transform::from_translation(position.extend(5.0))
                .with_scale(Vec3::new(scale, scale, 1.0)),
            Velocity(direction.normalize_or(Vec2::Y) * speed),
            ProjectileAbility::default(),
        ));
}

fn reflect_projectiles(
    actor: Entity,
    new_owner: ProjectileOwner,
    center: Vec2,
    direction: Vec2,
    radius: f32,
    damage_multiplier: f32,
    min_dot: f32,
    new_generation: u32,
    projectiles: &mut Query<
        (
            Entity,
            &Transform,
            &mut Velocity,
            &mut ProjectileOwner,
            &mut ProjectileDamage,
            &mut ProjectileGeneration,
            &mut ProjectileAbility,
            &ProjectileEvolution,
        ),
        With<Projectile>,
    >,
) {
    for (_, transform, mut velocity, mut owner, mut damage, mut generation, mut ability, _) in
        projectiles.iter_mut()
    {
        let offset = transform.translation.xy() - center;
        if offset.length() > radius || *owner == new_owner {
            continue;
        }
        if direction != Vec2::ZERO && offset.normalize_or_zero().dot(direction) < min_dot {
            continue;
        }
        apply_reflection(
            &mut owner,
            &mut generation,
            &mut velocity,
            &mut damage,
            &mut ability,
            new_owner,
            new_generation,
            damage_multiplier,
        );
        let _ = actor;
    }
}

fn apply_reflection(
    owner: &mut ProjectileOwner,
    generation: &mut ProjectileGeneration,
    velocity: &mut Velocity,
    damage: &mut ProjectileDamage,
    ability: &mut ProjectileAbility,
    new_owner: ProjectileOwner,
    new_generation: u32,
    damage_multiplier: f32,
) {
    *owner = new_owner;
    generation.0 = new_generation;
    velocity.0 = -velocity.0;
    damage.0 *= damage_multiplier;
    ability.reflected = true;
}

fn push_nearby_tanks(
    actor: Entity,
    center: Vec2,
    radius: f32,
    actors: &mut Query<
        (
            Entity,
            &Transform,
            &mut ActiveAbilityState,
            &LifeGeneration,
            Option<&EnemyBotEvolution>,
            Option<&mut Slowed>,
            Option<&mut Velocity>,
            Option<&mut EnemyBotVelocity>,
        ),
        Without<Projectile>,
    >,
) {
    for (entity, transform, _, _, _, _, player_velocity, bot_velocity) in actors.iter_mut() {
        if entity == actor {
            continue;
        }
        let offset = transform.translation.xy() - center;
        if offset.length() > radius {
            continue;
        }
        let impulse = offset.normalize_or(Vec2::X) * 260.0;
        if let Some(mut velocity) = player_velocity {
            velocity.0 += impulse;
        }
        if let Some(mut velocity) = bot_velocity {
            velocity.0 += impulse;
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_constructs(
    mut commands: Commands,
    time: Res<Time>,
    projectile_assets: Res<ProjectileAssets>,
    mut constructs: Query<
        (Entity, &Transform, &mut Construct, &Children),
        Without<ConstructHealthFill>,
    >,
    mut fills: Query<
        &mut Transform,
        (
            With<ConstructHealthFill>,
            Without<Construct>,
            Without<Player>,
            Without<EnemyBot>,
        ),
    >,
    player: Query<
        (Entity, &Transform, &PlayerHealth, &LifeGeneration),
        (With<Player>, Without<ConstructHealthFill>),
    >,
    bots: Query<
        (Entity, &Transform, &EnemyBotHealth, &LifeGeneration),
        (
            With<EnemyBot>,
            Without<Player>,
            Without<ConstructHealthFill>,
        ),
    >,
    actor_materials: Query<&MeshMaterial2d<ColorMaterial>>,
) {
    let dt = time.delta_secs();
    for (entity, transform, mut construct, children) in &mut constructs {
        construct.remaining -= dt;
        construct.health -= construct.max_health / construct.duration.max(0.1) * dt;
        let owner_alive = match construct.owner {
            ProjectileOwner::Player => player.single().is_ok_and(|(_, _, health, generation)| {
                health.current > 0.0 && generation.0 == construct.generation
            }),
            ProjectileOwner::EnemyBot(owner) => {
                bots.get(owner).is_ok_and(|(_, _, health, generation)| {
                    health.current > 0.0 && generation.0 == construct.generation
                })
            }
        };
        if !owner_alive || construct.remaining <= 0.0 || construct.health <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let fraction = (construct.health / construct.max_health).clamp(0.0, 1.0);
        for child in children.iter() {
            if let Ok(mut fill) = fills.get_mut(child) {
                fill.scale.x = fraction;
                fill.translation.x = -(1.0 - fraction) * CONSTRUCT_BAR_WIDTH * 0.5;
            }
        }
        if construct.kind != ConstructKind::Turret {
            continue;
        }
        construct.fire_timer -= dt;
        if construct.fire_timer > 0.0 {
            continue;
        }
        let position = transform.translation.xy();
        let target = match construct.owner {
            ProjectileOwner::Player => bots
                .iter()
                .filter(|(_, _, health, _)| health.current > 0.0)
                .filter(|(_, target, _, _)| {
                    target.translation.xy().distance(position) <= construct.range
                })
                .min_by(|(_, a, _, _), (_, b, _, _)| {
                    a.translation
                        .xy()
                        .distance_squared(position)
                        .total_cmp(&b.translation.xy().distance_squared(position))
                })
                .map(|(_, transform, _, _)| transform.translation.xy()),
            ProjectileOwner::EnemyBot(owner) => {
                let mut targets = player
                    .iter()
                    .filter(|(_, _, health, _)| health.current > 0.0)
                    .map(|(_, target, _, _)| target.translation.xy())
                    .collect::<Vec<_>>();
                targets.extend(
                    bots.iter()
                        .filter(|(entity, _, health, _)| *entity != owner && health.current > 0.0)
                        .map(|(_, target, _, _)| target.translation.xy()),
                );
                targets
                    .into_iter()
                    .filter(|target| target.distance(position) <= construct.range)
                    .min_by(|a, b| {
                        a.distance_squared(position)
                            .total_cmp(&b.distance_squared(position))
                    })
            }
        };
        let Some(target) = target else {
            continue;
        };
        construct.fire_timer = 0.65;
        let material = match construct.owner {
            ProjectileOwner::Player => player
                .single()
                .ok()
                .and_then(|(entity, ..)| actor_materials.get(entity).ok()),
            ProjectileOwner::EnemyBot(entity) => actor_materials.get(entity).ok(),
        }
        .map_or_else(Handle::default, |material| material.0.clone());
        spawn_ability_projectile(
            &mut commands,
            &projectile_assets,
            construct.owner,
            construct.generation,
            position,
            (target - position).normalize_or_zero(),
            construct.damage,
            construct.projectile_speed,
            1.8,
            constants::PROJECTILE_RADIUS,
            material,
            EvolutionKind::Sentry,
        );
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn resolve_construct_collisions(
    mut commands: Commands,
    mut death_queue: ResMut<CombatDeathQueue>,
    mut constructs: Query<
        (Entity, &Transform, &mut Construct),
        (Without<Player>, Without<EnemyBot>, Without<Projectile>),
    >,
    mut player: Query<
        (
            Entity,
            &mut Transform,
            &mut PlayerHealth,
            &mut Velocity,
            &mut Slowed,
        ),
        (
            With<Player>,
            Without<EnemyBot>,
            Without<Construct>,
            Without<Projectile>,
        ),
    >,
    mut bots: Query<
        (
            Entity,
            &mut Transform,
            &mut EnemyBotHealth,
            &mut EnemyBotVelocity,
            &mut Slowed,
        ),
        (
            With<EnemyBot>,
            Without<Player>,
            Without<Construct>,
            Without<Projectile>,
        ),
    >,
    mut projectiles: Query<
        (
            Entity,
            &Transform,
            &ProjectileOwner,
            &ProjectileDamage,
            &ProjectileRadius,
        ),
        (
            With<Projectile>,
            Without<Construct>,
            Without<Player>,
            Without<EnemyBot>,
        ),
    >,
    mut passives: Query<&mut crate::passive::PassiveRuntime>,
) {
    let player_entity = player.single().ok().map(|player| player.0);
    let mut consumed_projectiles = Vec::new();
    for (construct_entity, construct_transform, mut construct) in &mut constructs {
        let center = construct_transform.translation.xy();
        for (projectile_entity, shot_transform, owner, damage, radius) in &mut projectiles {
            if *owner == construct.owner
                || consumed_projectiles.contains(&projectile_entity)
                || !projectile_hits_construct(
                    construct.kind,
                    construct_transform,
                    shot_transform,
                    radius.0,
                )
            {
                continue;
            }
            construct.health -= damage.0;
            commands.entity(projectile_entity).despawn();
            consumed_projectiles.push(projectile_entity);
            if construct.health <= 0.0 {
                commands.entity(construct_entity).despawn();
                break;
            }
        }
        if construct.health <= 0.0 {
            continue;
        }
        if construct.kind == ConstructKind::Mine {
            let victim = match construct.owner {
                ProjectileOwner::Player => bots
                    .iter_mut()
                    .find(|(_, transform, health, _, _)| {
                        health.current > 0.0 && transform.translation.xy().distance(center) <= 42.0
                    })
                    .map(|(entity, _, mut health, _, mut slow)| {
                        health.current = (health.current - construct.damage).max(0.0);
                        slow.amount = (slow.amount + 0.25).min(0.30);
                        slow.remaining = 3.0;
                        (CombatantId::EnemyBot(entity), health.current <= 0.0)
                    }),
                ProjectileOwner::EnemyBot(owner) => {
                    if let Ok((_, transform, mut health, _, mut slow)) = player.single_mut()
                        && health.current > 0.0
                        && transform.translation.xy().distance(center) <= 42.0
                    {
                        health.current = (health.current - construct.damage).max(0.0);
                        slow.amount = (slow.amount + 0.25).min(0.30);
                        slow.remaining = 3.0;
                        Some((CombatantId::Player, health.current <= 0.0))
                    } else {
                        bots.iter_mut()
                            .find(|(entity, transform, health, _, _)| {
                                *entity != owner
                                    && health.current > 0.0
                                    && transform.translation.xy().distance(center) <= 42.0
                            })
                            .map(|(entity, _, mut health, _, mut slow)| {
                                health.current = (health.current - construct.damage).max(0.0);
                                slow.amount = (slow.amount + 0.25).min(0.30);
                                slow.remaining = 3.0;
                                (CombatantId::EnemyBot(entity), health.current <= 0.0)
                            })
                    }
                }
            };
            if let Some((victim, died)) = victim {
                let victim_entity = match victim {
                    CombatantId::Player => player_entity,
                    CombatantId::EnemyBot(entity) => Some(entity),
                };
                let owner_entity = match construct.owner {
                    ProjectileOwner::Player => player_entity,
                    ProjectileOwner::EnemyBot(entity) => Some(entity),
                };
                if let (Some(owner_entity), Some(victim_entity)) = (owner_entity, victim_entity)
                    && let Ok(mut passive) = passives.get_mut(owner_entity)
                {
                    passive.tracked_target = Some(victim_entity);
                    passive.follow_up_hits = 3;
                    passive.stack_timer = 2.5;
                }
                if died {
                    death_queue.record(victim, Some(construct.owner.combatant()));
                }
                commands.entity(construct_entity).despawn();
            }
            continue;
        }
        if construct.kind != ConstructKind::Fortification {
            continue;
        }
        for (_, mut transform, health, mut velocity, _) in &mut player {
            if health.current > 0.0 {
                push_from_wall(&mut transform, &mut velocity.0, construct_transform);
            }
        }
        for (_, mut transform, health, mut velocity, _) in &mut bots {
            if health.current > 0.0 {
                push_from_wall(&mut transform, &mut velocity.0, construct_transform);
            }
        }
    }
}

fn projectile_hits_construct(
    kind: ConstructKind,
    construct: &Transform,
    projectile: &Transform,
    projectile_radius: f32,
) -> bool {
    if kind == ConstructKind::Fortification {
        let local = construct
            .compute_affine()
            .inverse()
            .transform_point3(projectile.translation);
        return local.x.abs() <= 58.0 + projectile_radius
            && local.y.abs() <= 14.0 + projectile_radius;
    }
    let radius = match kind {
        ConstructKind::Turret => 18.0,
        ConstructKind::Mine => 14.0,
        ConstructKind::Fortification => unreachable!(),
    };
    construct
        .translation
        .xy()
        .distance_squared(projectile.translation.xy())
        <= (radius + projectile_radius).powi(2)
}

pub fn resolve_projectile_manipulation(
    mut commands: Commands,
    lane_shots: Query<
        (
            Entity,
            &Transform,
            &Velocity,
            &ProjectileOwner,
            &ProjectileAbility,
        ),
        With<Projectile>,
    >,
    targets: Query<(Entity, &Transform, &ProjectileOwner, &ProjectileRadius), With<Projectile>>,
) {
    for (lane_entity, transform, velocity, owner, ability) in &lane_shots {
        if !ability.clears_projectiles {
            continue;
        }
        let origin = transform.translation.xy();
        let direction = velocity.0.normalize_or_zero();
        for (target_entity, target, target_owner, radius) in &targets {
            if target_entity == lane_entity || target_owner == owner {
                continue;
            }
            let offset = target.translation.xy() - origin;
            let forward = offset.dot(direction);
            let perpendicular = (offset - direction * forward).length();
            if (0.0..=120.0).contains(&forward) && perpendicular <= 10.0 + radius.0 {
                commands.entity(target_entity).despawn();
            }
        }
    }
}

fn push_from_wall(transform: &mut Transform, velocity: &mut Vec2, wall: &Transform) {
    let local = wall
        .compute_affine()
        .inverse()
        .transform_point3(transform.translation);
    if local.x.abs() <= 75.0 && local.y.abs() <= 38.0 {
        let local_normal = if local.y.abs() > 0.01 {
            Vec2::Y * local.y.signum()
        } else {
            Vec2::Y
        };
        let normal = (wall.rotation * local_normal.extend(0.0)).xy();
        transform.translation += (normal * (38.0 - local.y.abs())).extend(0.0);
        *velocity += normal * 80.0;
    }
}

pub fn ensure_ability_rings(
    mut commands: Commands,
    assets: Res<AbilityAssets>,
    actors: Query<
        (Entity, &MeshMaterial2d<ColorMaterial>, &ActiveAbilityState),
        Or<(With<Player>, With<EnemyBot>)>,
    >,
    rings: Query<&AbilityCooldownRing>,
    lines: Query<&AbilityAimLine>,
) {
    for (actor, material, state) in &actors {
        if state.kind.is_none() {
            continue;
        }
        if !rings.iter().any(|ring| ring.owner == actor) {
            commands.spawn((
                AbilityCooldownRing { owner: actor },
                Mesh2d(assets.ring_mesh.clone()),
                MeshMaterial2d(material.0.clone()),
                Transform::from_xyz(0.0, 0.0, 6.0),
            ));
        }
        if state.kind == Some(ActiveAbilityKind::Rangefinder)
            && state.prime_remaining > 0.0
            && !lines.iter().any(|line| line.owner == actor)
        {
            commands.spawn((
                AbilityAimLine { owner: actor },
                Mesh2d(assets.aim_line_mesh.clone()),
                MeshMaterial2d(assets.accent.clone()),
                Transform::from_xyz(0.0, 0.0, 5.5),
            ));
        }
    }
}

pub fn update_ability_presentation(
    player_evolution: Res<EvolutionState>,
    player: Query<(Entity, &ActiveAbilityState), With<Player>>,
    actors: Query<
        (&Transform, &ActiveAbilityState),
        (Without<AbilityCooldownRing>, Without<AbilityAimLine>),
    >,
    mut rings: Query<
        (&AbilityCooldownRing, &mut Transform, &mut Visibility),
        (
            Without<Player>,
            Without<ActiveAbilityState>,
            Without<AbilityPanel>,
        ),
    >,
    mut lines: Query<
        (&AbilityAimLine, &mut Transform, &mut Visibility),
        (
            Without<Player>,
            Without<AbilityCooldownRing>,
            Without<ActiveAbilityState>,
            Without<AbilityPanel>,
        ),
    >,
    mut panel: Query<
        &mut Visibility,
        (
            With<AbilityPanel>,
            Without<AbilityCooldownRing>,
            Without<AbilityAimLine>,
        ),
    >,
    mut text: Query<&mut Text, With<AbilityPanelText>>,
) {
    let player_state = player.single().ok();
    for (ring, mut transform, mut visibility) in &mut rings {
        let Ok((owner_transform, state)) = actors.get(ring.owner) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let Some(kind) = state.kind else {
            *visibility = Visibility::Hidden;
            continue;
        };
        *visibility = Visibility::Visible;
        transform.translation = owner_transform.translation + Vec3::Z * 6.0;
        let readiness = 1.0 - state.cooldown_remaining / kind.cooldown();
        transform.scale = Vec3::splat(0.84 + readiness.clamp(0.0, 1.0) * 0.16);
    }
    for (line, mut transform, mut visibility) in &mut lines {
        let Ok((owner_transform, state)) = actors.get(line.owner) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let active =
            state.kind == Some(ActiveAbilityKind::Rangefinder) && state.prime_remaining > 0.0;
        *visibility = if active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let forward = owner_transform.rotation * Vec3::Y;
        transform.translation = owner_transform.translation + forward * 450.0 + Vec3::Z * 5.5;
        transform.rotation = owner_transform.rotation;
    }
    let visible = player_state.is_some_and(|(_, state)| state.kind.is_some());
    for mut visibility in &mut panel {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let Some((_, state)) = player_state else {
        return;
    };
    let Some(kind) = ActiveAbilityKind::from_evolution(player_evolution.current_kind) else {
        return;
    };
    for mut label in &mut text {
        **label = if state.cooldown_remaining <= 0.0 {
            format!("RMB  {}  READY", kind.name())
        } else {
            format!("RMB  {}  {:.1}s", kind.name(), state.cooldown_remaining)
        };
    }
}

impl ProjectileOwner {
    pub fn combatant(self) -> CombatantId {
        match self {
            Self::Player => CombatantId::Player,
            Self::EnemyBot(entity) => CombatantId::EnemyBot(entity),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_capstone_has_the_stated_fixed_cooldown() {
        let cases = [
            (EvolutionKind::Sentry, ActiveAbilityKind::GunPod, 18.0),
            (EvolutionKind::Emplacement, ActiveAbilityKind::Brace, 14.0),
            (
                EvolutionKind::Siegebreaker,
                ActiveAbilityKind::Airburst,
                14.0,
            ),
            (EvolutionKind::Lancer, ActiveAbilityKind::PiercingLine, 14.0),
            (
                EvolutionKind::Fusillade,
                ActiveAbilityKind::FullBattery,
                14.0,
            ),
            (
                EvolutionKind::Rearguard,
                ActiveAbilityKind::Counterburst,
                16.0,
            ),
            (EvolutionKind::Deadeye, ActiveAbilityKind::Rangefinder, 12.0),
            (EvolutionKind::Pursuer, ActiveAbilityKind::HunterMine, 10.0),
            (
                EvolutionKind::Dreadnought,
                ActiveAbilityKind::RammingSpeed,
                16.0,
            ),
            (EvolutionKind::Vanguard, ActiveAbilityKind::Intercept, 12.0),
            (
                EvolutionKind::Bombardier,
                ActiveAbilityKind::Saturation,
                15.0,
            ),
            (
                EvolutionKind::Impaler,
                ActiveAbilityKind::PinningBurst,
                12.0,
            ),
            (EvolutionKind::Stronghold, ActiveAbilityKind::Fortify, 18.0),
            (EvolutionKind::Guardian, ActiveAbilityKind::ShieldWall, 16.0),
            (EvolutionKind::Afterburner, ActiveAbilityKind::Burnout, 10.0),
            (EvolutionKind::Ace, ActiveAbilityKind::CombatRoll, 10.0),
        ];
        for (evolution, ability, cooldown) in cases {
            assert_eq!(ActiveAbilityKind::from_evolution(evolution), Some(ability));
            assert_eq!(ability.cooldown(), cooldown);
        }
    }

    #[test]
    fn evolution_change_makes_new_ability_immediately_ready() {
        let mut state = ActiveAbilityState {
            kind: Some(ActiveAbilityKind::Brace),
            cooldown_remaining: 9.0,
            ..default()
        };
        state.sync_evolution(EvolutionKind::Lancer);
        assert_eq!(state.kind, Some(ActiveAbilityKind::PiercingLine));
        assert_eq!(state.cooldown_remaining, 0.0);
    }

    #[test]
    fn slow_and_active_reductions_remain_soft_control() {
        assert_eq!(
            Slowed {
                amount: 0.9,
                remaining: 1.0
            }
            .movement_multiplier(),
            0.7
        );
        let state = ActiveAbilityState {
            kind: Some(ActiveAbilityKind::Brace),
            active_remaining: 1.0,
            ..default()
        };
        assert_eq!(state.damage_multiplier(), 0.75);
    }

    #[test]
    fn invalid_casts_preserve_cooldown_and_construct_limits() {
        assert!(!cast_is_valid(ActiveAbilityKind::Airburst, 0, 0, 0, false));
        assert!(cast_is_valid(ActiveAbilityKind::Airburst, 0, 0, 0, true));
        assert!(!cast_is_valid(ActiveAbilityKind::GunPod, 1, 0, 0, true));
        assert!(!cast_is_valid(ActiveAbilityKind::HunterMine, 0, 2, 0, true));
        assert!(!cast_is_valid(ActiveAbilityKind::Fortify, 0, 0, 1, true));

        let state = ActiveAbilityState::default();
        assert_eq!(state.cooldown_remaining, 0.0);
    }

    #[test]
    fn reflection_transfers_owner_generation_direction_and_damage() {
        let mut owner = ProjectileOwner::EnemyBot(Entity::from_bits(7));
        let mut generation = ProjectileGeneration(3);
        let mut velocity = Velocity(Vec2::new(100.0, -25.0));
        let mut damage = ProjectileDamage(40.0);
        let mut ability = ProjectileAbility {
            pinning: true,
            ..default()
        };

        apply_reflection(
            &mut owner,
            &mut generation,
            &mut velocity,
            &mut damage,
            &mut ability,
            ProjectileOwner::Player,
            11,
            0.75,
        );

        assert_eq!(owner, ProjectileOwner::Player);
        assert_eq!(generation.0, 11);
        assert_eq!(velocity.0, Vec2::new(-100.0, 25.0));
        assert_eq!(damage.0, 30.0);
        assert!(ability.reflected);
    }

    #[test]
    fn pinning_burst_counts_tank_hits_instead_of_trigger_pulls() {
        let mut state = ActiveAbilityState {
            kind: Some(ActiveAbilityKind::PinningBurst),
            prime_remaining: 5.0,
            charges: 6,
            ..default()
        };
        assert!(state.primed_shot().pinning);
        assert!(state.primed_shot().pinning);
        assert_eq!(state.charges, 6);
        for remaining in (0..6).rev() {
            assert!(state.consume_pinning_hit());
            assert_eq!(state.charges, remaining);
        }
        assert!(!state.consume_pinning_hit());
        assert_eq!(state.prime_remaining, 0.0);
    }

    #[test]
    fn every_construct_has_projectile_collision_geometry() {
        let center = Transform::default();
        let near = Transform::from_xyz(16.0, 0.0, 0.0);
        let far = Transform::from_xyz(80.0, 80.0, 0.0);
        assert!(projectile_hits_construct(
            ConstructKind::Turret,
            &center,
            &near,
            4.8
        ));
        assert!(projectile_hits_construct(
            ConstructKind::Mine,
            &center,
            &near,
            4.8
        ));
        assert!(!projectile_hits_construct(
            ConstructKind::Mine,
            &center,
            &far,
            4.8
        ));

        let wall = Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        let along_rotated_wall = Transform::from_xyz(0.0, 50.0, 0.0);
        assert!(projectile_hits_construct(
            ConstructKind::Fortification,
            &wall,
            &along_rotated_wall,
            4.8
        ));
    }

    #[test]
    fn bot_ability_preconditions_are_role_specific() {
        assert!(!bot_should_cast(
            ActiveAbilityKind::Airburst,
            300.0,
            false,
            1.0,
            false
        ));
        assert!(bot_should_cast(
            ActiveAbilityKind::Airburst,
            300.0,
            false,
            1.0,
            true
        ));
        assert!(!bot_should_cast(
            ActiveAbilityKind::Brace,
            300.0,
            false,
            0.95,
            true
        ));
        assert!(bot_should_cast(
            ActiveAbilityKind::Brace,
            300.0,
            false,
            0.50,
            true
        ));
        assert!(bot_should_cast(
            ActiveAbilityKind::ShieldWall,
            600.0,
            true,
            1.0,
            false
        ));
        assert!(!bot_should_cast(
            ActiveAbilityKind::ShieldWall,
            600.0,
            false,
            1.0,
            false
        ));
    }

    #[test]
    fn full_battery_persists_until_exactly_two_volleys_fire() {
        let mut state = ActiveAbilityState {
            kind: Some(ActiveAbilityKind::FullBattery),
            prime_remaining: f32::INFINITY,
            charges: 2,
            ..default()
        };
        assert!(state.full_battery());
        assert!(state.full_battery());
        assert!(!state.full_battery());
    }
}
