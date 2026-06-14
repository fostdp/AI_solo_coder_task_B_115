use serde::{Deserialize, Serialize};
use crate::ammo::{AmmoType, AmmoProfile};
use crate::ballistics::{simulate_ballistics_with_ammo, BallisticResult};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CompareConfig {
    pub initial_velocity_mps: f64,
    pub launch_angle_deg: f64,
    pub projectile_mass_kg: f64,
    pub air_density_kgm3: f64,
    pub launch_height_m: f64,
    pub wind_speed_mps: f64,
    pub wind_direction_deg: f64,
}

impl Default for CompareConfig {
    fn default() -> Self {
        Self {
            initial_velocity_mps: 50.0,
            launch_angle_deg: 45.0,
            projectile_mass_kg: 90.0,
            air_density_kgm3: 1.225,
            launch_height_m: 5.0,
            wind_speed_mps: 0.0,
            wind_direction_deg: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonDetail {
    pub ammo_type: AmmoType,
    pub estimated_range_m: f64,
    pub estimated_max_height_m: f64,
    pub estimated_flight_time_s: f64,
    pub estimated_impact_velocity_mps: f64,
    pub estimated_impact_energy_j: f64,
    pub explosive_energy_j: f64,
    pub total_damage_potential: f64,
    pub blast_radius_m: f64,
    pub contamination_radius_m: f64,
    pub base_drag_coefficient: f64,
    pub shape_factor: f64,
    pub density_kgm3: f64,
    pub effective_diameter_m: f64,
    pub ballistic_coefficient: f64,
    pub drag_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmoComparatorResult {
    pub config: CompareConfig,
    pub round_stone: ComparisonDetail,
    pub gunpowder_bomb: ComparisonDetail,
    pub corpse_shell: ComparisonDetail,
    pub summary: ComparisonSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonSummary {
    pub best_range: AmmoType,
    pub best_damage: AmmoType,
    pub best_blast: AmmoType,
    pub best_contamination: AmmoType,
    pub range_ranking: Vec<AmmoType>,
    pub damage_ranking: Vec<AmmoType>,
    pub notes: Vec<String>,
}

pub struct AmmoComparator {
    config: CompareConfig,
}

impl AmmoComparator {
    pub fn new(config: CompareConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(CompareConfig::default())
    }

    pub fn set_velocity(&mut self, v: f64) {
        self.config.initial_velocity_mps = v;
    }

    pub fn set_angle(&mut self, ang: f64) {
        self.config.launch_angle_deg = ang;
    }

    pub fn compare(&self) -> AmmoComparatorResult {
        let ammo_types = AmmoType::all();
        let mut details: Vec<(AmmoType, ComparisonDetail)> = Vec::new();

        for ammo_type in &ammo_types {
            let profile = AmmoProfile::from_type(*ammo_type, self.config.projectile_mass_kg);
            let sim_result = self.run_ballistic_simulation(*ammo_type);

            let ballistic_coeff = self.calculate_ballistic_coefficient(*ammo_type, &profile);

            let total_damage = sim_result.impact_kinetic_energy_j
                + profile.explosive_yield_j * 0.3
                + profile.contamination_radius_m * 1000.0;

            let detail = ComparisonDetail {
                ammo_type: *ammo_type,
                estimated_range_m: sim_result.range_m,
                estimated_max_height_m: sim_result.max_height_m,
                estimated_flight_time_s: sim_result.flight_time_s,
                estimated_impact_velocity_mps: sim_result.impact_velocity_mps,
                estimated_impact_energy_j: sim_result.impact_kinetic_energy_j,
                explosive_energy_j: profile.explosive_yield_j,
                total_damage_potential: total_damage,
                blast_radius_m: profile.blast_radius_m,
                contamination_radius_m: profile.contamination_radius_m,
                base_drag_coefficient: ammo_type.base_drag_coefficient(),
                shape_factor: ammo_type.shape_factor(),
                density_kgm3: ammo_type.density_kgm3(),
                effective_diameter_m: profile.effective_diameter(self.config.projectile_mass_kg),
                ballistic_coefficient: ballistic_coeff,
                drag_notes: ammo_type.drag_estimation_notes().to_string(),
            };
            details.push((*ammo_type, detail));
        }

        let round_stone = details.iter().find(|(t, _)| *t == AmmoType::RoundStone).unwrap().1.clone();
        let gunpowder_bomb = details.iter().find(|(t, _)| *t == AmmoType::GunpowderBomb).unwrap().1.clone();
        let corpse_shell = details.iter().find(|(t, _)| *t == AmmoType::CorpseShell).unwrap().1.clone();

        let summary = self.build_summary(&details);

        AmmoComparatorResult {
            config: self.config,
            round_stone,
            gunpowder_bomb,
            corpse_shell,
            summary,
        }
    }

    fn run_ballistic_simulation(&self, ammo_type: AmmoType) -> BallisticResult {
        let profile = AmmoProfile::from_type(ammo_type, self.config.projectile_mass_kg);
        simulate_ballistics_with_ammo(
            self.config.initial_velocity_mps,
            self.config.launch_angle_deg,
            self.config.projectile_mass_kg,
            self.config.air_density_kgm3,
            self.config.wind_speed_mps,
            self.config.wind_direction_deg,
            self.config.launch_height_m,
            &profile,
        )
    }

    fn calculate_ballistic_coefficient(&self, ammo_type: AmmoType, profile: &AmmoProfile) -> f64 {
        let mass = self.config.projectile_mass_kg;
        let diameter = profile.effective_diameter(self.config.projectile_mass_kg);
        let cd = ammo_type.base_drag_coefficient();
        let i = ammo_type.shape_factor();

        mass / (cd * i * diameter * diameter)
    }

    fn build_summary(&self, details: &[(AmmoType, ComparisonDetail)]) -> ComparisonSummary {
        let mut range_sorted: Vec<_> = details.to_vec();
        range_sorted.sort_by(|a, b| b.1.estimated_range_m.partial_cmp(&a.1.estimated_range_m).unwrap());
        let range_ranking: Vec<AmmoType> = range_sorted.iter().map(|(t, _)| *t).collect();

        let mut damage_sorted: Vec<_> = details.to_vec();
        damage_sorted.sort_by(|a, b| b.1.total_damage_potential.partial_cmp(&a.1.total_damage_potential).unwrap());
        let damage_ranking: Vec<AmmoType> = damage_sorted.iter().map(|(t, _)| *t).collect();

        let best_blast = details.iter()
            .max_by(|a, b| a.1.blast_radius_m.partial_cmp(&b.1.blast_radius_m).unwrap())
            .map(|(t, _)| *t)
            .unwrap_or(AmmoType::RoundStone);

        let best_contamination = details.iter()
            .max_by(|a, b| a.1.contamination_radius_m.partial_cmp(&b.1.contamination_radius_m).unwrap())
            .map(|(t, _)| *t)
            .unwrap_or(AmmoType::RoundStone);

        let mut notes = Vec::new();
        notes.push(format!(
            "射程排序: {} > {} > {}",
            self.ammo_name(range_ranking[0]),
            self.ammo_name(range_ranking[1]),
            self.ammo_name(range_ranking[2])
        ));
        notes.push(format!(
            "综合破坏排序: {} > {} > {}",
            self.ammo_name(damage_ranking[0]),
            self.ammo_name(damage_ranking[1]),
            self.ammo_name(damage_ranking[2])
        ));
        notes.push("圆石弹: 高射程低成本，无附加效果".to_string());
        notes.push("火药弹: 爆炸效果显著，适合破坏城墙结构".to_string());
        notes.push("腐尸弹: 低射程但有污染效果，适合心理战".to_string());

        ComparisonSummary {
            best_range: range_ranking[0],
            best_damage: damage_ranking[0],
            best_blast,
            best_contamination,
            range_ranking,
            damage_ranking,
            notes,
        }
    }

    fn ammo_name(&self, t: AmmoType) -> &str {
        match t {
            AmmoType::RoundStone => "圆石弹",
            AmmoType::GunpowderBomb => "火药弹",
            AmmoType::CorpseShell => "腐尸弹",
        }
    }
}

pub fn compare_ammo_with_config(config: CompareConfig) -> AmmoComparatorResult {
    AmmoComparator::new(config).compare()
}

pub fn compare_ammo_simple(velocity: f64, angle: f64, mass_kg: f64, air_density: f64) -> AmmoComparatorResult {
    let config = CompareConfig {
        initial_velocity_mps: velocity,
        launch_angle_deg: angle,
        projectile_mass_kg: mass_kg,
        air_density_kgm3: air_density,
        ..Default::default()
    };
    AmmoComparator::new(config).compare()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq_f64(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn test_comparator_default_config() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert_eq!(result.config.initial_velocity_mps, 50.0);
        assert_eq!(result.config.launch_angle_deg, 45.0);
        assert_eq!(result.config.projectile_mass_kg, 90.0);
    }

    #[test]
    fn test_comparator_stone_vs_gunpowder_range() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert!(result.round_stone.estimated_range_m >= result.gunpowder_bomb.estimated_range_m,
            "RoundStone should have greater or equal range than GunpowderBomb");
        assert!(result.gunpowder_bomb.estimated_range_m >= result.corpse_shell.estimated_range_m,
            "GunpowderBomb should have greater or equal range than CorpseShell");
    }

    #[test]
    fn test_comparator_explosive_damage() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert!(result.round_stone.explosive_energy_j == 0.0,
            "RoundStone should have no explosive energy");
        assert!(result.gunpowder_bomb.explosive_energy_j > 0.0,
            "GunpowderBomb should have explosive energy");
        assert!(result.gunpowder_bomb.blast_radius_m > 0.0,
            "GunpowderBomb should have blast radius");
        assert!(result.corpse_shell.contamination_radius_m > 0.0,
            "CorpseShell should have contamination radius");
    }

    #[test]
    fn test_comparator_drag_coefficients() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert!(approx_eq_f64(result.round_stone.base_drag_coefficient, 0.47, 0.001));
        assert!(approx_eq_f64(result.gunpowder_bomb.base_drag_coefficient, 0.72, 0.001));
        assert!(approx_eq_f64(result.corpse_shell.base_drag_coefficient, 1.15, 0.001));
    }

    #[test]
    fn test_comparator_density_ordering() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert!(result.round_stone.density_kgm3 > result.gunpowder_bomb.density_kgm3);
        assert!(result.gunpowder_bomb.density_kgm3 > result.corpse_shell.density_kgm3);
    }

    #[test]
    fn test_comparator_effective_diameter() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert!(result.corpse_shell.effective_diameter_m > result.gunpowder_bomb.effective_diameter_m);
        assert!(result.gunpowder_bomb.effective_diameter_m > result.round_stone.effective_diameter_m);
    }

