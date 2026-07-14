use crate::palette::PaletteId;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

const PROFILE_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct IdentityData {
    pub player_name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordData {
    pub lifetime_kills: u32,
    pub lifetime_deaths: u32,
    pub shapes_destroyed: u32,
    pub best_life_score: u32,
    pub longest_life_secs: f32,
    pub best_crown_streak_secs: f32,
    pub total_crown_time_secs: f32,
    pub best_life_kills: u32,
    pub highest_level: u32,
    pub used_level_five_evolutions: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsData {
    pub screen_shake: f32,
    pub damage_indicators: bool,
    pub fullscreen: bool,
}

impl Default for SettingsData {
    fn default() -> Self {
        Self {
            screen_shake: 0.35,
            damage_indicators: true,
            fullscreen: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AchievementId {
    ShapeHunter,
    FirstKill,
    FirstEvolution,
    ClaimCrown,
    CrownThirty,
    Survivor,
    ScoreThousand,
    FiveKillLife,
    AdvancedEvolution,
    CrownOneTwenty,
    EvolutionMastery,
}

impl AchievementId {
    pub const ALL: [Self; 11] = [
        Self::ShapeHunter,
        Self::FirstKill,
        Self::FirstEvolution,
        Self::ClaimCrown,
        Self::CrownThirty,
        Self::Survivor,
        Self::ScoreThousand,
        Self::FiveKillLife,
        Self::AdvancedEvolution,
        Self::CrownOneTwenty,
        Self::EvolutionMastery,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::ShapeHunter => "Shape Hunter",
            Self::FirstKill => "First Blood",
            Self::FirstEvolution => "Evolved",
            Self::ClaimCrown => "Crowned",
            Self::CrownThirty => "Dominant",
            Self::Survivor => "Survivor",
            Self::ScoreThousand => "High Score",
            Self::FiveKillLife => "Hunter",
            Self::AdvancedEvolution => "Apex",
            Self::CrownOneTwenty => "Sovereign",
            Self::EvolutionMastery => "Versatile",
        }
    }

    pub fn palette(self) -> PaletteId {
        PaletteId::ALL[self as usize + 1]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileData {
    pub version: u32,
    pub identity: IdentityData,
    pub records: RecordData,
    pub achievements: BTreeSet<AchievementId>,
    pub unlocked_palettes: BTreeSet<PaletteId>,
    pub selected_palette: PaletteId,
    pub settings: SettingsData,
}

impl Default for ProfileData {
    fn default() -> Self {
        Self {
            version: PROFILE_VERSION,
            identity: IdentityData::default(),
            records: RecordData::default(),
            achievements: BTreeSet::new(),
            unlocked_palettes: BTreeSet::from([PaletteId::CoreBlue]),
            selected_palette: PaletteId::CoreBlue,
            settings: SettingsData::default(),
        }
    }
}

#[derive(Resource, Debug)]
pub struct Profile {
    pub data: ProfileData,
    path: Option<PathBuf>,
    dirty: bool,
}

impl Profile {
    pub fn load() -> Self {
        let path = profile_path();
        let legacy = legacy_name_path();
        Self::load_from(path, legacy)
    }

    fn load_from(path: Option<PathBuf>, legacy: Option<PathBuf>) -> Self {
        let mut data = path.as_deref().and_then(read_profile).unwrap_or_default();
        data.version = PROFILE_VERSION;
        data.unlocked_palettes.insert(PaletteId::CoreBlue);
        if !data.unlocked_palettes.contains(&data.selected_palette) {
            data.selected_palette = PaletteId::CoreBlue;
        }

        let mut dirty = false;
        if data.identity.player_name.is_empty()
            && let Some(name_path) = legacy.as_deref()
            && let Ok(name) = fs::read_to_string(name_path)
        {
            data.identity.player_name = sanitize_player_name(&name);
            dirty = !data.identity.player_name.is_empty();
        }

        Self { data, path, dirty }
    }

    pub fn set_player_name(&mut self, name: &str) {
        let name = sanitize_player_name(name);
        if self.data.identity.player_name != name {
            self.data.identity.player_name = name;
            self.dirty = true;
        }
    }

    pub fn unlock(&mut self, achievement: AchievementId) -> bool {
        if !self.data.achievements.insert(achievement) {
            return false;
        }
        self.data.unlocked_palettes.insert(achievement.palette());
        self.dirty = true;
        true
    }

    pub fn select_palette(&mut self, palette: PaletteId) -> bool {
        if !self.data.unlocked_palettes.contains(&palette) {
            return false;
        }
        if self.data.selected_palette != palette {
            self.data.selected_palette = palette;
            self.dirty = true;
        }
        true
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let Some(path) = self.path.as_deref() else {
            return Ok(());
        };
        write_profile(path, &self.data)?;
        self.dirty = false;
        Ok(())
    }
}

pub fn flush_profile(mut profile: ResMut<Profile>) {
    let _ = profile.save();
}

fn read_profile(path: &Path) -> Option<ProfileData> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => return None,
        Err(_) => return None,
    };
    match serde_json::from_str(&contents) {
        Ok(profile) => Some(profile),
        Err(_) => {
            let backup = path.with_extension("corrupt.json");
            let _ = fs::rename(path, backup);
            None
        }
    }
}

fn write_profile(path: &Path, data: &ProfileData) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(data).map_err(std::io::Error::other)?;
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)
}

fn config_dir() -> Option<PathBuf> {
    if let Ok(config_home) = env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(config_home).join("polycore"));
    }
    if cfg!(target_os = "windows")
        && let Ok(app_data) = env::var("APPDATA")
    {
        return Some(PathBuf::from(app_data).join("Polycore"));
    }
    env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config").join("polycore"))
}

fn profile_path() -> Option<PathBuf> {
    config_dir().map(|path| path.join("profile.json"))
}

fn legacy_name_path() -> Option<PathBuf> {
    config_dir().map(|path| path.join("player_name.txt"))
}

pub fn sanitize_player_name(name: &str) -> String {
    name.trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(18)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("polycore-{name}-{}", std::process::id()))
    }

    #[test]
    fn profile_round_trip_and_unlocks_are_idempotent() {
        let root = test_path("profile-round-trip");
        let path = root.join("profile.json");
        let _ = fs::remove_dir_all(&root);
        let mut profile = Profile::load_from(Some(path.clone()), None);
        profile.set_player_name("  Pilot  ");
        assert!(profile.unlock(AchievementId::FirstKill));
        assert!(!profile.unlock(AchievementId::FirstKill));
        assert!(profile.select_palette(PaletteId::Ember));
        profile.save().unwrap();

        let loaded = Profile::load_from(Some(path), None);
        assert_eq!(loaded.data.identity.player_name, "Pilot");
        assert_eq!(loaded.data.selected_palette, PaletteId::Ember);
        assert!(loaded.data.achievements.contains(&AchievementId::FirstKill));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_name_is_migrated_and_corrupt_profile_is_backed_up() {
        let root = test_path("profile-migration");
        let path = root.join("profile.json");
        let legacy = root.join("player_name.txt");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(&legacy, "  Legacy\n").unwrap();
        let mut migrated = Profile::load_from(Some(path.clone()), Some(legacy));
        assert_eq!(migrated.data.identity.player_name, "Legacy");
        migrated.save().unwrap();
        fs::write(&path, "not json").unwrap();

        let recovered = Profile::load_from(Some(path.clone()), None);
        assert_eq!(recovered.data.version, PROFILE_VERSION);
        assert!(path.with_extension("corrupt.json").exists());
        let _ = fs::remove_dir_all(root);
    }
}
