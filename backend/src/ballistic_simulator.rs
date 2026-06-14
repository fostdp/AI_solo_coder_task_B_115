use crate::ballistics::{
    estimate_projectile_diameter, simulate_ballistics, BallisticInput, BallisticResult,
};
use crate::config::{AppConfig, MaterialConfig, SolverConfig, AtmosphereConfig};
use crate::metrics;
use crate::storage::{Database, SensorData};
use crate::udp_receiver::{SensorEnvelope, UdpSensorMessage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc::Receiver, mpsc::Sender, Mutex};
use tracing::{debug, info, warn, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallisticEnvelope {
    pub trebuchet_id: u32,
    pub sensor_ts: DateTime<Utc>,
    pub computed_at: DateTime<Utc>,
    pub input: BallisticInput,
    pub result: BallisticResult,
    pub trebuchet_projectile_kg: f64,
    pub trebuchet_arm_length_m: f64,
}

pub struct BallisticSimulator {
    config: Arc<AppConfig>,
    db: Arc<Database>,
    latest_results: Arc<Mutex<HashMap<u32, BallisticResult>>>,
    rx: Receiver<SensorEnvelope>,
    tx_to_siege: Sender<BallisticEnvelope>,
}

impl BallisticSimulator {
    pub fn new(
        config: Arc<AppConfig>,
        db: Arc<Database>,
        latest_results: Arc<Mutex<HashMap<u32, BallisticResult>>>,
        rx: Receiver<SensorEnvelope>,
        tx_to_siege: Sender<BallisticEnvelope>,
    ) -> Self {
        Self {
            config,
            db,
            latest_results,
            rx,
            tx_to_siege,
        }
    }

    pub async fn run(mut self) {
        info!("[BallisticSimulator] started (RK4 + adaptive step)");
        let mut processed: u64 = 0;
        let mut timer = Instant::now();

        while let Some(envelope) = self.rx.recv().await {
            let SensorEnvelope {
                received_at,
                message,
                ..
            } = envelope;

            let computed_at = Utc::now();

            if let Some(trebuchet) = self
                .db
                .get_trebuchet_by_id(message.trebuchet_id)
                .await
            {
                let sensor_ts = message.timestamp.unwrap_or(received_at);

                self.persist_sensor_data(sensor_ts, &message).await;

                let sim_start = Instant::now();
                let (ballistic_input, result) = Self::compute_ballistics(
                    &message,
                    &trebuchet.projectile_kg,
                    &trebuchet.arm_length_m,
                    &self.config.material,
                );
                metrics::record_ballistic_duration(sim_start);
                metrics::increment_ballistic_simulations();
                metrics::gauge_ballistic_solver_steps(result.solver_steps);

                Self::update_latest_cache(
                    &self.latest_results,
                    message.trebuchet_id,
                    result.clone(),
                )
                .await;

                self.persist_ballistics_result(
                    message.trebuchet_id,
                    sensor_ts,
                    message.initial_velocity_mps,
                    message.launch_angle_deg,
                    &result,
                )
                .await;

                let out = BallisticEnvelope {
                    trebuchet_id: message.trebuchet_id,
                    sensor_ts,
                    computed_at,
                    input: ballistic_input,
                    result: result.clone(),
                    trebuchet_projectile_kg: trebuchet.projectile_kg,
                    trebuchet_arm_length_m: trebuchet.arm_length_m,
                };

                if let Err(e) = self.tx_to_siege.send(out).await {
                    warn!(
                        "[BallisticSimulator] failed to send to siege channel: {}",
                        e
                    );
                }
                metrics::gauge_ballistic_channel_depth(self.tx_to_siege.capacity());

                processed += 1;
            } else {
                warn!(
                    "[BallisticSimulator] unknown trebuchet id={}",
                    message.trebuchet_id
                );
            }

            if timer.elapsed() > std::time::Duration::from_secs(30) {
                if processed > 0 {
                    debug!(
                        "[BallisticSimulator] processed={}, queue_depth={}",
                        processed,
                        self.rx.len()
                    );
                }
                processed = 0;
                timer = Instant::now();
            }
        }

        warn!("[BallisticSimulator] channel closed, exiting");
    }

    fn compute_ballistics(
        msg: &UdpSensorMessage,
        projectile_kg: &f64,
        arm_length_m: &f64,
        material_cfg: &MaterialConfig,
    ) -> (BallisticInput, BallisticResult) {
        let _ = (solver_config_ref(), atmosphere_config_ref());
        let diameter = estimate_projectile_diameter(*projectile_kg, material_cfg.stone_density_kgm3);

        let input = BallisticInput {
            initial_velocity: msg.initial_velocity_mps,
            launch_angle_deg: msg.launch_angle_deg,
            projectile_mass_kg: *projectile_kg,
            projectile_diameter_m: diameter,
            air_density_kgm3: msg.air_density_kgm3,
            wind_speed_mps: msg.wind_speed_mps,
            wind_direction_deg: msg.wind_direction_deg,
            launch_height_m: arm_length_m * 0.4,
        };

        let result = simulate_ballistics(&input);
        (input, result)
    }

    async fn persist_sensor_data(&self, ts: DateTime<Utc>, msg: &UdpSensorMessage) {
        let data = SensorData {
            timestamp: ts,
            trebuchet_id: msg.trebuchet_id,
            cable_tension_newton: msg.cable_tension_newton,
            launch_angle_deg: msg.launch_angle_deg,
            initial_velocity_mps: msg.initial_velocity_mps,
            wind_speed_mps: msg.wind_speed_mps,
            wind_direction_deg: msg.wind_direction_deg,
            temperature_c: msg.temperature_c,
            air_density_kgm3: msg.air_density_kgm3,
        };

        if let Err(e) = self.db.insert_sensor_data(data).await {
            eprintln!("[BallisticSimulator] persist sensor failed: {}", e);
        }
    }

    async fn persist_ballistics_result(
        &self,
        trebuchet_id: u32,
        sensor_ts: DateTime<Utc>,
        velocity: f64,
        angle: f64,
        result: &BallisticResult,
    ) {
        if let Err(e) = self
            .db
            .insert_ballistics_result(trebuchet_id, sensor_ts, velocity, angle, result)
            .await
        {
            eprintln!("[BallisticSimulator] persist ballistics failed: {}", e);
        }
    }

    async fn update_latest_cache(
        cache: &Arc<Mutex<HashMap<u32, BallisticResult>>>,
        id: u32,
        result: BallisticResult,
    ) {
        let mut guard = cache.lock().await;
        guard.insert(id, result);
    }
}

fn solver_config_ref() -> &'static SolverConfig {
    Box::leak(Box::new(SolverConfig::default()))
}

fn atmosphere_config_ref() -> &'static AtmosphereConfig {
    Box::leak(Box::new(AtmosphereConfig::default()))
}
