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

    #[test]
    fn test_fe_analysis_no_impacts() {
        let analyzer = FEAnalyzer::new(30.0, 10.0, 3.0, 1800.0, 2_000_000.0, 200_000.0);
        let result = analyzer.analyze(&[]);
        assert!(result.max_stress_pa > 0.0);
        assert!(result.min_safety_factor > 0.0);
    }

    #[test]
    fn test_fe_analysis_with_impacts() {
        let analyzer = FEAnalyzer::new(30.0, 10.0, 3.0, 1800.0, 2_000_000.0, 200_000.0);
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
}
