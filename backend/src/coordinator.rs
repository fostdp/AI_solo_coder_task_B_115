use crate::ammo::AmmoType;
use crate::fea::ImpactLoad;
use crate::siege::WallProperties;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub exploration_rate: f64,
    pub exploration_decay: f64,
    pub min_exploration_rate: f64,
    pub state_bins: usize,
    pub target_zones: usize,
}

impl Default for CoordinatorConfig {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrebuchetState {
    pub id: u32,
    pub ammo_type: AmmoType,
    pub range_m: f64,
    pub reload_time_s: f64,
    pub ready: bool,
    pub assigned_target: Option<TargetAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetAssignment {
    pub target_x_m: f64,
    pub target_y_m: f64,
    pub priority: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallRegionState {
    pub x_m: f64,
    pub y_m: f64,
    pub width_m: f64,
    pub height_m: f64,
    pub damage_ratio: f64,
    pub stress_ratio: f64,
    pub strategic_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationResult {
    pub assignments: Vec<TrebuchetAssignment>,
    pub expected_total_damage: f64,
    pub coordination_efficiency: f64,
    pub q_table_size: usize,
    pub episodes_trained: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrebuchetAssignment {
    pub trebuchet_id: u32,
    pub target_x_m: f64,
    pub target_y_m: f64,
    pub ammo_type: AmmoType,
    pub expected_damage: f64,
    pub priority: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingEpisode {
    pub episode: u32,
    pub total_reward: f64,
    pub assignments: Vec<TrebuchetAssignment>,
}

pub struct QLearner {
    config: CoordinatorConfig,
    q_table: HashMap<u64, Vec<f64>>,
    state_bins: usize,
    episodes_trained: u32,
}

impl QLearner {
    pub fn new(config: CoordinatorConfig) -> Self {
        Self {
            config,
            q_table: HashMap::new(),
            state_bins: 10,
            episodes_trained: 0,
        }
    }

    pub fn choose_action(&mut self, state_key: u64, num_actions: usize) -> usize {
        if pseudo_random_f64() < self.config.exploration_rate {
            (pseudo_random_f64() * num_actions as f64) as usize
        } else {
            let q_values = self.q_table.get(&state_key).cloned().unwrap_or_else(|| vec![0.0; num_actions]);
            let mut best = 0;
            for i in 1..q_values.len() {
                if q_values[i] > q_values[best] {
                    best = i;
                }
            }
            best
        }
    }

    pub fn update(&mut self, state_key: u64, action: usize, reward: f64, next_state_key: u64, num_actions: usize) {
        let next_q_values = self.q_table.get(&next_state_key).cloned().unwrap_or_else(|| vec![0.0; num_actions]);
        let max_next_q = next_q_values.iter().cloned().fold(0.0_f64, f64::max);

        let q_values = self.q_table.entry(state_key).or_insert_with(|| vec![0.0; num_actions]);
        let current_q = q_values[action.min(q_values.len() - 1)];

        let new_q = current_q + self.config.learning_rate * (
            reward + self.config.discount_factor * max_next_q - current_q
        );

        if action < q_values.len() {
            q_values[action] = new_q;
        }
    }

    pub fn decay_exploration(&mut self) {
        self.config.exploration_rate = (self.config.exploration_rate * self.config.exploration_decay)
            .max(self.config.min_exploration_rate);
    }

    pub fn q_table_size(&self) -> usize {
        self.q_table.len()
    }
}

pub struct SiegeCoordinator {
    config: CoordinatorConfig,
    learner: QLearner,
    wall: WallProperties,
    num_target_zones: usize,
}

impl SiegeCoordinator {
    pub fn new(config: CoordinatorConfig, wall: WallProperties) -> Self {
        let learner = QLearner::new(config.clone());
        Self {
            config,
            learner,
            wall,
            num_target_zones: 9,
        }
    }

    pub fn coordinate(
        &mut self,
        trebuchets: &[TrebuchetState],
        wall_regions: &[WallRegionState],
        existing_impacts: &[ImpactLoad],
    ) -> CoordinationResult {
        let state_key = self.encode_state(wall_regions);
        let num_actions = self.num_target_zones;

        let mut assignments = Vec::new();
        let mut total_expected_damage = 0.0;

        for t in trebuchets {
            if !t.ready {
                continue;
            }

            let action = self.learner.choose_action(state_key, num_actions);
            let (target_x, target_y) = self.decode_action(action);

            let region_damage = self.estimate_damage_at_target(
                target_x, target_y, t, wall_regions, existing_impacts,
            );

            assignments.push(TrebuchetAssignment {
                trebuchet_id: t.id,
                target_x_m: target_x,
                target_y_m: target_y,
                ammo_type: t.ammo_type,
                expected_damage: region_damage,
                priority: 1.0 / (1.0 + t.reload_time_s / 60.0),
            });

            total_expected_damage += region_damage;
        }

        assignments.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));

        let coordination_efficiency = if !assignments.is_empty() {
            total_expected_damage / assignments.len() as f64
        } else {
            0.0
        };

        CoordinationResult {
            assignments,
            expected_total_damage: total_expected_damage,
            coordination_efficiency,
            q_table_size: self.learner.q_table_size(),
            episodes_trained: self.learner.episodes_trained,
        }
    }

    pub fn train_episode(
        &mut self,
        trebuchets: &[TrebuchetState],
        wall_regions: &[WallRegionState],
        existing_impacts: &[ImpactLoad],
        actual_damage: f64,
    ) -> TrainingEpisode {
        let state_key = self.encode_state(wall_regions);
        let num_actions = self.num_target_zones;

        let mut ep_assignments = Vec::new();
        let mut total_reward = 0.0;

        for t in trebuchets {
            let action = self.learner.choose_action(state_key, num_actions);
            let (target_x, target_y) = self.decode_action(action);

            let reward = self.compute_reward(target_x, target_y, wall_regions, actual_damage);

            let next_wall_regions = self.simulate_impact(wall_regions, target_x, target_y, actual_damage);
            let next_state_key = self.encode_state(&next_wall_regions);

            self.learner.update(state_key, action, reward, next_state_key, num_actions);

            ep_assignments.push(TrebuchetAssignment {
                trebuchet_id: t.id,
                target_x_m: target_x,
                target_y_m: target_y,
                ammo_type: t.ammo_type,
                expected_damage: reward,
                priority: reward,
            });

            total_reward += reward;
        }

        self.learner.decay_exploration();
        self.learner.episodes_trained += 1;

        TrainingEpisode {
            episode: self.learner.episodes_trained,
            total_reward,
            assignments: ep_assignments,
        }
    }

    fn encode_state(&self, regions: &[WallRegionState]) -> u64 {
        if regions.is_empty() {
            return 0;
        }

        let mut hash: u64 = 0;
        for (i, r) in regions.iter().take(9).enumerate() {
            let damage_bin = (r.damage_ratio * self.learner.state_bins as f64) as u64;
            let stress_bin = (r.stress_ratio * self.learner.state_bins as f64) as u64;
            let shift = (i * 8) as u64;
            hash ^= (damage_bin.min(self.learner.state_bins as u64) + stress_bin.min(self.learner.state_bins as u64)) << shift;
        }
        hash
    }

    fn decode_action(&self, action: usize) -> (f64, f64) {
        let grid_size = (self.num_target_zones as f64).sqrt() as usize;
        let row = action / grid_size;
        let col = action % grid_size;
        let cell_w = 30.0 / grid_size as f64;
        let cell_h = 10.0 / grid_size as f64;
        (
            (col as f64 + 0.5) * cell_w,
            (row as f64 + 0.5) * cell_h,
        )
    }

    fn estimate_damage_at_target(
        &self,
        x: f64,
        y: f64,
        trebuchet: &TrebuchetState,
        regions: &[WallRegionState],
        _existing_impacts: &[ImpactLoad],
    ) -> f64 {
        let mut base_damage = 0.0;

        for r in regions {
            let dx = (r.x_m - x).abs();
            let dy = (r.y_m - y).abs();
            if dx < r.width_m && dy < r.height_m {
                base_damage += r.strategic_value * (1.0 - r.damage_ratio) * 0.5;
            }
        }

        let ammo_factor = match trebuchet.ammo_type {
            AmmoType::RoundStone => 1.0,
            AmmoType::GunpowderBomb => 1.5,
            AmmoType::CorpseShell => 0.8,
        };

        let range_factor = if trebuchet.range_m > 0.0 {
            1.0 / (1.0 + (30.0 / trebuchet.range_m - 1.0).abs() * 0.1)
        } else {
            0.5
        };

        base_damage * ammo_factor * range_factor
    }

    fn compute_reward(
        &self,
        target_x: f64,
        target_y: f64,
        regions: &[WallRegionState],
        actual_damage: f64,
    ) -> f64 {
        let mut reward = actual_damage;

        for r in regions {
            let dx = (r.x_m - target_x).abs();
            let dy = (r.y_m - target_y).abs();
            if dx < r.width_m && dy < r.height_m {
                if r.damage_ratio > 0.7 {
                    reward += 10.0;
                }
                reward += r.strategic_value * 0.5;
            }
        }

        let gate_center = 15.0;
        let gate_dist = (target_x - gate_center).abs();
        if gate_dist < 3.0 {
            reward += 5.0 * (1.0 - gate_dist / 3.0);
        }

        reward
    }

    fn simulate_impact(
        &self,
        regions: &[WallRegionState],
        impact_x: f64,
        impact_y: f64,
        damage: f64,
    ) -> Vec<WallRegionState> {
        regions
            .iter()
            .map(|r| {
                let dx = (r.x_m - impact_x).abs();
                let dy = (r.y_m - impact_y).abs();
                let proximity_factor = if dx < r.width_m * 2.0 && dy < r.height_m * 2.0 {
                    1.0 / (1.0 + dx / r.width_m + dy / r.height_m)
                } else {
                    0.0
                };
                let new_damage = (r.damage_ratio + damage * proximity_factor * 0.01).min(1.0);
                WallRegionState {
                    damage_ratio: new_damage,
                    stress_ratio: r.stress_ratio * (1.0 + proximity_factor * 0.1),
                    ..r.clone()
                }
            })
            .collect()
    }
}

fn pseudo_random_f64() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let x = ns.wrapping_mul(6364136223846793005u64).wrapping_add(1442695040888963407u64);
    (x >> 33) as f64 / (1u64 << 31) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate() {
        let config = CoordinatorConfig {
            state_bins: 5,
            ..Default::default()
        };
        let wall = WallProperties {
            thickness_m: 3.0,
            material: "rammed_earth".to_string(),
            density_kgm3: 1800.0,
            compressive_strength_pa: 2_000_000.0,
            tensile_strength_pa: 200_000.0,
        };
        let mut coordinator = SiegeCoordinator::new(config, wall);

        let trebuchets = vec![
            TrebuchetState {
                id: 1,
                ammo_type: AmmoType::RoundStone,
                range_m: 200.0,
                reload_time_s: 60.0,
                ready: true,
                assigned_target: None,
            },
            TrebuchetState {
                id: 2,
                ammo_type: AmmoType::GunpowderBomb,
                range_m: 180.0,
                reload_time_s: 90.0,
                ready: true,
                assigned_target: None,
            },
        ];

        let regions = vec![WallRegionState {
            x_m: 15.0,
            y_m: 5.0,
            width_m: 10.0,
            height_m: 5.0,
            damage_ratio: 0.3,
            stress_ratio: 0.5,
            strategic_value: 1.0,
        }];

        let result = coordinator.coordinate(&trebuchets, &regions, &[]);
        assert!(!result.assignments.is_empty());
    }
}
