use crate::ballistic_simulator::BallisticEnvelope;
use crate::ballistics::estimate_projectile_diameter;
use crate::config::{AppConfig, SiegeConfig, MaterialConfig};
use crate::metrics;
use crate::siege::{assess_siege_damage, SiegeAssessment, SiegeInput, WallProperties};
use crate::storage::Database;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc::Receiver, Mutex};
use tracing::{debug, info, warn, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiegeEnvelope {
    pub trebuchet_id: u32,
    pub sensor_ts: DateTime<Utc>,
    pub assessed_at: DateTime<Utc>,
    pub input: SiegeInput,
    pub assessment: SiegeAssessment,
    pub optimal_angle_deg: f64,
    pub optimal_velocity_mps: f64,
}

pub struct SiegeEvaluator {
    config: Arc<AppConfig>,
    db: Arc<Database>,
    latest_siege: Arc<Mutex<HashMap<u32, SiegeAssessment>>>,
    rx: Receiver<BallisticEnvelope>,
}

impl SiegeEvaluator {
    pub fn new(
        config: Arc<AppConfig>,
        db: Arc<Database>,
        latest_siege: Arc<Mutex<HashMap<u32, SiegeAssessment>>>,
        rx: Receiver<BallisticEnvelope>,
    ) -> Self {
        Self {
            config,
            db,
            latest_siege,
            rx,
        }
    }

    pub async fn run(mut self) {
        info!(
            "[SiegeEvaluator] started (default_wall=rammed_earth {}m)",
            DEFAULT_WALL.thickness_m
        );
        let mut processed: u64 = 0;
        let mut timer = Instant::now();

        while let Some(env) = self.rx.recv().await {
            let assessed_at = Utc::now();

            let default_wall = Self::default_wall();

            let projectile_diameter = estimate_projectile_diameter(
                env.trebuchet_projectile_kg,
                self.config.material.stone_density_kgm3,
            );

            let eval_start = Instant::now();
            let (input, assessment) = Self::assess(
                &self.config.siege,
                env.result.impact_kinetic_energy_j,
                env.trebuchet_projectile_kg,
                projectile_diameter,
                env.result.impact_angle_deg,
                default_wall.clone(),
            );
            metrics::record_siege_duration(eval_start);
            metrics::increment_siege_assessments();

            {
                let mut guard = self.latest_siege.lock().await;
                guard.insert(env.trebuchet_id, assessment.clone());
            }

            let optimal_angle = 45.0f64;
            let optimal_velocity = env.input.initial_velocity;

            if let Err(e) = self
                .db
                .insert_siege_assessment(
                    env.trebuchet_id,
                    default_wall.thickness_m,
                    &default_wall.material,
                    default_wall.density_kgm3,
                    default_wall.compressive_strength_pa,
                    env.result.impact_kinetic_energy_j,
                    &assessment,
                    optimal_angle,
                    optimal_velocity,
                )
                .await
            {
                warn!("[SiegeEvaluator] persist failed: {}", e);
            }

            let _envelope = SiegeEnvelope {
                trebuchet_id: env.trebuchet_id,
                sensor_ts: env.sensor_ts,
                assessed_at,
                input,
                assessment,
                optimal_angle_deg: optimal_angle,
                optimal_velocity_mps: optimal_velocity,
            };

            processed += 1;

            if timer.elapsed() > std::time::Duration::from_secs(30) {
                if processed > 0 {
                    debug!(
                        "[SiegeEvaluator] processed={}, queue_depth={}",
                        processed,
                        self.rx.len()
                    );
                }
                processed = 0;
                timer = Instant::now();
            }
        }

        warn!("[SiegeEvaluator] channel closed, exiting");
    }

    fn assess(
        siege_cfg: &SiegeConfig,
        impact_energy_j: f64,
        projectile_mass_kg: f64,
        projectile_diameter_m: f64,
        impact_angle_deg: f64,
        wall: WallProperties,
    ) -> (SiegeInput, SiegeAssessment) {
        let _ = siege_cfg;
        let input = SiegeInput {
            impact_energy_j,
            projectile_mass_kg,
            projectile_diameter_m,
            impact_angle_deg,
            wall,
            ammo_type: crate::ammo::AmmoType::RoundStone,
        };
        let assessment = assess_siege_damage(&input);
        (input, assessment)
    }

    fn default_wall() -> WallProperties {
        WallProperties {
            thickness_m: DEFAULT_WALL.thickness_m,
            material: DEFAULT_WALL.material_str.to_string(),
            density_kgm3: DEFAULT_WALL.density_kgm3,
            compressive_strength_pa: DEFAULT_WALL.compressive_strength_pa,
            tensile_strength_pa: DEFAULT_WALL.tensile_strength_pa,
        }
    }
}

struct DefaultWall;
impl DefaultWall {
    const THICKNESS: f64 = 3.0;
    const MATERIAL: &'static str = "rammed_earth";
    const DENSITY: f64 = 1800.0;
    const COMPRESSIVE: f64 = 2_000_000.0;
    const TENSILE: f64 = 200_000.0;
}

static DEFAULT_WALL: DefaultWallStruct = DefaultWallStruct {
    thickness_m: DefaultWall::THICKNESS,
    material_str: "rammed_earth",
    density_kgm3: DefaultWall::DENSITY,
    compressive_strength_pa: DefaultWall::COMPRESSIVE,
    tensile_strength_pa: DefaultWall::TENSILE,
};

struct DefaultWallStruct {
    thickness_m: f64,
    material_str: &'static str,
    density_kgm3: f64,
    compressive_strength_pa: f64,
    tensile_strength_pa: f64,
}

impl DefaultWallStruct {
    fn clone(&self) -> WallProperties {
        WallProperties {
            thickness_m: self.thickness_m,
            material: self.material_str.to_string(),
            density_kgm3: self.density_kgm3,
            compressive_strength_pa: self.compressive_strength_pa,
            tensile_strength_pa: self.tensile_strength_pa,
        }
    }
}

#[allow(dead_code)]
pub fn _material_config_ref() -> &'static MaterialConfig {
    Box::leak(Box::new(MaterialConfig::default()))
}

#[allow(dead_code)]
pub fn _app_config_ref() -> &'static AppConfig {
    Box::leak(Box::new(AppConfig::default()))
}
