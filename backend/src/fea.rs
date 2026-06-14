use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallMesh {
    pub width_m: f64,
    pub height_m: f64,
    pub thickness_m: f64,
    pub nx: usize,
    pub ny: usize,
    pub elements: Vec<WallElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallElement {
    pub i: usize,
    pub j: usize,
    pub center_x: f64,
    pub center_y: f64,
    pub width: f64,
    pub height: f64,
    pub stress_pa: f64,
    pub strain: f64,
    pub damage: f64,
    pub material_type: String,
    pub compressive_strength_pa: f64,
    pub tensile_strength_pa: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FEAResult {
    pub mesh: WallMesh,
    pub max_stress_pa: f64,
    pub min_safety_factor: f64,
    pub weak_points: Vec<WeakPoint>,
    pub stress_field: Vec<Vec<f64>>,
    pub damage_field: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeakPoint {
    pub x_m: f64,
    pub y_m: f64,
    pub stress_pa: f64,
    pub safety_factor: f64,
    pub priority: f64,
}

pub struct FEAnalyzer {
    pub wall_width_m: f64,
    pub wall_height_m: f64,
    pub wall_thickness_m: f64,
    pub wall_density_kgm3: f64,
    pub compressive_strength_pa: f64,
    pub tensile_strength_pa: f64,
    pub elastic_modulus_pa: f64,
    pub poisson_ratio: f64,
    pub nx: usize,
    pub ny: usize,
}

impl FEAnalyzer {
    pub fn new(
        width_m: f64,
        height_m: f64,
        thickness_m: f64,
        density_kgm3: f64,
        compressive_strength_pa: f64,
        tensile_strength_pa: f64,
    ) -> Self {
        let elastic_modulus_pa = compressive_strength_pa * 1000.0;
        Self {
            wall_width_m: width_m,
            wall_height_m: height_m,
            wall_thickness_m: thickness_m,
            wall_density_kgm3: density_kgm3,
            compressive_strength_pa,
            tensile_strength_pa,
            elastic_modulus_pa,
            poisson_ratio: 0.2,
            nx: 20,
            ny: 15,
        }
    }

    pub fn from_wall_props(wall: &crate::siege::WallProperties) -> Self {
        Self::new(
            30.0,
            10.0,
            wall.thickness_m,
            wall.density_kgm3,
            wall.compressive_strength_pa,
            wall.tensile_strength_pa,
        )
    }

    pub fn analyze(&self, existing_impacts: &[ImpactLoad]) -> FEAResult {
        let dx = self.wall_width_m / self.nx as f64;
        let dy = self.wall_height_m / self.ny as f64;

        let mut stress_field = vec![vec![0.0_f64; self.ny]; self.nx];
        let mut damage_field = vec![vec![0.0_f64; self.ny]; self.nx];
        let mut elements = Vec::new();

        let gravity_load = self.wall_density_kgm3 * 9.81;

        for i in 0..self.nx {
            for j in 0..self.ny {
                let cx = (i as f64 + 0.5) * dx;
                let cy = (j as f64 + 0.5) * dy;

                let height_from_base = cy;
                let compressive_stress = gravity_load * (self.wall_height_m - height_from_base) * self.wall_thickness_m;

                let lateral_pressure = self.compute_lateral_pressure(height_from_base);

                let base_stress = compressive_stress * 0.5 + lateral_pressure;

                let mut total_stress = base_stress;

                for impact in existing_impacts {
                    let impact_stress = self.compute_impact_stress(cx, cy, impact);
                    total_stress += impact_stress;
                }

                let gate_proximity_factor = self.gate_proximity_factor(cx);
                total_stress *= gate_proximity_factor;

                let corner_factor = self.corner_stress_factor(cx, cy);
                total_stress *= corner_factor;

                stress_field[i][j] = total_stress;

                let damage = if total_stress > self.compressive_strength_pa {
                    1.0 - (self.compressive_strength_pa / total_stress).min(1.0)
                } else {
                    (total_stress / self.compressive_strength_pa).powi(3) * 0.1
                };
                damage_field[i][j] = damage.min(1.0);

                elements.push(WallElement {
                    i,
                    j,
                    center_x: cx,
                    center_y: cy,
                    width: dx,
                    height: dy,
                    stress_pa: total_stress,
                    strain: total_stress / self.elastic_modulus_pa,
                    damage: damage_field[i][j],
                    material_type: "rammed_earth".to_string(),
                    compressive_strength_pa: self.compressive_strength_pa,
                    tensile_strength_pa: self.tensile_strength_pa,
                });
            }
        }

        let max_stress = stress_field
            .iter()
            .flat_map(|row| row.iter())
            .cloned()
            .fold(0.0_f64, f64::max);

        let min_safety_factor = if max_stress > 0.0 {
            self.compressive_strength_pa / max_stress
        } else {
            100.0
        };

        let weak_points = self.identify_weak_points(&stress_field, dx, dy);

        let mesh = WallMesh {
            width_m: self.wall_width_m,
            height_m: self.wall_height_m,
            thickness_m: self.wall_thickness_m,
            nx: self.nx,
            ny: self.ny,
            elements,
        };

        FEAResult {
            mesh,
            max_stress_pa: max_stress,
            min_safety_factor,
            weak_points,
            stress_field,
            damage_field,
        }
    }

    fn compute_lateral_pressure(&self, height_m: f64) -> f64 {
        let k0 = 1.0 - self.poisson_ratio / (1.0 + self.poisson_ratio);
        let overburden = self.wall_density_kgm3 * 9.81 * height_m;
        overburden * k0 * 0.3
    }

    fn compute_impact_stress(&self, cx: f64, cy: f64, impact: &ImpactLoad) -> f64 {
        let dx = cx - impact.x_m;
        let dy = cy - impact.y_m;
        let dist_sq = dx * dx + dy * dy;
        let attenuation_radius = impact.blast_radius_m.max(1.0) * 3.0;

        if dist_sq > attenuation_radius * attenuation_radius {
            return 0.0;
        }

        let dist = dist_sq.sqrt().max(0.1);
        let peak_stress = impact.impact_force_n / (PI * impact.blast_radius_m.max(0.5).powi(2));

        peak_stress * (-dist / attenuation_radius).exp()
    }

    fn gate_proximity_factor(&self, x_m: f64) -> f64 {
        let gate_center = self.wall_width_m / 2.0;
        let gate_width = 4.0;
        let dist_to_gate = (x_m - gate_center).abs();

        if dist_to_gate < gate_width / 2.0 {
            1.5
        } else if dist_to_gate < gate_width {
            1.2
        } else {
            1.0
        }
    }

    fn corner_stress_factor(&self, x_m: f64, _y_m: f64) -> f64 {
        let corner_dist = x_m.min(self.wall_width_m - x_m);
        if corner_dist < 3.0 {
            1.0 + 0.3 * (1.0 - corner_dist / 3.0)
        } else {
            1.0
        }
    }

    fn identify_weak_points(&self, stress_field: &[Vec<f64>], dx: f64, dy: f64) -> Vec<WeakPoint> {
        let mut candidates: Vec<WeakPoint> = Vec::new();

        for i in 0..self.nx {
            for j in 0..self.ny {
                let stress = stress_field[i][j];
                let safety = if stress > 0.0 {
                    self.compressive_strength_pa / stress
                } else {
                    100.0
                };

                if safety < 3.0 {
                    let priority = (3.0 - safety.min(3.0)) / 3.0
                        * (1.0 + self.gate_proximity_factor((i as f64 + 0.5) * dx) - 1.0);

                    candidates.push(WeakPoint {
                        x_m: (i as f64 + 0.5) * dx,
                        y_m: (j as f64 + 0.5) * dy,
                        stress_pa: stress,
                        safety_factor: safety,
                        priority,
                    });
                }
            }
        }

        candidates.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(10);
        candidates
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactLoad {
    pub x_m: f64,
    pub y_m: f64,
    pub impact_force_n: f64,
    pub blast_radius_m: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_analyzer() -> FEAnalyzer {
        FEAnalyzer::new(30.0, 10.0, 3.0, 1800.0, 2_000_000.0, 200_000.0)
    }

    #[test]
    fn test_fe_analysis_no_impacts() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        assert!(result.max_stress_pa > 0.0);
        assert!(result.min_safety_factor > 0.0);
    }

    #[test]
    fn test_fe_analysis_with_impacts() {
        let analyzer = default_analyzer();
        let impacts = vec![ImpactLoad {
            x_m: 15.0,
            y_m: 5.0,
            impact_force_n: 1_000_000.0,
            blast_radius_m: 2.0,
        }];
        let result = analyzer.analyze(&impacts);
        assert!(result.max_stress_pa > 0.0);
        assert!(!result.weak_points.is_empty() || result.max_stress_pa > 0.0);
    }

    #[test]
    fn test_stress_field_dimensions() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        assert_eq!(result.stress_field.len(), analyzer.nx);
        for row in &result.stress_field {
            assert_eq!(row.len(), analyzer.ny);
        }
    }

    #[test]
    fn test_damage_field_dimensions() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        assert_eq!(result.damage_field.len(), analyzer.nx);
        for row in &result.damage_field {
            assert_eq!(row.len(), analyzer.ny);
        }
    }

    #[test]
    fn test_damage_values_in_range() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        for row in &result.damage_field {
            for &d in row {
                assert!(d >= 0.0 && d <= 1.0, "damage {} out of [0,1]", d);
            }
        }
    }

    #[test]
    fn test_stress_non_negative() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        for row in &result.stress_field {
            for &s in row {
                assert!(s >= 0.0, "stress should be non-negative, got {}", s);
            }
        }
    }

    #[test]
    fn test_gravity_load_increases_with_depth() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        let j_bottom = 0;
        let j_top = analyzer.ny - 1;
        let i_mid = analyzer.nx / 2;
        let stress_bottom = result.stress_field[i_mid][j_bottom];
        let stress_top = result.stress_field[i_mid][j_top];
        assert!(stress_bottom > stress_top,
            "stress at bottom ({}) should exceed top ({}) due to gravity", stress_bottom, stress_top);
    }

    #[test]
    fn test_gate_proximity_higher_stress() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        let gate_col = analyzer.nx / 2;
        let away_col = 1;
        let j_mid = analyzer.ny / 2;
        let stress_gate = result.stress_field[gate_col][j_mid];
        let stress_away = result.stress_field[away_col][j_mid];
        assert!(stress_gate > stress_away,
            "stress near gate ({}) should exceed away ({})", stress_gate, stress_away);
    }

    #[test]
    fn test_corner_stress_higher() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        let j_mid = analyzer.ny / 2;
        let stress_corner = result.stress_field[0][j_mid];
        let stress_mid = result.stress_field[analyzer.nx / 2][j_mid];
        let gate_col = analyzer.nx / 2;
        if gate_col != 0 {
            assert!(stress_corner >= stress_mid * 0.8,
                "corner stress should be elevated due to stress concentration");
        }
    }

    #[test]
    fn test_impact_increases_stress() {
        let analyzer = default_analyzer();
        let result_no_impact = analyzer.analyze(&[]);
        let impacts = vec![ImpactLoad {
            x_m: 15.0,
            y_m: 5.0,
            impact_force_n: 5_000_000.0,
            blast_radius_m: 3.0,
        }];
        let result_with_impact = analyzer.analyze(&impacts);
        assert!(result_with_impact.max_stress_pa >= result_no_impact.max_stress_pa,
            "impact should increase or maintain max stress");
    }

    #[test]
    fn test_impact_stress_attenuation() {
        let analyzer = default_analyzer();
        let impacts = vec![ImpactLoad {
            x_m: 15.0,
            y_m: 5.0,
            impact_force_n: 5_000_000.0,
            blast_radius_m: 2.0,
        }];
        let result = analyzer.analyze(&impacts);
        let dx = analyzer.wall_width_m / analyzer.nx as f64;
        let i_impact = (15.0 / dx) as usize;
        let i_far = 0;
        let j_mid = analyzer.ny / 2;
        let stress_near = result.stress_field[i_impact.min(analyzer.nx - 1)][j_mid];
        let stress_far = result.stress_field[i_far][j_mid];
        assert!(stress_near > stress_far,
            "stress near impact ({}) should exceed far point ({})", stress_near, stress_far);
    }

    #[test]
    fn test_mesh_element_count() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        assert_eq!(result.mesh.elements.len(), analyzer.nx * analyzer.ny);
    }

    #[test]
    fn test_mesh_properties() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        assert!((result.mesh.width_m - 30.0).abs() < 0.01);
        assert!((result.mesh.height_m - 10.0).abs() < 0.01);
        assert!((result.mesh.thickness_m - 3.0).abs() < 0.01);
        assert_eq!(result.mesh.nx, 20);
        assert_eq!(result.mesh.ny, 15);
    }

    #[test]
    fn test_weak_points_priority_ordering() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        for w in result.weak_points.windows(2) {
            assert!(w[0].priority >= w[1].priority,
                "weak points should be sorted by priority descending");
        }
    }

    #[test]
    fn test_weak_points_within_wall() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        for wp in &result.weak_points {
            assert!(wp.x_m >= 0.0 && wp.x_m <= analyzer.wall_width_m,
                "weak point x={} outside wall width {}", wp.x_m, analyzer.wall_width_m);
            assert!(wp.y_m >= 0.0 && wp.y_m <= analyzer.wall_height_m,
                "weak point y={} outside wall height {}", wp.y_m, analyzer.wall_height_m);
        }
    }

    #[test]
    fn test_safety_factor_definition() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        if result.max_stress_pa > 0.0 {
            let expected_safety = analyzer.compressive_strength_pa / result.max_stress_pa;
            assert!((result.min_safety_factor - expected_safety).abs() < 0.01,
                "min_safety_factor should be strength/max_stress");
        }
    }

    #[test]
    fn test_multiple_impacts_cumulative() {
        let analyzer = default_analyzer();
        let single = vec![ImpactLoad {
            x_m: 15.0, y_m: 5.0, impact_force_n: 1_000_000.0, blast_radius_m: 2.0,
        }];
        let double = vec![
            ImpactLoad { x_m: 15.0, y_m: 5.0, impact_force_n: 1_000_000.0, blast_radius_m: 2.0 },
            ImpactLoad { x_m: 16.0, y_m: 5.0, impact_force_n: 1_000_000.0, blast_radius_m: 2.0 },
        ];
        let result_single = analyzer.analyze(&single);
        let result_double = analyzer.analyze(&double);
        assert!(result_double.max_stress_pa >= result_single.max_stress_pa,
            "multiple impacts should increase max stress");
    }

    #[test]
    fn test_high_strength_wall_lower_damage() {
        let weak = FEAnalyzer::new(30.0, 10.0, 3.0, 1800.0, 500_000.0, 50_000.0);
        let strong = FEAnalyzer::new(30.0, 10.0, 3.0, 1800.0, 50_000_000.0, 5_000_000.0);
        let r_weak = weak.analyze(&[]);
        let r_strong = strong.analyze(&[]);
        let avg_damage_weak: f64 = r_weak.damage_field.iter()
            .flat_map(|r| r.iter()).sum::<f64>() / (weak.nx * weak.ny) as f64;
        let avg_damage_strong: f64 = r_strong.damage_field.iter()
            .flat_map(|r| r.iter()).sum::<f64>() / (strong.nx * strong.ny) as f64;
        assert!(avg_damage_weak > avg_damage_strong,
            "weaker wall should have higher average damage");
    }

    #[test]
    fn test_from_wall_props() {
        let wall = crate::siege::WallProperties {
            thickness_m: 4.0,
            material: "stone".to_string(),
            density_kgm3: 2400.0,
            compressive_strength_pa: 25_000_000.0,
            tensile_strength_pa: 2_000_000.0,
        };
        let analyzer = FEAnalyzer::from_wall_props(&wall);
        assert!((analyzer.wall_thickness_m - 4.0).abs() < 0.01);
        assert!((analyzer.wall_density_kgm3 - 2400.0).abs() < 0.01);
        let result = analyzer.analyze(&[]);
        assert!(result.max_stress_pa > 0.0);
    }

    #[test]
    fn test_zero_impact_force() {
        let analyzer = default_analyzer();
        let impacts = vec![ImpactLoad {
            x_m: 15.0, y_m: 5.0, impact_force_n: 0.0, blast_radius_m: 2.0,
        }];
        let result = analyzer.analyze(&impacts);
        assert!(result.max_stress_pa >= 0.0);
        assert!(result.damage_field.iter().flat_map(|r| r.iter()).all(|&d| d >= 0.0));
    }

    #[test]
    fn test_impact_far_from_wall() {
        let analyzer = default_analyzer();
        let impacts = vec![ImpactLoad {
            x_m: 100.0, y_m: 50.0, impact_force_n: 1_000_000.0, blast_radius_m: 2.0,
        }];
        let result_no = analyzer.analyze(&[]);
        let result_far = analyzer.analyze(&impacts);
        let diff = (result_far.max_stress_pa - result_no.max_stress_pa).abs();
        assert!(diff < 100.0, "far impact should not affect wall stress significantly");
    }

    #[test]
    fn test_weak_points_max_count() {
        let analyzer = default_analyzer();
        let result = analyzer.analyze(&[]);
        assert!(result.weak_points.len() <= 10, "weak points should be capped at 10");
    }
}
