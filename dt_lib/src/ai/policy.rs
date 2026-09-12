//! Декларативные правила ИИ (serde, грузятся с карты). План §2.5 (BattlePolicy)
//! и §3.3 (MapPolicy — только структура+дефолты; логика в этапе 4).

use serde::{Deserialize, Serialize};

/// Веса архетипов для `unit_value` (план §2.5 target_priority).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TargetPriority {
    pub mage: f32,
    pub ranged: f32,
    pub melee: f32,
    /// Множитель сверх архетипного веса за is_main (герой).
    pub main: f32,
}
impl Default for TargetPriority {
    fn default() -> Self {
        Self {
            mage: 1.5,
            ranged: 1.3,
            melee: 1.0,
            main: 1.2,
        }
    }
}

/// Политика боя: жадный 1-pyl + позиционные правила (план §2.5).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BattlePolicy {
    /// 0..1 — вес атаки над самосохранением.
    pub aggression: f32,
    /// Бонус к score за добивание.
    pub kill_bonus: f32,
    /// Вес ответной угрозы при размене.
    pub threat_weight: f32,
    pub target_priority: TargetPriority,
    /// Стрелок/маг не должен стоять в Front — штраф позиции.
    pub keep_backline: bool,
    /// Юнит с hp ниже доли от max не лезет в размен (0 = не отступать).
    pub hp_threshold_retreat: f32,
    /// 0/1/2 — глубина lookahead (2-ply в этом спринте не реализуется).
    pub lookahead: u8,
    /// Разрешить каст-решения (фаза 2; пока не используется).
    pub cast_spells: bool,
    /// Skip вместо манёвра, когда выгодного хода нет.
    pub skip_when_no_gain: bool,
}
impl Default for BattlePolicy {
    fn default() -> Self {
        Self {
            aggression: 0.7,
            kill_bonus: 40.,
            threat_weight: 0.35,
            target_priority: TargetPriority::default(),
            keep_backline: true,
            hp_threshold_retreat: 0.,
            lookahead: 1,
            cast_spells: false,
            skip_when_no_gain: true,
        }
    }
}

/// Политика карты: perceive → assess → engage/retreat/patrol (план §3.3).
/// Только структура и дефолты — логика армий на карте в этапах 4-6.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MapPolicy {
    pub enabled: bool,
    /// Радиус восприятия в клетках пути.
    pub sight_radius: u32,
    /// Период решения в тиках.
    pub think_every_ticks: u32,
    /// Атаковать при my_power/their_power ≥ engage_ratio (1.2 = +20% перевес).
    pub engage_ratio: f32,
    /// Отступать ниже (0.8 = -20%).
    pub disengage_ratio: f32,
    /// Множитель силы обороняющегося в строении.
    pub defender_bonus: f32,
    /// Трогать ли нейтралов.
    pub hostile_neutral: bool,
    /// Не преследовать далеко от базы.
    pub hold_ground: bool,
    pub max_pursuit_radius: u32,
    /// Маршрут патруля.
    pub waypoints: Option<Vec<(usize, usize)>>,
    /// Фаза 2: брать золото вместо боя (Village-семантика).
    pub pay_mercy: bool,
}
impl Default for MapPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            sight_radius: 6,
            think_every_ticks: 10,
            engage_ratio: 1.2,
            disengage_ratio: 0.8,
            defender_bonus: 1.25,
            hostile_neutral: false,
            hold_ground: true,
            max_pursuit_radius: 8,
            waypoints: None,
            pay_mercy: false,
        }
    }
}

/// Настройки ИИ карты: опциональное поле в мете (план §3.4).
/// Отсутствие = дефолты (обратная совместимость).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct AiSettings {
    #[serde(default)]
    pub battle: BattlePolicy,
    #[serde(default)]
    pub map: MapPolicy,
}
