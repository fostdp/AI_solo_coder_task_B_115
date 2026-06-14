use serde::{Deserialize, Serialize};
use crate::ammo::AmmoType;
use crate::fea::{FEAnalyzer, FEAResult, ImpactLoad};
use crate::genetic::{GeneticOptimizer, GeneticConfig, GAResult};
use crate::siege::WallProperties;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WeakPointConfig {
    pub wall_width_m: f64,
    pub wall_height_m: f64,
    pub wall_thickness_m: f64,
    pub wall_density_kgm3: f64,
    pub compressive_strength_pa: f64,
    pub tensile_strength_pa: f64,
    pub mesh_nx: usize,
    pub mesh_ny: usize,
    pub population_size: usize,
    pub generations: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub projectile_mass_kg: f64,
    pub impact_energy_j: f64,
    pub ammo_type: AmmoType,
}

impl Default for WeakPointConfig {
    fn default() -> Self {
        Self {
            wall_width_m: 30.0,
            wall_height_m: 10.0,
            wall_thickness_m: 3.0,
            wall_density_kgm3: 1800.0,
            compressive_strength_pa: 2_000_000.0,
            tensile_strength_pa: 200_000.0,
            mesh_nx: 20,
            mesh_ny: 15,
            population_size: 60,
            generations: 50,
            mutation_rate: 0.1,
            crossover_rate: 0.8,
            projectile_mass_kg: 90.0,
            impact_energy_j: 112_500.0,
            ammo_type: AmmoType::RoundStone,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeakPointResult {
    pub best: WeakPoint,
    pub fea: FEAResult,
    pub genetic: GAResult,
    pub analysis_summary: AnalysisSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeakPoint {
    pub x_m: f64,
    pub y_m: f64,
    pub stress_pa: f64,
    pub damage_ratio: f64,
    pub safety_factor: f64,
    pub priority: f64,
    pub is_mortar_joint: bool,
    pub material_type: String,
    pub horizontal_strength_ratio: f64,
    pub vertical_strength_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    pub total_generations: usize,
    pub convergence_improvement: f64,
    pub max_stress_pa: f64,
    pub min_safety_factor: f64,
    pub weak_points_count: usize,
    pub wall_width_m: f64,
    pub wall_height_m: f64,
    pub is_masonry: bool,
    pub brick_height_m: f64,
    pub mortar_joint_height_m: f64,
    pub avg_horizontal_strength_ratio: f64,
    pub avg_vertical_strength_ratio: f64,
    pub recommendations: Vec<String>,
}

pub struct WallWeaknessFinder {
    config: WeakPointConfig,
}

impl WallWeaknessFinder {
    pub fn new(config: WeakPointConfig) -> Self {
        Self { config }
    }

    pub fn from_wall(wall: &WallProperties) -> Self {
        let mut config = WeakPointConfig {
            wall_thickness_m: wall.thickness_m,
            wall_density_kgm3: wall.density_kgm3,
            compressive_strength_pa: wall.compressive_strength_pa,
            tensile_strength_pa: wall.tensile_strength_pa,
            ..Default::default()
        };

        match wall.material.as_str() {
            "stone_masonry" => {
                config.compression_stone_masonry();
            }
            "brick_masonry" => {
                config.compression_brick_masonry();
            }
            _ => {}
        }

        Self::new(config)
    }

    pub fn find(&self) -> WeakPointResult {
        let fea_result = self.analyze_stress();
        let ga_result = self.optimize_weak_point();

        let best_point = self.build_weak_point(&ga_result, &fea_result);
        let summary = self.build_summary(&fea_result, &ga_result);

        WeakPointResult {
            best: best_point,
            fea: fea_result,
            genetic: ga_result,
            analysis_summary: summary,
        }
    }

    pub fn analyze_stress(&self) -> FEAResult {
        let analyzer = self.build_analyzer();
        analyzer.analyze(&[])
    }

    pub fn analyze_stress_with_impacts(&self, impacts: &[ImpactLoad]) -> FEAResult {
        let analyzer = self.build_analyzer();
        analyzer.analyze(impacts)
    }

    pub fn optimize_weak_point(&self) -> GAResult {
        let wall = WallProperties {
            thickness_m: self.config.wall_thickness_m,
            material: self.infer_material(),
            density_kgm3: self.config.wall_density_kgm3,
            compressive_strength_pa: self.config.compressive_strength_pa,
            tensile_strength_pa: self.config.tensile_strength_pa,
        };

        let ga_config = GeneticConfig {
            population_size: self.config.population_size,
            generations: self.config.generations,
            mutation_rate: self.config.mutation_rate,
            crossover_rate: self.config.crossover_rate,
            ..Default::default()
        };

        let optimizer = GeneticOptimizer::new(
            ga_config,
            wall,
            vec![],
            self.config.ammo_type,
            self.config.projectile_mass_kg,
            self.config.impact_energy_j,
        );

        optimizer.optimize()
    }

    fn build_analyzer(&self) -> FEAnalyzer {
        let mut analyzer = FEAnalyzer::new(
            self.config.wall_width_m,
            self.config.wall_height_m,
            self.config.wall_thickness_m,
            self.config.wall_density_kgm3,
            self.config.compressive_strength_pa,
            self.config.tensile_strength_pa,
        );

        analyzer.nx = self.config.mesh_nx;
        analyzer.ny = self.config.mesh_ny;

        analyzer
    }

    fn infer_material(&self) -> String {
        if self.config.compressive_strength_pa > 20_000_000.0 {
            "stone_masonry".to_string()
        } else if self.config.compressive_strength_pa > 5_000_000.0 {
            "brick_masonry".to_string()
        } else {
            "rammed_earth".to_string()
        }
    }

    fn build_weak_point(&self, ga: &GAResult, fea: &FEAResult) -> WeakPoint {
        let best_ga = &ga.best;
        let wall = WallProperties {
            thickness_m: self.config.wall_thickness_m,
            material: self.infer_material(),
            density_kgm3: self.config.wall_density_kgm3,
            compressive_strength_pa: self.config.compressive_strength_pa,
            tensile_strength_pa: self.config.tensile_strength_pa,
        };
        let test_analyzer = FEAnalyzer::from_wall_props(&wall);

        let local_strength = test_analyzer.local_compressive_strength(best_ga.y_m, fea.max_stress_pa);
        let safety_factor = local_strength / fea.max_stress_pa.max(1.0);
        let damage = if fea.max_stress_pa > local_strength {
            1.0 - local_strength / fea.max_stress_pa
        } else {
            (fea.max_stress_pa / local_strength).powi(3) * 0.1
        };

        WeakPoint {
            x_m: best_ga.x_m,
            y_m: best_ga.y_m,
            stress_pa: fea.max_stress_pa,
            damage_ratio: damage.min(1.0),
            safety_factor,
            priority: if safety_factor < 1.0 { 1.0 } else { 1.0 / safety_factor },
            is_mortar_joint: test_analyzer.is_mortar_joint(best_ga.y_m),
            material_type: if test_analyzer.is_masonry {
                if test_analyzer.is_mortar_joint(best_ga.y_m) { "mortar_joint".to_string() } else { "masonry_brick".to_string() }
            } else {
                "homogeneous".to_string()
            },
            horizontal_strength_ratio: test_analyzer.horizontal_strength_ratio,
            vertical_strength_ratio: test_analyzer.vertical_strength_ratio,
        }
    }

    fn build_summary(&self, fea: &FEAResult, ga: &GAResult) -> AnalysisSummary {
        let wall = WallProperties {
            thickness_m: self.config.wall_thickness_m,
            material: self.infer_material(),
            density_kgm3: self.config.wall_density_kgm3,
            compressive_strength_pa: self.config.compressive_strength_pa,
            tensile_strength_pa: self.config.tensile_strength_pa,
        };
        let test_analyzer = FEAnalyzer::from_wall_props(&wall);

        let mut recommendations = Vec::new();

        if fea.min_safety_factor < 1.0 {
            recommendations.push("⚠️ 存在安全系数小于1的危险区域，建议立即加固".to_string());
        } else if fea.min_safety_factor < 1.5 {
            recommendations.push("⚠️ 部分区域安全系数较低，建议重点监测".to_string());
        } else {
            recommendations.push("✅ 城墙整体安全状况良好".to_string());
        }

        if test_analyzer.is_masonry {
            recommendations.push(format!(
                "砖石砌体结构: 竖向强度仅为水平向的 {:.0}%，灰缝为薄弱层",
                test_analyzer.vertical_strength_ratio * 100.0
            ));

            let avg_damage_at_joints = fea.mesh.elements.iter()
                .filter(|e| e.material_type == "mortar_joint")
                .map(|e| e.damage)
                .sum::<f64>() / fea.mesh.elements.iter().filter(|e| e.material_type == "mortar_joint").count().max(1) as f64;

            if avg_damage_at_joints > 0.3 {
                recommendations.push(format!(
                    "⚠️ 灰缝层平均损伤 {:.1}%，建议进行灰缝修复",
                    avg_damage_at_joints * 100.0
                ));
            }
        }

        if let Some(first) = ga.convergence_data.first() {
            if let Some(last) = ga.convergence_data.last() {
                let improvement = if *first > 0.0 { (last - first) / first * 100.0 } else { 0.0 };
                recommendations.push(format!(
                    "遗传算法优化: 适应度提升 {:.1}%，建议打击点 ({:.1}, {:.1}) m",
                    improvement, ga.best.x_m, ga.best.y_m
                ));
            }
        }

        if fea.max_stress_pa > self.config.compressive_strength_pa {
            recommendations.push("💥 最大应力超过抗压强度，可能发生结构破坏".to_string());
        }

        AnalysisSummary {
            total_generations: ga.total_generations,
            convergence_improvement: {
                if !ga.convergence_data.is_empty() {
                    let first = ga.convergence_data[0];
                    let last = ga.convergence_data[ga.convergence_data.len() - 1];
                    if first > 0.0 { (last - first) / first } else { 0.0 }
                } else {
                    0.0
                }
            },
            max_stress_pa: fea.max_stress_pa,
            min_safety_factor: fea.min_safety_factor,
            weak_points_count: fea.weak_points.len(),
            wall_width_m: self.config.wall_width_m,
            wall_height_m: self.config.wall_height_m,
            is_masonry: test_analyzer.is_masonry,
            brick_height_m: test_analyzer.brick_height_m,
            mortar_joint_height_m: test_analyzer.mortar_joint_height_m,
            avg_horizontal_strength_ratio: test_analyzer.horizontal_strength_ratio,
            avg_vertical_strength_ratio: test_analyzer.vertical_strength_ratio,
            recommendations,
        }
    }
}

impl WeakPointConfig {
    fn compression_stone_masonry(&mut self) {
        self.wall_density_kgm3 = 2400.0;
    }

    fn compression_brick_masonry(&mut self) {
        self.wall_density_kgm3 = 2000.0;
    }
}

pub fn find_weak_point_simple(
    wall: &WallProperties,
    ammo_type: AmmoType,
    projectile_mass_kg: f64,
    impact_energy_j: f64,
) -> WeakPointResult {
    let finder = WallWeaknessFinder::from_wall(wall);
    let mut config = finder.config;
    config.ammo_type = ammo_type;
    config.projectile_mass_kg = projectile_mass_kg;
    config.impact_energy_j = impact_energy_j;
    WallWeaknessFinder::new(config).find()
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

    #[test]
    fn test_finder_default_config() {
        let finder = WallWeaknessFinder::new(WeakPointConfig::default());
        let result = finder.find();

        assert_eq!(result.genetic.total_generations, 50);
        assert!(result.best.x_m > 0.0);
        assert!(result.best.x_m < 30.0);
        assert!(result.best.y_m > 0.0);
        assert!(result.best.y_m < 10.0);
    }

    #[test]
    fn test_finder_from_wall() {
        let wall = default_wall();
        let finder = WallWeaknessFinder::from_wall(&wall);

        assert_eq!(finder.config.wall_thickness_m, 3.0);
        assert_eq!(finder.config.wall_density_kgm3, 1800.0);
        assert_eq!(finder.config.compressive_strength_pa, 2_000_000.0);
    }

    #[test]
    fn test_finder_stress_analysis() {
        let finder = WallWeaknessFinder::new(WeakPointConfig::default());
        let fea = finder.analyze_stress();

        assert_eq!(fea.mesh.nx, 20);
        assert_eq!(fea.mesh.ny, 15);
        assert!(fea.max_stress_pa > 0.0);
        assert!(fea.min_safety_factor > 0.0);
    }

    #[test]
    fn test_finder_weak_point_bounds() {
        let finder = WallWeaknessFinder::new(WeakPointConfig::default());
        let result = finder.find();

        assert!(result.best.x_m >= 0.5);
        assert!(result.best.x_m <= 29.5);
        assert!(result.best.y_m >= 0.5);
        assert!(result.best.y_m <= 9.5);
    }

    #[test]
    fn test_finder_safety_factor() {
        let finder = WallWeaknessFinder::new(WeakPointConfig::default());
        let result = finder.find();

        assert!(result.best.safety_factor > 0.0);
        assert!(result.best.priority > 0.0);
    }

    #[test]
    fn test_finder_summary_recommendations() {
        let finder = WallWeaknessFinder::new(WeakPointConfig::default());
        let result = finder.find();

        assert!(result.analysis_summary.recommendations.len() >= 2);
    }

    #[test]
    fn test_finder_summary_convergence() {
        let finder = WallWeaknessFinder::new(WeakPointConfig::default());
        let result = finder.find();

        assert_eq!(result.analysis_summary.total_generations, 50);
        assert!(result.analysis_summary.convergence_improvement >= 0.0);
    }

    #[test]
    fn test_finder_masonry_detection() {
        let wall = WallProperties {
            thickness_m: 4.0,
            material: "stone_masonry".to_string(),
            density_kgm3: 2400.0,
            compressive_strength_pa: 25_000_000.0,
            tensile_strength_pa: 2_000_000.0,
        };
        let finder = WallWeaknessFinder::from_wall(&wall);
        let result = finder.find();

        assert_eq!(result.analysis_summary.is_masonry, true);
        assert!(result.analysis_summary.avg_vertical_strength_ratio < 1.0);
    }

    #[test]
    fn test_finder_stress_with_impacts() {
        let finder = WallWeaknessFinder::new(WeakPointConfig::default());
        let impacts = vec![
            ImpactLoad { x_m: 15.0, y_m: 5.0, impact_force_n: 5_000_000.0, blast_radius_m: 0.0 },
        ];
        let fea = finder.analyze_stress_with_impacts(&impacts);
        let fea_no_impact = finder.analyze_stress();

        assert!(fea.max_stress_pa > fea_no_impact.max_stress_pa);
    }

    #[test]
    fn test_finder_different_ammo() {
        let wall = default_wall();

        let result_stone = find_weak_point_simple(&wall, AmmoType::RoundStone, 90.0, 100_000.0);
        let result_gunpowder = find_weak_point_simple(&wall, AmmoType::GunpowderBomb, 90.0, 100_000.0);
        let result_corpse = find_weak_point_simple(&wall, AmmoType::CorpseShell, 90.0, 100_000.0);

        assert_eq!(result_stone.best.material_type, "homogeneous");
        assert_eq!(result_gunpowder.best.material_type, "homogeneous");
        assert_eq!(result_corpse.best.material_type, "homogeneous");
    }

    #[test]
    fn test_finder_boundary_zero_energy() {
        let wall = default_wall();
        let result = find_weak_point_simple(&wall, AmmoType::RoundStone, 90.0, 0.0);

        assert!(result.best.x_m > 0.0);
        assert!(result.best.x_m < 30.0);
    }

    #[test]
    fn test_finder_boundary_high_energy() {
        let wall = default_wall();
        let result = find_weak_point_simple(&wall, AmmoType::RoundStone, 90.0, 10_000_000.0);

        assert!(result.analysis_summary.max_stress_pa > 0.0);
    }

    #[test]
    fn test_finder_boundary_small_mesh() {
        let mut config = WeakPointConfig::default();
        config.mesh_nx = 5;
        config.mesh_ny = 5;
        let finder = WallWeaknessFinder::new(config);
        let result = finder.find();

        assert_eq!(result.fea.mesh.nx, 5);
        assert_eq!(result.fea.mesh.ny, 5);
    }

    #[test]
    fn test_finder_boundary_small_population() {
        let mut config = WeakPointConfig::default();
        config.population_size = 10;
        config.generations = 5;
        let finder = WallWeaknessFinder::new(config);
        let result = finder.find();

        assert_eq!(result.genetic.total_generations, 5);
    }

    #[test]
    fn test_finder_weak_point_damage_bounded() {
        let finder = WallWeaknessFinder::new(WeakPointConfig::default());
        let result = finder.find();

        assert!(result.best.damage_ratio >= 0.0);
        assert!(result.best.damage_ratio <= 1.0);
    }

    #[test]
    fn test_finder_summary_max_stress() {
        let finder = WallWeaknessFinder::new(WeakPointConfig::default());
        let result = finder.find();

        assert_eq!(result.analysis_summary.max_stress_pa, result.fea.max_stress_pa);
    }

    #[test]
    fn test_finder_summary_weak_points_count() {
        let finder = WallWeaknessFinder::new(WeakPointConfig::default());
        let result = finder.find();

        assert_eq!(
            result.analysis_summary.weak_points_count,
            result.fea.weak_points.len()
        );
    }

    #[test]
    fn test_finder_summary_wall_dimensions() {
        let finder = WallWeaknessFinder::new(WeakPointConfig::default());
        let result = finder.find();

        assert_eq!(result.analysis_summary.wall_width_m, 30.0);
        assert_eq!(result.analysis_summary.wall_height_m, 10.0);
    }

    #[test]
    fn test_finder_convergence_improvement_non_negative() {
        let finder = WallWeaknessFinder::new(WeakPointConfig::default());
        let result = finder.find();

        assert!(result.genetic.convergence_data.len() >= 2);
        let last = result.genetic.convergence_data.last().unwrap();
        let first = result.genetic.convergence_data.first().unwrap();
        assert!(last >= first, "Last gen fitness should be >= first gen");
    }
}
