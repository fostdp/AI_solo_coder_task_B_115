use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use crate::ammo::AmmoType;
use crate::coordinator::{
    SiegeCoordinator, CoordinatorConfig, TrebuchetState, WallRegionState,
    CoordinationResult,
};
use crate::siege::WallProperties;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrainingStatus {
    Idle,
    Training,
    Completed,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OptimizerConfig {
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub exploration_rate: f64,
    pub exploration_decay: f64,
    pub min_exploration_rate: f64,
    pub state_bins: usize,
    pub target_zones: usize,
    pub use_fast_dynamics: bool,
    pub training_episodes: usize,
    pub background_training: bool,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            discount_factor: 0.95,
            exploration_rate: 0.3,
            exploration_decay: 0.995,
            min_exploration_rate: 0.01,
            state_bins: 10,
            target_zones: 9,
            use_fast_dynamics: true,
            training_episodes: 100,
            background_training: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingProgress {
    pub status: TrainingStatus,
    pub current_episode: usize,
    pub total_episodes: usize,
    pub current_reward: f64,
    pub best_reward: f64,
    pub average_reward: f64,
    pub q_table_size: usize,
    pub episodes_trained: usize,
    pub training_history: Vec<f64>,
}

impl Default for TrainingProgress {
    fn default() -> Self {
        Self {
            status: TrainingStatus::Idle,
            current_episode: 0,
            total_episodes: 0,
            current_reward: 0.0,
            best_reward: 0.0,
            average_reward: 0.0,
            q_table_size: 0,
            episodes_trained: 0,
            training_history: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalvoOptimizerResult {
    pub coordination: CoordinationResult,
    pub training_progress: TrainingProgress,
    pub recommendations: Vec<String>,
}

enum TrainingCommand {
    Start,
    Stop,
    GetProgress,
    TrainEpisodes(usize),
}

enum TrainingResponse {
    Progress(TrainingProgress),
    Started,
    Stopped,
}

pub struct SalvoOptimizer {
    coordinator: Arc<Mutex<SiegeCoordinator>>,
    config: OptimizerConfig,
    wall: WallProperties,
    progress: Arc<RwLock<TrainingProgress>>,
    training_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    command_tx: Arc<Mutex<Option<mpsc::Sender<TrainingCommand>>>>,
}

impl SalvoOptimizer {
    pub fn new(config: OptimizerConfig, wall: WallProperties) -> Self {
        let coord_config = CoordinatorConfig {
            learning_rate: config.learning_rate,
            discount_factor: config.discount_factor,
            exploration_rate: config.exploration_rate,
            exploration_decay: config.exploration_decay,
            min_exploration_rate: config.min_exploration_rate,
            state_bins: config.state_bins,
            target_zones: config.target_zones,
            use_fast_dynamics: config.use_fast_dynamics,
        };

        let coordinator = SiegeCoordinator::new(coord_config, wall.clone());

        Self {
            coordinator: Arc::new(Mutex::new(coordinator)),
            config,
            wall,
            progress: Arc::new(RwLock::new(TrainingProgress::default())),
            training_handle: Arc::new(Mutex::new(None)),
            command_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub fn from_wall(wall: &WallProperties) -> Self {
        Self::new(OptimizerConfig::default(), wall.clone())
    }

    pub async fn optimize(
        &self,
        trebuchets: &[TrebuchetState],
        wall_regions: &[WallRegionState],
        existing_impacts: &[crate::fea::ImpactLoad],
    ) -> SalvoOptimizerResult {
        let current_progress = self.get_progress().await;

        let result = {
            let mut coord = self.coordinator.lock().await;
            coord.coordinate(trebuchets, wall_regions, existing_impacts)
        };

        let mut recommendations = Vec::new();

        if current_progress.status == TrainingStatus::Idle && self.config.background_training {
            recommendations.push("💡 后台训练未启动，建议先调用 start_training() 预热模型".to_string());
        }

        if current_progress.episodes_trained < 10 {
            recommendations.push("⚠️ 训练样本不足，分配结果可能不够优化".to_string());
        }

        if result.episodes_trained > 100 {
            recommendations.push("✅ 模型已充分训练，分配结果具有较高可信度".to_string());
        }

        if self.config.use_fast_dynamics {
            recommendations.push("⚡ 已启用快速动力学模式，训练速度提升 10~100 倍".to_string());
        }

        let ready_count = trebuchets.iter().filter(|t| t.ready).count();
        if ready_count < trebuchets.len() {
            recommendations.push(format!(
                "⚠️ {}/{} 台投石机未就绪，已自动跳过",
                trebuchets.len() - ready_count,
                trebuchets.len()
            ));
        }

        recommendations.push(format!(
            "📊 本次分配 {} 台投石机，预期总伤害 {:.1}%，协同效率 {:.1}%",
            result.assignments.len(),
            result.expected_total_damage * 100.0,
            result.coordination_efficiency * 100.0
        ));

        SalvoOptimizerResult {
            coordination: result,
            training_progress: current_progress,
            recommendations,
        }
    }

    pub async fn start_training(&self) {
        let mut handle_guard = self.training_handle.lock().await;
        if handle_guard.is_some() {
            return;
        }

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<TrainingCommand>(32);
        *self.command_tx.lock().await = Some(cmd_tx);

        let coordinator = self.coordinator.clone();
        let progress = self.progress.clone();
        let config = self.config;
        let wall = self.wall.clone();

        let handle = tokio::spawn(async move {
            let mut progress_guard = progress.write().await;
            progress_guard.status = TrainingStatus::Training;
            progress_guard.total_episodes = config.training_episodes;
            drop(progress_guard);

            let trebuchets = Self::generate_training_trebuchets();
            let wall_regions = Self::generate_training_regions(&wall);

            let mut episode = 0;
            let mut best_reward = 0.0_f64;
            let mut reward_history: Vec<f64> = Vec::new();

            while episode < config.training_episodes {
                tokio::select! {
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(TrainingCommand::Stop) => {
                                let mut pg = progress.write().await;
                                pg.status = TrainingStatus::Idle;
                                return;
                            }
                            Some(TrainingCommand::GetProgress) => {
                                continue;
                            }
                            Some(TrainingCommand::TrainEpisodes(n)) => {
                                for _ in 0..n {
                                    let ep_result = {
                                        let mut coord = coordinator.lock().await;
                                        if config.use_fast_dynamics {
                                            coord.train_episode_fast(&trebuchets, &wall_regions)
                                        } else {
                                            coord.train_episode_fast(&trebuchets, &wall_regions)
                                        }
                                    };

                                    episode += 1;
                                    reward_history.push(ep_result.total_reward);
                                    best_reward = best_reward.max(ep_result.total_reward);

                                    let avg_reward = if !reward_history.is_empty() {
                                        reward_history.iter().sum::<f64>() / reward_history.len() as f64
                                    } else {
                                        0.0
                                    };

                                    let q_size = {
                                        let coord = coordinator.lock().await;
                                        coord.q_table_size()
                                    };

                                    let mut pg = progress.write().await;
                                    pg.current_episode = episode;
                                    pg.current_reward = ep_result.total_reward;
                                    pg.best_reward = best_reward;
                                    pg.average_reward = avg_reward;
                                    pg.q_table_size = q_size;
                                    pg.episodes_trained = episode;
                                    if reward_history.len() > 100 {
                                        pg.training_history = reward_history[reward_history.len() - 100..].to_vec();
                                    } else {
                                        pg.training_history = reward_history.clone();
                                    }
                                }
                            }
                            Some(TrainingCommand::Start) => continue,
                            None => return,
                        }
                    }
                    else => {
                        let ep_result = {
                            let mut coord = coordinator.lock().await;
                            coord.train_episode_fast(&trebuchets, &wall_regions)
                        };

                        episode += 1;
                        reward_history.push(ep_result.total_reward);
                        best_reward = best_reward.max(ep_result.total_reward);

                        let avg_reward = if !reward_history.is_empty() {
                            reward_history.iter().sum::<f64>() / reward_history.len() as f64
                        } else {
                            0.0
                        };

                        let q_size = {
                            let coord = coordinator.lock().await;
                            coord.q_table_size()
                        };

                        let mut pg = progress.write().await;
                        pg.current_episode = episode;
                        pg.current_reward = ep_result.total_reward;
                        pg.best_reward = best_reward;
                        pg.average_reward = avg_reward;
                        pg.q_table_size = q_size;
                        pg.episodes_trained = episode;
                        if reward_history.len() > 100 {
                            pg.training_history = reward_history[reward_history.len() - 100..].to_vec();
                        } else {
                            pg.training_history = reward_history.clone();
                        }

                        if episode >= config.training_episodes {
                            pg.status = TrainingStatus::Completed;
                            break;
                        }

                        drop(pg);
                        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                    }
                }
            }

            let mut pg = progress.write().await;
            if pg.status != TrainingStatus::Error {
                pg.status = TrainingStatus::Completed;
            }
        });

        *handle_guard = Some(handle);

        if self.config.background_training {
            if let Some(tx) = self.command_tx.lock().await.as_ref() {
                let _ = tx.send(TrainingCommand::TrainEpisodes(self.config.training_episodes)).await;
            }
        }
    }

    pub async fn stop_training(&self) {
        if let Some(tx) = self.command_tx.lock().await.as_ref() {
            let _ = tx.send(TrainingCommand::Stop).await;
        }

        if let Some(handle) = self.training_handle.lock().await.take() {
            handle.abort();
        }

        let mut pg = self.progress.write().await;
        pg.status = TrainingStatus::Idle;
    }

    pub async fn get_progress(&self) -> TrainingProgress {
        self.progress.read().await.clone()
    }

    pub async fn train_additional_episodes(&self, episodes: usize) {
        if let Some(tx) = self.command_tx.lock().await.as_ref() {
            let _ = tx.send(TrainingCommand::TrainEpisodes(episodes)).await;
        }
    }

    pub async fn wait_for_completion(&self, timeout_ms: Option<u64>) -> TrainingProgress {
        let start = std::time::Instant::now();
        loop {
            let progress = self.get_progress().await;
            if progress.status != TrainingStatus::Training {
                return progress;
            }
            if let Some(timeout) = timeout_ms {
                if start.elapsed().as_millis() > timeout as u128 {
                    return progress;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    fn generate_training_trebuchets() -> Vec<TrebuchetState> {
        vec![
            TrebuchetState {
                id: 1, ammo_type: AmmoType::RoundStone, range_m: 200.0,
                reload_time_s: 60.0, ready: true, assigned_target: None,
            },
            TrebuchetState {
                id: 2, ammo_type: AmmoType::RoundStone, range_m: 200.0,
                reload_time_s: 60.0, ready: true, assigned_target: None,
            },
            TrebuchetState {
                id: 3, ammo_type: AmmoType::RoundStone, range_m: 200.0,
                reload_time_s: 60.0, ready: true, assigned_target: None,
            },
        ]
    }

    fn generate_training_regions(wall: &WallProperties) -> Vec<WallRegionState> {
        vec![
            WallRegionState {
                x_m: 10.0, y_m: 5.0, width_m: 10.0, height_m: 5.0,
                damage_ratio: 0.0, stress_ratio: 0.3, strategic_value: 0.5,
            },
            WallRegionState {
                x_m: 20.0, y_m: 5.0, width_m: 10.0, height_m: 5.0,
                damage_ratio: 0.0, stress_ratio: 0.3, strategic_value: 1.0,
            },
        ]
    }
}

pub async fn create_salvo_optimizer(
    wall: &WallProperties,
    start_training: bool,
) -> Arc<SalvoOptimizer> {
    let optimizer = Arc::new(SalvoOptimizer::from_wall(wall));
    if start_training {
        optimizer.start_training().await;
    }
    optimizer
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_wall() -> WallProperties {
        WallProperties {
            thickness_m: 3.0,
            material: "rammed_earth".to_string(),
            density_kgm3: 1800.0,
            compressive_strength_pa: 2_000_000.0,
            tensile_strength_pa: 200_000.0,
        }
    }

    #[tokio::test]
    async fn test_optimizer_creation() {
        let wall = default_wall();
        let optimizer = SalvoOptimizer::new(OptimizerConfig::default(), wall);
        let progress = optimizer.get_progress().await;

        assert_eq!(progress.status, TrainingStatus::Idle);
        assert_eq!(progress.episodes_trained, 0);
    }

    #[tokio::test]
    async fn test_optimizer_from_wall() {
        let wall = default_wall();
        let optimizer = SalvoOptimizer::from_wall(&wall);
        let progress = optimizer.get_progress().await;

        assert_eq!(progress.status, TrainingStatus::Idle);
    }

    #[tokio::test]
    async fn test_optimizer_config_defaults() {
        let config = OptimizerConfig::default();
        assert_eq!(config.learning_rate, 0.1);
        assert_eq!(config.discount_factor, 0.95);
        assert_eq!(config.exploration_rate, 0.3);
        assert_eq!(config.target_zones, 9);
        assert_eq!(config.use_fast_dynamics, true);
        assert_eq!(config.training_episodes, 100);
    }

    #[tokio::test]
    async fn test_optimizer_optimize_basic() {
        let wall = default_wall();
        let optimizer = SalvoOptimizer::new(OptimizerConfig::default(), wall);

        let trebuchets = vec![
            TrebuchetState {
                id: 1, ammo_type: AmmoType::RoundStone, range_m: 200.0,
                reload_time_s: 60.0, ready: true, assigned_target: None,
            },
        ];

        let regions = vec![
            WallRegionState {
                x_m: 15.0, y_m: 5.0, width_m: 10.0, height_m: 5.0,
                damage_ratio: 0.0, stress_ratio: 0.5, strategic_value: 1.0,
            },
        ];

        let impacts = vec![];
        let result = optimizer.optimize(&trebuchets, &regions, &impacts).await;

        assert!(result.coordination.assignments.len() > 0);
        assert!(result.coordination.expected_total_damage > 0.0);
        assert!(result.recommendations.len() > 0);
    }

    #[tokio::test]
    async fn test_optimizer_units_not_ready() {
        let wall = default_wall();
        let optimizer = SalvoOptimizer::new(OptimizerConfig::default(), wall);

        let trebuchets = vec![
            TrebuchetState {
                id: 1, ammo_type: AmmoType::RoundStone, range_m: 200.0,
                reload_time_s: 60.0, ready: true, assigned_target: None,
            },
            TrebuchetState {
                id: 2, ammo_type: AmmoType::RoundStone, range_m: 200.0,
                reload_time_s: 60.0, ready: false, assigned_target: None,
            },
        ];

        let regions = vec![
            WallRegionState {
                x_m: 15.0, y_m: 5.0, width_m: 10.0, height_m: 5.0,
                damage_ratio: 0.0, stress_ratio: 0.5, strategic_value: 1.0,
            },
        ];

        let impacts = vec![];
        let result = optimizer.optimize(&trebuchets, &regions, &impacts).await;

        assert_eq!(result.coordination.assignments.len(), 1);
        assert!(result.recommendations.iter().any(|r| r.contains("未就绪")));
    }

    #[tokio::test]
    async fn test_optimizer_no_ready_units() {
        let wall = default_wall();
        let optimizer = SalvoOptimizer::new(OptimizerConfig::default(), wall);

        let trebuchets = vec![
            TrebuchetState {
                id: 1, ammo_type: AmmoType::RoundStone, range_m: 200.0,
                reload_time_s: 60.0, ready: false, assigned_target: None,
            },
        ];

        let regions = vec![
            WallRegionState {
                x_m: 15.0, y_m: 5.0, width_m: 10.0, height_m: 5.0,
                damage_ratio: 0.0, stress_ratio: 0.5, strategic_value: 1.0,
            },
        ];

        let impacts = vec![];
        let result = optimizer.optimize(&trebuchets, &regions, &impacts).await;

        assert_eq!(result.coordination.assignments.len(), 0);
    }

    #[tokio::test]
    async fn test_optimizer_training_progress_default() {
        let wall = default_wall();
        let optimizer = SalvoOptimizer::new(OptimizerConfig::default(), wall);
        let progress = optimizer.get_progress().await;

        assert_eq!(progress.status, TrainingStatus::Idle);
        assert_eq!(progress.current_episode, 0);
        assert_eq!(progress.total_episodes, 0);
        assert_eq!(progress.q_table_size, 0);
        assert_eq!(progress.episodes_trained, 0);
    }

    #[tokio::test]
    async fn test_optimizer_fast_dynamics_config() {
        let mut config = OptimizerConfig::default();
        config.use_fast_dynamics = true;

        let wall = default_wall();
        let optimizer = SalvoOptimizer::new(config, wall);

        let trebuchets = vec![
            TrebuchetState {
                id: 1, ammo_type: AmmoType::RoundStone, range_m: 200.0,
                reload_time_s: 60.0, ready: true, assigned_target: None,
            },
        ];

        let regions = vec![
            WallRegionState {
                x_m: 15.0, y_m: 5.0, width_m: 10.0, height_m: 5.0,
                damage_ratio: 0.0, stress_ratio: 0.5, strategic_value: 1.0,
            },
        ];

        let impacts = vec![];
        let result = optimizer.optimize(&trebuchets, &regions, &impacts).await;

        assert!(result.recommendations.iter().any(|r| r.contains("快速动力学")));
    }

    #[tokio::test]
    async fn test_optimizer_recommendations_count() {
        let wall = default_wall();
        let optimizer = SalvoOptimizer::new(OptimizerConfig::default(), wall);

        let trebuchets = vec![
            TrebuchetState {
                id: 1, ammo_type: AmmoType::RoundStone, range_m: 200.0,
                reload_time_s: 60.0, ready: true, assigned_target: None,
            },
        ];

        let regions = vec![
            WallRegionState {
                x_m: 15.0, y_m: 5.0, width_m: 10.0, height_m: 5.0,
                damage_ratio: 0.0, stress_ratio: 0.5, strategic_value: 1.0,
            },
        ];

        let impacts = vec![];
        let result = optimizer.optimize(&trebuchets, &regions, &impacts).await;

        assert!(result.recommendations.len() >= 3);
    }

    #[tokio::test]
    async fn test_optimizer_coordination_result_fields() {
        let wall = default_wall();
        let optimizer = SalvoOptimizer::new(OptimizerConfig::default(), wall);

        let trebuchets = vec![
            TrebuchetState {
                id: 1, ammo_type: AmmoType::RoundStone, range_m: 200.0,
                reload_time_s: 60.0, ready: true, assigned_target: None,
            },
            TrebuchetState {
                id: 2, ammo_type: AmmoType::GunpowderBomb, range_m: 200.0,
                reload_time_s: 60.0, ready: true, assigned_target: None,
            },
        ];

        let regions = vec![
            WallRegionState {
                x_m: 15.0, y_m: 5.0, width_m: 10.0, height_m: 5.0,
                damage_ratio: 0.0, stress_ratio: 0.5, strategic_value: 1.0,
            },
        ];

        let impacts = vec![];
        let result = optimizer.optimize(&trebuchets, &regions, &impacts).await;

        assert_eq!(result.coordination.assignments.len(), 2);
        assert!(result.coordination.expected_total_damage > 0.0);
        assert!(result.coordination.coordination_efficiency >= 0.0);
        assert!(result.training_progress.episodes_trained >= 0);
    }

    #[tokio::test]
    async fn test_optimizer_target_zones_within_bounds() {
        let wall = default_wall();
        let optimizer = SalvoOptimizer::new(OptimizerConfig::default(), wall);

        let trebuchets = vec![
            TrebuchetState {
                id: 1, ammo_type: AmmoType::RoundStone, range_m: 200.0,
                reload_time_s: 60.0, ready: true, assigned_target: None,
            },
        ];

        let regions = vec![
            WallRegionState {
                x_m: 15.0, y_m: 5.0, width_m: 10.0, height_m: 5.0,
                damage_ratio: 0.0, stress_ratio: 0.5, strategic_value: 1.0,
            },
        ];

        let impacts = vec![];
        let result = optimizer.optimize(&trebuchets, &regions, &impacts).await;

        for assignment in &result.coordination.assignments {
            assert!(assignment.target_x_m >= 0.0);
            assert!(assignment.target_x_m <= 30.0);
            assert!(assignment.target_y_m >= 0.0);
            assert!(assignment.target_y_m <= 10.0);
        }
    }

    #[tokio::test]
    async fn test_optimizer_ammo_type_preserved() {
        let wall = default_wall();
        let optimizer = SalvoOptimizer::new(OptimizerConfig::default(), wall);

        let trebuchets = vec![
            TrebuchetState {
                id: 1, ammo_type: AmmoType::GunpowderBomb, range_m: 200.0,
                reload_time_s: 60.0, ready: true, assigned_target: None,
            },
        ];

        let regions = vec![
            WallRegionState {
                x_m: 15.0, y_m: 5.0, width_m: 10.0, height_m: 5.0,
                damage_ratio: 0.0, stress_ratio: 0.5, strategic_value: 1.0,
            },
        ];

        let impacts = vec![];
        let result = optimizer.optimize(&trebuchets, &regions, &impacts).await;

        assert_eq!(result.coordination.assignments[0].ammo_type, AmmoType::GunpowderBomb);
    }

    #[tokio::test]
    async fn test_create_salvo_optimizer() {
        let wall = default_wall();
        let optimizer = create_salvo_optimizer(&wall, false).await;
        let progress = optimizer.get_progress().await;

        assert_eq!(progress.status, TrainingStatus::Idle);
    }

    #[tokio::test]
    async fn test_optimizer_training_status_variants() {
        assert_eq!(TrainingStatus::Idle as i32, 0);
        assert_eq!(TrainingStatus::Training as i32, 1);
        assert_eq!(TrainingStatus::Completed as i32, 2);
        assert_eq!(TrainingStatus::Error as i32, 3);
    }

    #[tokio::test]
    async fn test_optimizer_priority_ordering() {
        let wall = default_wall();
        let optimizer = SalvoOptimizer::new(OptimizerConfig::default(), wall);

        let trebuchets = vec![
            TrebuchetState {
                id: 1, ammo_type: AmmoType::RoundStone, range_m: 200.0,
                reload_time_s: 60.0, ready: true, assigned_target: None,
            },
            TrebuchetState {
                id: 2, ammo_type: AmmoType::GunpowderBomb, range_m: 200.0,
                reload_time_s: 60.0, ready: true, assigned_target: None,
            },
        ];

        let regions = vec![
            WallRegionState {
                x_m: 15.0, y_m: 5.0, width_m: 10.0, height_m: 5.0,
                damage_ratio: 0.0, stress_ratio: 0.5, strategic_value: 1.0,
            },
        ];

        let impacts = vec![];
        let result = optimizer.optimize(&trebuchets, &regions, &impacts).await;

        let priorities: Vec<f64> = result.coordination.assignments
            .iter()
            .map(|a| a.priority)
            .collect();

        for i in 1..priorities.len() {
            assert!(priorities[i - 1] >= priorities[i], "Priorities should be descending");
        }
    }
}