    #[test]
    fn test_comparator_ballistic_coefficient() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert!(result.round_stone.ballistic_coefficient > result.gunpowder_bomb.ballistic_coefficient);
        assert!(result.gunpowder_bomb.ballistic_coefficient > result.corpse_shell.ballistic_coefficient);
    }

    #[test]
    fn test_comparator_summary_best_range() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert_eq!(result.summary.best_range, AmmoType::RoundStone);
        assert_eq!(result.summary.range_ranking[0], AmmoType::RoundStone);
    }

    #[test]
    fn test_comparator_summary_best_damage() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert_eq!(result.summary.best_damage, AmmoType::GunpowderBomb);
        assert_eq!(result.summary.best_blast, AmmoType::GunpowderBomb);
    }

    #[test]
    fn test_comparator_summary_best_contamination() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert_eq!(result.summary.best_contamination, AmmoType::CorpseShell);
    }

    #[test]
    fn test_comparator_summary_notes() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert_eq!(result.summary.notes.len(), 5);
        assert!(result.summary.notes.iter().any(|n| n.contains("射程排序")));
        assert!(result.summary.notes.iter().any(|n| n.contains("综合破坏排序")));
    }

    #[test]
    fn test_comparator_boundary_zero_velocity() {
        let config = CompareConfig {
            initial_velocity_mps: 0.0,
            launch_angle_deg: 45.0,
            projectile_mass_kg: 90.0,
            ..Default::default()
        };
        let result = compare_ammo_with_config(config);

        assert!(result.round_stone.estimated_range_m < 1.0);
        assert!(result.round_stone.estimated_flight_time_s < 2.0);
    }

    #[test]
    fn test_comparator_boundary_extreme_angle() {
        let config = CompareConfig {
            initial_velocity_mps: 50.0,
            launch_angle_deg: 85.0,
            projectile_mass_kg: 90.0,
            ..Default::default()
        };
        let result = compare_ammo_with_config(config);

        assert!(result.round_stone.estimated_max_height_m > result.round_stone.estimated_range_m);
    }

    #[test]
    fn test_comparator_boundary_extreme_mass() {
        let config = CompareConfig {
            initial_velocity_mps: 50.0,
            launch_angle_deg: 45.0,
            projectile_mass_kg: 50000.0,
            ..Default::default()
        };
        let result = compare_ammo_with_config(config);

        assert!(result.gunpowder_bomb.explosive_energy_j > 1_000_000.0);
        assert!(result.corpse_shell.effective_diameter_m > 1.0);
    }

    #[test]
    fn test_comparator_setters() {
        let mut comparator = AmmoComparator::with_defaults();
        comparator.set_velocity(60.0);
        comparator.set_angle(30.0);

        let result = comparator.compare();
        assert_eq!(result.config.initial_velocity_mps, 60.0);
        assert_eq!(result.config.launch_angle_deg, 30.0);
    }

    #[test]
    fn test_comparator_simple_fn() {
        let result = compare_ammo_simple(50.0, 45.0, 90.0, 1.225);

        assert_eq!(result.config.initial_velocity_mps, 50.0);
        assert_eq!(result.config.launch_angle_deg, 45.0);
        assert!(result.round_stone.estimated_range_m > 100.0);
    }

    #[test]
    fn test_comparator_drag_notes() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert!(result.round_stone.drag_notes.contains("球形标准阻力系数"));
        assert!(result.gunpowder_bomb.drag_notes.contains("柱球形带引信"));
        assert!(result.corpse_shell.drag_notes.contains("不规则生物形状"));
    }

    #[test]
    fn test_comparator_all_types_present() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert_eq!(result.round_stone.ammo_type, AmmoType::RoundStone);
        assert_eq!(result.gunpowder_bomb.ammo_type, AmmoType::GunpowderBomb);
        assert_eq!(result.corpse_shell.ammo_type, AmmoType::CorpseShell);
    }

    #[test]
    fn test_comparator_shape_factor_ordering() {
        let comparator = AmmoComparator::with_defaults();
        let result = comparator.compare();

        assert!(result.round_stone.shape_factor > result.gunpowder_bomb.shape_factor);
        assert!(result.gunpowder_bomb.shape_factor > result.corpse_shell.shape_factor);
    }
}
