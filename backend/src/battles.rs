use crate::ammo::AmmoType;
use crate::siege::WallProperties;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleScenario {
    pub id: u32,
    pub name: String,
    pub year: i32,
    pub description: String,
    pub historical_context: String,
    pub attacker: String,
    pub defender: String,
    pub duration_days: u32,
    pub terrain: TerrainConfig,
    pub attacker_trebuchets: Vec<BattleTrebuchet>,
    pub wall: WallProperties,
    pub wall_display_name: String,
    pub victory_conditions: VictoryConditions,
    pub phases: Vec<BattlePhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainConfig {
    pub width_m: f64,
    pub depth_m: f64,
    pub elevation_change_m: f64,
    pub has_moat: bool,
    pub moat_width_m: f64,
    pub moat_depth_m: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleTrebuchet {
    pub trebuchet_id: u32,
    pub name: String,
    pub ammo_type: AmmoType,
    pub position_x: f64,
    pub position_z: f64,
    pub available_ammo: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VictoryConditions {
    pub wall_breach_required: f64,
    pub max_casualties_pct: f64,
    pub time_limit_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattlePhase {
    pub name: String,
    pub day_start: u32,
    pub day_end: u32,
    pub description: String,
    pub special_events: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleState {
    pub scenario_id: u32,
    pub current_day: u32,
    pub wall_damage: f64,
    pub total_impacts: u32,
    pub successful_hits: u32,
    pub ammo_remaining: HashMap<u32, u32>,
    pub is_victory: bool,
    pub is_defeat: bool,
    pub score: f64,
    pub impact_log: Vec<ImpactRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactRecord {
    pub day: u32,
    pub trebuchet_id: u32,
    pub target_x: f64,
    pub target_y: f64,
    pub ammo_type: AmmoType,
    pub damage_ratio: f64,
}

impl BattleState {
    pub fn new(scenario: &BattleScenario) -> Self {
        let mut ammo_remaining = HashMap::new();
        for bt in &scenario.attacker_trebuchets {
            ammo_remaining.insert(bt.trebuchet_id, bt.available_ammo);
        }
        Self {
            scenario_id: scenario.id,
            current_day: 1,
            wall_damage: 0.0,
            total_impacts: 0,
            successful_hits: 0,
            ammo_remaining,
            is_victory: false,
            is_defeat: false,
            score: 0.0,
            impact_log: Vec::new(),
        }
    }

    pub fn record_impact(
        &mut self,
        trebuchet_id: u32,
        target_x: f64,
        target_y: f64,
        ammo_type: AmmoType,
        damage_ratio: f64,
    ) {
        self.total_impacts += 1;
        self.wall_damage += damage_ratio;
        self.successful_hits += 1;

        if let Some(ammo) = self.ammo_remaining.get_mut(&trebuchet_id) {
            if *ammo > 0 {
                *ammo -= 1;
            }
        }

        self.impact_log.push(ImpactRecord {
            day: self.current_day,
            trebuchet_id,
            target_x,
            target_y,
            ammo_type,
            damage_ratio,
        });

        if self.wall_damage >= 1.0 {
            self.is_victory = true;
            self.score = self.calculate_score();
        }

        if self.current_day > 30 {
            self.is_defeat = !self.is_victory;
        }
    }

    pub fn advance_day(&mut self) {
        self.current_day += 1;
        if self.current_day > 30 && !self.is_victory {
            self.is_defeat = true;
        }
    }

    pub fn has_ammo(&self, trebuchet_id: u32) -> bool {
        self.ammo_remaining.get(&trebuchet_id).copied().unwrap_or(0) > 0
    }

    fn calculate_score(&self) -> f64 {
        let accuracy = if self.total_impacts > 0 {
            self.successful_hits as f64 / self.total_impacts as f64
        } else {
            0.0
        };
        let efficiency = if self.total_impacts > 0 {
            self.wall_damage / self.total_impacts as f64
        } else {
            0.0
        };
        let speed_bonus = (30 - self.current_day.min(30)) as f64 * 2.0;
        accuracy * 30.0 + efficiency * 40.0 + speed_bonus + 30.0
    }
}

pub fn get_historical_battles() -> Vec<BattleScenario> {
    vec![
        BattleScenario {
            id: 1,
            name: "襄阳之战".to_string(),
            year: 1267,
            description: "蒙古军队围攻南宋襄阳城，历时近6年，回回炮首次投入攻城战".to_string(),
            historical_context: "忽必烈征调西域工匠制造回回炮，射程远超宋军投石机，最终攻破襄阳".to_string(),
            attacker: "蒙古帝国".to_string(),
            defender: "南宋".to_string(),
            duration_days: 180,
            terrain: TerrainConfig {
                width_m: 200.0,
                depth_m: 150.0,
                elevation_change_m: 5.0,
                has_moat: true,
                moat_width_m: 15.0,
                moat_depth_m: 3.0,
            },
            attacker_trebuchets: vec![
                BattleTrebuchet { trebuchet_id: 1, name: "回回炮-甲".to_string(), ammo_type: AmmoType::RoundStone, position_x: -40.0, position_z: 20.0, available_ammo: 200 },
                BattleTrebuchet { trebuchet_id: 2, name: "回回炮-乙".to_string(), ammo_type: AmmoType::RoundStone, position_x: -35.0, position_z: -10.0, available_ammo: 150 },
                BattleTrebuchet { trebuchet_id: 3, name: "襄阳砲-壹".to_string(), ammo_type: AmmoType::GunpowderBomb, position_x: -50.0, position_z: 5.0, available_ammo: 50 },
            ],
            wall: WallProperties {
                thickness_m: 6.0,
                material: "double_rammed_earth".to_string(),
                density_kgm3: 1700.0,
                compressive_strength_pa: 1_800_000.0,
                tensile_strength_pa: 180_000.0,
            },
            wall_display_name: "襄阳城墙(双层夯土)".to_string(),
            victory_conditions: VictoryConditions {
                wall_breach_required: 0.6,
                max_casualties_pct: 0.3,
                time_limit_days: 180,
            },
            phases: vec![
                BattlePhase { name: "围城准备".to_string(), day_start: 1, day_end: 30, description: "建造攻城器械，试探性射击".to_string(), special_events: vec!["回回炮运抵前线".to_string()] },
                BattlePhase { name: "持续轰击".to_string(), day_start: 31, day_end: 120, description: "回回炮持续轰击城墙".to_string(), special_events: vec!["城墙出现裂缝".to_string()] },
                BattlePhase { name: "总攻突破".to_string(), day_start: 121, day_end: 180, description: "集中火力轰击弱点".to_string(), special_events: vec!["城墙崩塌".to_string()] },
            ],
        },
        BattleScenario {
            id: 2,
            name: "君士坦丁堡之围(模拟)".to_string(),
            year: 1453,
            description: "奥斯曼帝国使用巨型投石机围攻君士坦丁堡的狄奥多西城墙".to_string(),
            historical_context: "奥斯曼军队使用乌尔班大炮和投石机攻破千年之城".to_string(),
            attacker: "奥斯曼帝国".to_string(),
            defender: "拜占庭帝国".to_string(),
            duration_days: 53,
            terrain: TerrainConfig {
                width_m: 250.0,
                depth_m: 200.0,
                elevation_change_m: 8.0,
                has_moat: true,
                moat_width_m: 20.0,
                moat_depth_m: 4.0,
            },
            attacker_trebuchets: vec![
                BattleTrebuchet { trebuchet_id: 8, name: "无敌砲".to_string(), ammo_type: AmmoType::RoundStone, position_x: -50.0, position_z: 10.0, available_ammo: 300 },
                BattleTrebuchet { trebuchet_id: 10, name: "震天雷砲".to_string(), ammo_type: AmmoType::GunpowderBomb, position_x: -60.0, position_z: -5.0, available_ammo: 80 },
                BattleTrebuchet { trebuchet_id: 7, name: "虎蹲砲".to_string(), ammo_type: AmmoType::RoundStone, position_x: -35.0, position_z: 25.0, available_ammo: 200 },
                BattleTrebuchet { trebuchet_id: 1, name: "回回炮-甲".to_string(), ammo_type: AmmoType::CorpseShell, position_x: -40.0, position_z: -20.0, available_ammo: 40 },
            ],
            wall: WallProperties {
                thickness_m: 4.0,
                material: "stone_masonry".to_string(),
                density_kgm3: 2400.0,
                compressive_strength_pa: 25_000_000.0,
                tensile_strength_pa: 2_000_000.0,
            },
            wall_display_name: "狄奥多西城墙(石砌)".to_string(),
            victory_conditions: VictoryConditions {
                wall_breach_required: 0.5,
                max_casualties_pct: 0.4,
                time_limit_days: 53,
            },
            phases: vec![
                BattlePhase { name: "炮击准备".to_string(), day_start: 1, day_end: 10, description: "部署攻城器械".to_string(), special_events: vec!["巨型投石机组装".to_string()] },
                BattlePhase { name: "密集轰击".to_string(), day_start: 11, day_end: 40, description: "日夜不停轰击".to_string(), special_events: vec!["守军夜间修补城墙".to_string()] },
                BattlePhase { name: "最终突击".to_string(), day_start: 41, day_end: 53, description: "总攻突破".to_string(), special_events: vec!["城门被突破".to_string()] },
            ],
        },
        BattleScenario {
            id: 3,
            name: "太原攻防战".to_string(),
            year: 979,
            description: "北宋太宗亲征北汉太原城，使用投石机轰击坚固城墙".to_string(),
            historical_context: "宋军大量使用投石机攻城，最终灭北汉统一中原".to_string(),
            attacker: "北宋".to_string(),
            defender: "北汉".to_string(),
            duration_days: 90,
            terrain: TerrainConfig {
                width_m: 180.0,
                depth_m: 130.0,
                elevation_change_m: 10.0,
                has_moat: false,
                moat_width_m: 0.0,
                moat_depth_m: 0.0,
            },
            attacker_trebuchets: vec![
                BattleTrebuchet { trebuchet_id: 4, name: "人力砲-一号".to_string(), ammo_type: AmmoType::RoundStone, position_x: -30.0, position_z: 15.0, available_ammo: 300 },
                BattleTrebuchet { trebuchet_id: 5, name: "人力砲-二号".to_string(), ammo_type: AmmoType::RoundStone, position_x: -25.0, position_z: -10.0, available_ammo: 250 },
                BattleTrebuchet { trebuchet_id: 6, name: "旋风砲".to_string(), ammo_type: AmmoType::RoundStone, position_x: -28.0, position_z: 0.0, available_ammo: 200 },
                BattleTrebuchet { trebuchet_id: 9, name: "飞云砲".to_string(), ammo_type: AmmoType::CorpseShell, position_x: -32.0, position_z: -20.0, available_ammo: 30 },
            ],
            wall: WallProperties {
                thickness_m: 3.0,
                material: "rammed_earth".to_string(),
                density_kgm3: 1800.0,
                compressive_strength_pa: 2_000_000.0,
                tensile_strength_pa: 200_000.0,
            },
            wall_display_name: "太原城墙(夯土)".to_string(),
            victory_conditions: VictoryConditions {
                wall_breach_required: 0.5,
                max_casualties_pct: 0.2,
                time_limit_days: 90,
            },
            phases: vec![
                BattlePhase { name: "围城".to_string(), day_start: 1, day_end: 20, description: "宋军合围太原".to_string(), special_events: vec![] },
                BattlePhase { name: "攻城".to_string(), day_start: 21, day_end: 70, description: "人力砲轮番轰击".to_string(), special_events: vec!["北汉求援被拒".to_string()] },
                BattlePhase { name: "城破".to_string(), day_start: 71, day_end: 90, description: "城墙崩塌，北汉投降".to_string(), special_events: vec!["北汉灭亡".to_string()] },
            ],
        },
    ]
}

pub fn get_battle_by_id(id: u32) -> Option<BattleScenario> {
    get_historical_battles().into_iter().find(|b| b.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battle_scenarios() {
        let battles = get_historical_battles();
        assert!(!battles.is_empty());
        for b in &battles {
            assert!(!b.attacker_trebuchets.is_empty());
            assert!(b.wall.thickness_m > 0.0);
        }
    }

    #[test]
    fn test_battle_state() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        assert_eq!(state.current_day, 1);
        assert_eq!(state.wall_damage, 0.0);

        state.record_impact(1, 15.0, 5.0, AmmoType::RoundStone, 0.1);
        assert_eq!(state.total_impacts, 1);
        assert!((state.wall_damage - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_victory_condition() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        for _ in 0..20 {
            state.record_impact(1, 15.0, 5.0, AmmoType::RoundStone, 0.1);
        }
        assert!(state.is_victory);
        assert!(state.score > 0.0);
    }

    #[test]
    fn test_get_battle_by_id() {
        let battle = get_battle_by_id(1);
        assert!(battle.is_some());
        assert_eq!(battle.unwrap().name, "襄阳之战");
        let none = get_battle_by_id(999);
        assert!(none.is_none());
    }

    #[test]
    fn test_battle_state_initial_ammo() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let state = BattleState::new(&scenario);
        for bt in &scenario.attacker_trebuchets {
            assert_eq!(state.ammo_remaining.get(&bt.trebuchet_id), Some(&bt.available_ammo));
        }
    }

    #[test]
    fn test_ammo_consumption() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        let initial_ammo = *state.ammo_remaining.get(&1).unwrap();
        state.record_impact(1, 15.0, 5.0, AmmoType::RoundStone, 0.1);
        assert_eq!(*state.ammo_remaining.get(&1).unwrap(), initial_ammo - 1);
    }

    #[test]
    fn test_ammo_not_below_zero() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        let bt = &scenario.attacker_trebuchets[0];
        for _ in 0..(bt.available_ammo + 10) {
            state.record_impact(bt.trebuchet_id, 15.0, 5.0, AmmoType::RoundStone, 0.01);
        }
        assert_eq!(*state.ammo_remaining.get(&bt.trebuchet_id).unwrap(), 0);
    }

    #[test]
    fn test_impact_log_recording() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        state.record_impact(1, 15.0, 5.0, AmmoType::RoundStone, 0.1);
        state.record_impact(2, 10.0, 3.0, AmmoType::GunpowderBomb, 0.2);
        assert_eq!(state.impact_log.len(), 2);
        assert_eq!(state.impact_log[0].trebuchet_id, 1);
        assert_eq!(state.impact_log[1].ammo_type, AmmoType::GunpowderBomb);
    }

    #[test]
    fn test_advance_day() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        assert_eq!(state.current_day, 1);
        state.advance_day();
        assert_eq!(state.current_day, 2);
        state.advance_day();
        assert_eq!(state.current_day, 3);
    }

    #[test]
    fn test_impact_day_tracking() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        state.record_impact(1, 15.0, 5.0, AmmoType::RoundStone, 0.1);
        state.advance_day();
        state.record_impact(1, 15.0, 5.0, AmmoType::RoundStone, 0.1);
        assert_eq!(state.impact_log[0].day, 1);
        assert_eq!(state.impact_log[1].day, 2);
    }

    #[test]
    fn test_score_calculation() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        for _ in 0..20 {
            state.record_impact(1, 15.0, 5.0, AmmoType::RoundStone, 0.1);
        }
        assert!(state.is_victory);
        assert!(state.score > 30.0, "score should have base + accuracy + efficiency + speed components");
    }

    #[test]
    fn test_wall_damage_accumulates() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        state.record_impact(1, 15.0, 5.0, AmmoType::RoundStone, 0.1);
        assert!((state.wall_damage - 0.1).abs() < 0.001);
        state.record_impact(1, 15.0, 5.0, AmmoType::RoundStone, 0.15);
        assert!((state.wall_damage - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_has_ammo() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let state = BattleState::new(&scenario);
        assert!(state.has_ammo(1));
        assert!(!state.has_ammo(999));
    }

    #[test]
    fn test_victory_with_different_ammo() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        state.record_impact(3, 15.0, 5.0, AmmoType::GunpowderBomb, 0.2);
        state.record_impact(1, 15.0, 5.0, AmmoType::RoundStone, 0.1);
        assert!(state.wall_damage > 0.0);
        let log = &state.impact_log;
        assert_eq!(log[0].ammo_type, AmmoType::GunpowderBomb);
        assert_eq!(log[1].ammo_type, AmmoType::RoundStone);
    }

    #[test]
    fn test_three_historical_battles() {
        let battles = get_historical_battles();
        assert_eq!(battles.len(), 3);
        let ids: Vec<u32> = battles.iter().map(|b| b.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(ids.contains(&3));
    }

    #[test]
    fn test_xiangyang_battle_properties() {
        let battle = get_battle_by_id(1).unwrap();
        assert_eq!(battle.year, 1267);
        assert!(battle.terrain.has_moat);
        assert_eq!(battle.attacker_trebuchets.len(), 3);
        assert!(battle.victory_conditions.wall_breach_required > 0.0);
    }

    #[test]
    fn test_constantinople_battle_properties() {
        let battle = get_battle_by_id(2).unwrap();
        assert_eq!(battle.year, 1453);
        assert!(battle.wall.compressive_strength_pa > 10_000_000.0,
            "Constantinople walls should be very strong (stone masonry)");
        assert_eq!(battle.attacker_trebuchets.len(), 4);
    }

    #[test]
    fn test_taiyuan_battle_properties() {
        let battle = get_battle_by_id(3).unwrap();
        assert_eq!(battle.year, 979);
        assert!(!battle.terrain.has_moat);
        assert!(battle.phases.len() >= 2);
    }

    #[test]
    fn test_battle_phases() {
        for battle in get_historical_battles() {
            assert!(!battle.phases.is_empty(), "each battle should have at least one phase");
            for phase in &battle.phases {
                assert!(phase.day_start <= phase.day_end);
            }
        }
    }

    #[test]
    fn test_terrain_config_validity() {
        for battle in get_historical_battles() {
            assert!(battle.terrain.width_m > 0.0);
            assert!(battle.terrain.depth_m > 0.0);
            if battle.terrain.has_moat {
                assert!(battle.terrain.moat_width_m > 0.0);
                assert!(battle.terrain.moat_depth_m > 0.0);
            }
        }
    }

    #[test]
    fn test_victory_conditions_validity() {
        for battle in get_historical_battles() {
            assert!(battle.victory_conditions.wall_breach_required > 0.0
                && battle.victory_conditions.wall_breach_required <= 1.0);
            assert!(battle.victory_conditions.max_casualties_pct > 0.0
                && battle.victory_conditions.max_casualties_pct <= 1.0);
            assert!(battle.victory_conditions.time_limit_days > 0);
        }
    }

    #[test]
    fn test_defeat_condition_timeout() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        for _ in 0..35 {
            state.advance_day();
        }
        assert!(state.current_day > 30);
        assert!(!state.is_victory);
        assert!(state.is_defeat);
    }

    #[test]
    fn test_no_defeat_if_victory() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        for _ in 0..20 {
            state.record_impact(1, 15.0, 5.0, AmmoType::RoundStone, 0.1);
        }
        assert!(state.is_victory);
        for _ in 0..35 {
            state.advance_day();
        }
        assert!(state.is_victory);
        assert!(!state.is_defeat);
    }

    #[test]
    fn test_battle_trebuchet_positions() {
        for battle in get_historical_battles() {
            for bt in &battle.attacker_trebuchets {
                assert!(bt.available_ammo > 0, "trebuchet {} should have ammo", bt.trebuchet_id);
                assert!(bt.position_x < 0.0, "attacker trebuchet x should be negative (behind lines)");
            }
        }
    }

    #[test]
    fn test_corpse_shell_in_battle() {
        let constantinople = get_battle_by_id(2).unwrap();
        let has_corpse = constantinople.attacker_trebuchets.iter()
            .any(|bt| bt.ammo_type == AmmoType::CorpseShell);
        assert!(has_corpse, "Constantinople battle should have corpse shell trebuchet");
    }

    #[test]
    fn test_battle_state_serialization() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        state.record_impact(1, 15.0, 5.0, AmmoType::RoundStone, 0.1);
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: BattleState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_impacts, 1);
        assert!((deserialized.wall_damage - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_scenario_serialization() {
        let scenario = get_battle_by_id(1).unwrap();
        let json = serde_json::to_string(&scenario).unwrap();
        let deserialized: BattleScenario = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "襄阳之战");
    }

    #[test]
    fn test_successful_hits_increments() {
        let scenario = get_historical_battles().into_iter().next().unwrap();
        let mut state = BattleState::new(&scenario);
        assert_eq!(state.successful_hits, 0);
        state.record_impact(1, 15.0, 5.0, AmmoType::RoundStone, 0.1);
        assert_eq!(state.successful_hits, 1);
        state.record_impact(2, 10.0, 3.0, AmmoType::GunpowderBomb, 0.15);
        assert_eq!(state.successful_hits, 2);
    }

    #[test]
    fn test_multi_day_battle_simulation() {
        let scenario = get_battle_by_id(3).unwrap();
        let mut state = BattleState::new(&scenario);
        let mut day = 1;
        while !state.is_victory && !state.is_defeat && day < 100 {
            for bt in &scenario.attacker_trebuchets {
                if state.has_ammo(bt.trebuchet_id) {
                    let damage = match bt.ammo_type {
                        AmmoType::RoundStone => 0.02,
                        AmmoType::GunpowderBomb => 0.05,
                        AmmoType::CorpseShell => 0.01,
                    };
                    state.record_impact(bt.trebuchet_id, 15.0, 5.0, bt.ammo_type, damage);
                }
            }
            state.advance_day();
            day += 1;
        }
        assert!(state.is_victory || state.is_defeat || state.wall_damage > 0.0);
        assert!(state.total_impacts > 0);
        assert!(!state.impact_log.is_empty());
    }
}
