use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig {
    pub initial_dt: f64,
    pub min_dt: f64,
    pub max_dt: f64,
    pub error_tolerance: f64,
    pub gravity: f64,
    pub record_interval: f64,
    pub max_flight_time_s: f64,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            initial_dt: 1.0e-4,
            min_dt: 1.0e-7,
            max_dt: 0.05,
            error_tolerance: 1.0e-6,
            gravity: 9.81,
            record_interval: 0.01,
            max_flight_time_s: 120.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtmosphereConfig {
    pub standard_air_density_kgm3: f64,
    pub standard_pressure_pa: f64,
    pub r_specific: f64,
    pub gamma: f64,
    pub sutherland_s: f64,
    pub sutherland_t0_k: f64,
    pub sutherland_mu0: f64,
    pub speed_of_sound_ref: f64,
}

impl Default for AtmosphereConfig {
    fn default() -> Self {
        Self {
            standard_air_density_kgm3: 1.225,
            standard_pressure_pa: 101325.0,
            r_specific: 287.058,
            gamma: 1.4,
            sutherland_s: 120.0,
            sutherland_t0_k: 291.15,
            sutherland_mu0: 1.827e-5,
            speed_of_sound_ref: 340.29,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialConfig {
    pub stone_density_kgm3: f64,
    pub counterweight_density_kgm3: f64,
}

impl Default for MaterialConfig {
    fn default() -> Self {
        Self {
            stone_density_kgm3: 2600.0,
            counterweight_density_kgm3: 7000.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiegeConfig {
    pub k_factor: f64,
    pub crater_depth_scaling: f64,
    pub crater_diameter_ratio: f64,
    pub penetration_weight: f64,
    pub diameter_weight_max: f64,
    pub damage_weight: f64,
    pub energy_weight: f64,
    pub mass_efficiency_weight: f64,
    pub break_energy_scaling: f64,
    pub mass_efficiency_scale_kg: f64,
}

impl Default for SiegeConfig {
    fn default() -> Self {
        Self {
            k_factor: 0.0001,
            crater_depth_scaling: 0.8,
            crater_diameter_ratio: 2.5,
            penetration_weight: 0.7,
            diameter_weight_max: 0.3,
            damage_weight: 60.0,
            energy_weight: 25.0,
            mass_efficiency_weight: 15.0,
            break_energy_scaling: 0.001,
            mass_efficiency_scale_kg: 200.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerConfig {
    pub angle_steps: usize,
    pub velocity_steps: usize,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            angle_steps: 20,
            velocity_steps: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpConfig {
    pub bind_addr: String,
    pub frame_magic: u32,
    pub frame_version: u8,
    pub frame_header_size: usize,
    pub max_payload_size: usize,
    pub stats_interval_secs: u64,
    pub buffer_capacity: usize,
}

impl Default for UdpConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:9001".to_string(),
            frame_magic: 0x53474553,
            frame_version: 1,
            frame_header_size: 15,
            max_payload_size: 65000,
            stats_interval_secs: 30,
            buffer_capacity: 65536,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub udp_to_ballistic_capacity: usize,
    pub ballistic_to_siege_capacity: usize,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            udp_to_ballistic_capacity: 1024,
            ballistic_to_siege_capacity: 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub sensor_buffer_limit: usize,
    pub ballistics_buffer_limit: usize,
    pub siege_buffer_limit: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            sensor_buffer_limit: 1000,
            ballistics_buffer_limit: 500,
            siege_buffer_limit: 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmoConfig {
    pub default_ammo: String,
    pub gunpowder_energy_density_jkg: f64,
    pub corpse_contamination_factor: f64,
}

impl Default for AmmoConfig {
    fn default() -> Self {
        Self {
            default_ammo: "round_stone".to_string(),
            gunpowder_energy_density_jkg: 3_000_000.0,
            corpse_contamination_factor: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FEAConfig {
    pub mesh_nx: usize,
    pub mesh_ny: usize,
    pub elastic_modulus_factor: f64,
    pub gate_center_ratio: f64,
    pub gate_width_ratio: f64,
    pub corner_stress_factor: f64,
}

impl Default for FEAConfig {
    fn default() -> Self {
        Self {
            mesh_nx: 20,
            mesh_ny: 15,
            elastic_modulus_factor: 1000.0,
            gate_center_ratio: 0.5,
            gate_width_ratio: 0.13,
            corner_stress_factor: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAConfig {
    pub population_size: usize,
    pub generations: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub elite_count: usize,
    pub tournament_size: usize,
}

impl Default for GAConfig {
    fn default() -> Self {
        Self {
            population_size: 60,
            generations: 50,
            mutation_rate: 0.15,
            crossover_rate: 0.8,
            elite_count: 4,
            tournament_size: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLConfig {
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub exploration_rate: f64,
    pub exploration_decay: f64,
    pub min_exploration_rate: f64,
    pub state_bins: usize,
    pub target_zones: usize,
}

impl Default for RLConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            discount_factor: 0.95,
            exploration_rate: 0.3,
            exploration_decay: 0.995,
            min_exploration_rate: 0.01,
            state_bins: 10,
            target_zones: 9,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub solver: SolverConfig,
    pub atmosphere: AtmosphereConfig,
    pub material: MaterialConfig,
    pub siege: SiegeConfig,
    pub optimizer: OptimizerConfig,
    pub udp: UdpConfig,
    pub channel: ChannelConfig,
    pub storage: StorageConfig,
    pub ammo: AmmoConfig,
    pub fea: FEAConfig,
    pub ga: GAConfig,
    pub rl: RLConfig,
}

impl AppConfig {
    pub fn load() -> Self {
        Self::default()
    }
}
