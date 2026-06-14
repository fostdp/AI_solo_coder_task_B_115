use crate::ballistics::BallisticResult;
use crate::siege::SiegeAssessment;
use crate::ammo::AmmoType;
use crate::battles::BattleState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallType {
    pub id: u32,
    pub name: String,
    pub material: String,
    pub thickness_m: f64,
    pub density_kgm3: f64,
    pub compressive_strength_pa: f64,
    pub tensile_strength_pa: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorData {
    pub timestamp: DateTime<Utc>,
    pub trebuchet_id: u32,
    pub cable_tension_newton: f64,
    pub launch_angle_deg: f64,
    pub initial_velocity_mps: f64,
    pub wind_speed_mps: f64,
    pub wind_direction_deg: f64,
    pub temperature_c: f64,
    pub air_density_kgm3: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrebuchetInfo {
    pub id: u32,
    pub name: String,
    pub type_: String,
    pub counterweight_kg: f64,
    pub projectile_kg: f64,
    pub arm_length_m: f64,
    pub max_angle_deg: f64,
}

#[derive(Debug, Clone)]
pub struct Database {
    trebuchets: Arc<Mutex<Vec<TrebuchetInfo>>>,
    wall_types: Arc<Mutex<Vec<WallType>>>,
    sensor_buffer: Arc<Mutex<Vec<SensorData>>>,
    ballistics_buffer: Arc<Mutex<Vec<BallisticsRecord>>>,
    siege_buffer: Arc<Mutex<Vec<SiegeRecord>>>,
    battle_states: Arc<Mutex<HashMap<u32, BattleState>>>,
}

#[derive(Debug, Clone, Serialize)]
struct BallisticsRecord {
    timestamp: DateTime<Utc>,
    trebuchet_id: u32,
    sensor_ts: DateTime<Utc>,
    initial_velocity_mps: f64,
    launch_angle_deg: f64,
    max_height_m: f64,
    range_m: f64,
    flight_time_s: f64,
    impact_velocity_mps: f64,
    impact_kinetic_energy_j: f64,
}

#[derive(Debug, Clone, Serialize)]
struct SiegeRecord {
    timestamp: DateTime<Utc>,
    trebuchet_id: u32,
    wall_thickness_m: f64,
    wall_material: String,
    wall_material_density: f64,
    wall_compressive_strength_pa: f64,
    impact_energy_j: f64,
    crater_depth_m: f64,
    crater_diameter_m: f64,
    damage_ratio: f64,
    effectiveness_score: f64,
    optimal_angle_deg: f64,
    optimal_velocity_mps: f64,
}

#[derive(Debug, Clone)]
struct BufferLimits {
    sensor: usize,
    ballistics: usize,
    siege: usize,
}

impl Default for BufferLimits {
    fn default() -> Self {
        Self {
            sensor: 1000,
            ballistics: 500,
            siege: 500,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StorageLimitsRef(pub BufferLimits);

impl Database {
    pub fn new() -> Self {
        Self::with_limits(BufferLimits::default())
    }

    pub fn new_with_config(cfg: &crate::config::StorageConfig) -> Self {
        Self::with_limits(BufferLimits {
            sensor: cfg.sensor_buffer_limit,
            ballistics: cfg.ballistics_buffer_limit,
            siege: cfg.siege_buffer_limit,
        })
    }

    fn with_limits(limits: BufferLimits) -> Self {
        let _limits = Arc::new(Mutex::new(limits));
        Self {
            trebuchets: Arc::new(Mutex::new(Vec::new())),
            wall_types: Arc::new(Mutex::new(Vec::new())),
            sensor_buffer: Arc::new(Mutex::new(Vec::new())),
            ballistics_buffer: Arc::new(Mutex::new(Vec::new())),
            siege_buffer: Arc::new(Mutex::new(Vec::new())),
            battle_states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn load_wall_types(&self) -> Result<Vec<WallType>, String> {
        let walls = vec![
            WallType {
                id: 1,
                name: "夯土墙".to_string(),
                material: "rammed_earth".to_string(),
                thickness_m: 3.0,
                density_kgm3: 1800.0,
                compressive_strength_pa: 2_000_000.0,
                tensile_strength_pa: 200_000.0,
            },
            WallType {
                id: 2,
                name: "包砖墙".to_string(),
                material: "brick_veneer".to_string(),
                thickness_m: 2.5,
                density_kgm3: 2000.0,
                compressive_strength_pa: 10_000_000.0,
                tensile_strength_pa: 800_000.0,
            },
            WallType {
                id: 3,
                name: "石砌墙".to_string(),
                material: "stone_masonry".to_string(),
                thickness_m: 4.0,
                density_kgm3: 2400.0,
                compressive_strength_pa: 25_000_000.0,
                tensile_strength_pa: 2_000_000.0,
            },
            WallType {
                id: 4,
                name: "双层夯土墙".to_string(),
                material: "double_rammed_earth".to_string(),
                thickness_m: 6.0,
                density_kgm3: 1700.0,
                compressive_strength_pa: 1_800_000.0,
                tensile_strength_pa: 180_000.0,
            },
            WallType {
                id: 5,
                name: "糯米灰浆墙".to_string(),
                material: "sticky_rice_lime".to_string(),
                thickness_m: 3.5,
                density_kgm3: 2100.0,
                compressive_strength_pa: 15_000_000.0,
                tensile_strength_pa: 1_200_000.0,
            },
        ];

        {
            let mut w = self.wall_types.lock().await;
            *w = walls.clone();
        }

        Ok(walls)
    }

    pub async fn get_wall_types(&self) -> Vec<WallType> {
        self.wall_types.lock().await.clone()
    }

    pub async fn get_wall_by_id(&self, id: u32) -> Option<WallType> {
        self.wall_types
            .lock()
            .await
            .iter()
            .find(|w| w.id == id)
            .cloned()
    }

    pub async fn load_trebuchets(&self) -> Result<Vec<TrebuchetInfo>, String> {
        let mut trebuchets = vec![
            TrebuchetInfo {
                id: 1,
                name: "回回炮-甲".to_string(),
                type_: "配重式".to_string(),
                counterweight_kg: 3000.0,
                projectile_kg: 90.0,
                arm_length_m: 12.0,
                max_angle_deg: 50.0,
            },
            TrebuchetInfo {
                id: 2,
                name: "回回炮-乙".to_string(),
                type_: "配重式".to_string(),
                counterweight_kg: 5000.0,
                projectile_kg: 150.0,
                arm_length_m: 15.0,
                max_angle_deg: 55.0,
            },
            TrebuchetInfo {
                id: 3,
                name: "襄阳砲-壹".to_string(),
                type_: "配重式".to_string(),
                counterweight_kg: 4000.0,
                projectile_kg: 120.0,
                arm_length_m: 13.5,
                max_angle_deg: 52.0,
            },
            TrebuchetInfo {
                id: 4,
                name: "人力砲-一号".to_string(),
                type_: "人力牵引式".to_string(),
                counterweight_kg: 0.0,
                projectile_kg: 30.0,
                arm_length_m: 8.0,
                max_angle_deg: 45.0,
            },
            TrebuchetInfo {
                id: 5,
                name: "人力砲-二号".to_string(),
                type_: "人力牵引式".to_string(),
                counterweight_kg: 0.0,
                projectile_kg: 25.0,
                arm_length_m: 7.5,
                max_angle_deg: 42.0,
            },
            TrebuchetInfo {
                id: 6,
                name: "旋风砲".to_string(),
                type_: "人力牵引式".to_string(),
                counterweight_kg: 0.0,
                projectile_kg: 20.0,
                arm_length_m: 6.0,
                max_angle_deg: 48.0,
            },
            TrebuchetInfo {
                id: 7,
                name: "虎蹲砲".to_string(),
                type_: "配重式".to_string(),
                counterweight_kg: 1500.0,
                projectile_kg: 50.0,
                arm_length_m: 9.0,
                max_angle_deg: 47.0,
            },
            TrebuchetInfo {
                id: 8,
                name: "无敌砲".to_string(),
                type_: "配重式".to_string(),
                counterweight_kg: 6000.0,
                projectile_kg: 200.0,
                arm_length_m: 18.0,
                max_angle_deg: 58.0,
            },
            TrebuchetInfo {
                id: 9,
                name: "飞云砲".to_string(),
                type_: "人力牵引式".to_string(),
                counterweight_kg: 0.0,
                projectile_kg: 15.0,
                arm_length_m: 5.5,
                max_angle_deg: 40.0,
            },
            TrebuchetInfo {
                id: 10,
                name: "震天雷砲".to_string(),
                type_: "配重式".to_string(),
                counterweight_kg: 8000.0,
                projectile_kg: 300.0,
                arm_length_m: 20.0,
                max_angle_deg: 60.0,
            },
        ];

        {
            let mut t = self.trebuchets.lock().await;
            *t = trebuchets.clone();
        }

        Ok(trebuchets)
    }

    pub async fn get_trebuchets(&self) -> Vec<TrebuchetInfo> {
        self.trebuchets.lock().await.clone()
    }

    pub async fn get_trebuchet_by_id(&self, id: u32) -> Option<TrebuchetInfo> {
        self.trebuchets
            .lock()
            .await
            .iter()
            .find(|t| t.id == id)
            .cloned()
    }

    pub async fn insert_sensor_data(&self, data: SensorData) -> Result<(), String> {
        let mut buffer = self.sensor_buffer.lock().await;
        buffer.push(data);
        let limit = 1000usize;
        if buffer.len() > limit {
            let excess = buffer.len() - limit;
            buffer.drain(0..excess);
        }
        Ok(())
    }

    pub async fn insert_ballistics_result(
        &self,
        trebuchet_id: u32,
        sensor_ts: DateTime<Utc>,
        initial_velocity_mps: f64,
        launch_angle_deg: f64,
        result: &BallisticResult,
    ) -> Result<(), String> {
        let record = BallisticsRecord {
            timestamp: Utc::now(),
            trebuchet_id,
            sensor_ts,
            initial_velocity_mps,
            launch_angle_deg,
            max_height_m: result.max_height_m,
            range_m: result.range_m,
            flight_time_s: result.flight_time_s,
            impact_velocity_mps: result.impact_velocity_mps,
            impact_kinetic_energy_j: result.impact_kinetic_energy_j,
        };

        let mut buffer = self.ballistics_buffer.lock().await;
        buffer.push(record);
        let limit = 500usize;
        if buffer.len() > limit {
            let excess = buffer.len() - limit;
            buffer.drain(0..excess);
        }
        Ok(())
    }

    pub async fn insert_siege_assessment(
        &self,
        trebuchet_id: u32,
        wall_thickness_m: f64,
        wall_material: &str,
        wall_density: f64,
        wall_compressive_strength: f64,
        impact_energy_j: f64,
        assessment: &SiegeAssessment,
        optimal_angle_deg: f64,
        optimal_velocity_mps: f64,
    ) -> Result<(), String> {
        let record = SiegeRecord {
            timestamp: Utc::now(),
            trebuchet_id,
            wall_thickness_m,
            wall_material: wall_material.to_string(),
            wall_material_density: wall_density,
            wall_compressive_strength_pa: wall_compressive_strength,
            impact_energy_j,
            crater_depth_m: assessment.crater_depth_m,
            crater_diameter_m: assessment.crater_diameter_m,
            damage_ratio: assessment.damage_ratio,
            effectiveness_score: assessment.effectiveness_score,
            optimal_angle_deg,
            optimal_velocity_mps,
        };

        let mut buffer = self.siege_buffer.lock().await;
        buffer.push(record);
        let limit = 500usize;
        if buffer.len() > limit {
            let excess = buffer.len() - limit;
            buffer.drain(0..excess);
        }
        Ok(())
    }

    pub async fn get_recent_sensor_data(&self, trebuchet_id: u32, limit: usize) -> Vec<SensorData> {
        let buffer = self.sensor_buffer.lock().await;
        buffer
            .iter()
            .filter(|s| s.trebuchet_id == trebuchet_id)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn get_recent_ballistics(&self, trebuchet_id: u32, limit: usize) -> Vec<BallisticsRecord> {
        let buffer = self.ballistics_buffer.lock().await;
        buffer
            .iter()
            .filter(|b| b.trebuchet_id == trebuchet_id)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn get_recent_siege(&self, trebuchet_id: u32, limit: usize) -> Vec<SiegeRecord> {
        let buffer = self.siege_buffer.lock().await;
        buffer
            .iter()
            .filter(|s| s.trebuchet_id == trebuchet_id)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn get_all_sensor_data(&self) -> Vec<SensorData> {
        self.sensor_buffer.lock().await.clone()
    }

    pub async fn get_battle_state(&self, scenario_id: u32) -> Option<BattleState> {
        self.battle_states.lock().await.get(&scenario_id).cloned()
    }

    pub async fn set_battle_state(&self, scenario_id: u32, state: BattleState) {
        self.battle_states.lock().await.insert(scenario_id, state);
    }

    pub async fn get_all_battle_states(&self) -> HashMap<u32, BattleState> {
        self.battle_states.lock().await.clone()
    }
}
