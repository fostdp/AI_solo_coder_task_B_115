use serde::{Deserialize, Serialize};
use crate::ammo::AmmoType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallProperties {
    pub thickness_m: f64,
    pub material: String,
    pub density_kgm3: f64,
    pub compressive_strength_pa: f64,
    pub tensile_strength_pa: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiegeInput {
    pub impact_energy_j: f64,
    pub projectile_mass_kg: f64,
    pub projectile_diameter_m: f64,
    pub impact_angle_deg: f64,
    pub wall: WallProperties,
    #[serde(default = "default_ammo_type")]
    pub ammo_type: AmmoType,
}

fn default_ammo_type() -> AmmoType {
    AmmoType::RoundStone
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiegeAssessment {
    pub crater_depth_m: f64,
    pub crater_diameter_m: f64,
    pub damage_ratio: f64,
    pub effectiveness_score: f64,
    pub penetration_potential: f64,
    pub structural_damage: String,
    #[serde(default)]
    pub blast_radius_m: f64,
    #[serde(default)]
    pub contamination_radius_m: f64,
    #[serde(default)]
    pub explosive_damage_ratio: f64,
    #[serde(default)]
    pub contamination_damage_ratio: f64,
    #[serde(default)]
    pub ammo_type: AmmoType,
}

pub fn assess_siege_damage(input: &SiegeInput) -> SiegeAssessment {
    let projectile_radius = input.projectile_diameter_m / 2.0;
    let impact_angle_rad = input.impact_angle_deg.to_radians();

    let normal_energy = input.impact_energy_j * impact_angle_rad.sin().powi(2);

    let crater_volume = estimate_crater_volume(
        normal_energy,
        &input.wall,
        input.projectile_mass_kg,
    );

    let crater_depth = (3.0 * crater_volume / (std::f64::consts::PI * 2.0)).powf(1.0 / 3.0) * 0.8;
    let crater_diameter = crater_depth * 2.5;

    let penetration_ratio = crater_depth / input.wall.thickness_m;
    let kinetic_damage_ratio = if penetration_ratio >= 1.0 {
        1.0
    } else {
        penetration_ratio * 0.7 + (crater_diameter / 10.0).min(0.3)
    };

    let ammo_profile = crate::ammo::AmmoProfile::from_type(input.ammo_type, input.projectile_mass_kg);

    let explosive_damage_ratio = match input.ammo_type {
        AmmoType::GunpowderBomb => {
            let blast_pressure = ammo_profile.explosive_yield_j
                / (std::f64::consts::PI * ammo_profile.blast_radius_m.powi(2));
            let wall_resistance = input.wall.compressive_strength_pa;
            (blast_pressure / wall_resistance * 0.5).min(0.6)
        }
        _ => 0.0,
    };

    let contamination_damage_ratio = match input.ammo_type {
        AmmoType::CorpseShell => {
            let contamination_area = std::f64::consts::PI * ammo_profile.contamination_radius_m.powi(2);
            let wall_area = input.wall.thickness_m * 5.0;
            (contamination_area / wall_area * 0.1).min(0.3)
        }
        _ => 0.0,
    };

    let damage_ratio = (kinetic_damage_ratio + explosive_damage_ratio + contamination_damage_ratio).min(1.0);

    let effectiveness = calculate_effectiveness_with_ammo(
        input.impact_energy_j,
        &input.wall,
        damage_ratio,
        input.projectile_mass_kg,
        input.ammo_type,
        &ammo_profile,
    );

    let structural_damage = classify_damage(damage_ratio);

    SiegeAssessment {
        crater_depth_m: crater_depth,
        crater_diameter_m: crater_diameter,
        damage_ratio: damage_ratio.max(0.0).min(1.0),
        effectiveness_score: effectiveness,
        penetration_potential: penetration_ratio,
        structural_damage,
        blast_radius_m: ammo_profile.blast_radius_m,
        contamination_radius_m: ammo_profile.contamination_radius_m,
        explosive_damage_ratio,
        contamination_damage_ratio,
        ammo_type: input.ammo_type,
    }
}

fn estimate_crater_volume(
    energy_j: f64,
    wall: &WallProperties,
    projectile_mass_kg: f64,
) -> f64 {
    let k_factor = 0.0001;
    let strength_factor = 1.0e6 / wall.compressive_strength_pa.max(1.0e5);
    let density_factor = 1000.0 / wall.density_kgm3.max(100.0);

    let volume = k_factor
        * energy_j.powf(0.7)
        * strength_factor.powf(0.5)
        * density_factor.powf(0.3)
        * (1.0 + projectile_mass_kg / 100.0).powf(0.2);

    volume.max(0.01)
}

fn calculate_effectiveness(
    energy_j: f64,
    wall: &WallProperties,
    damage_ratio: f64,
    projectile_mass_kg: f64,
) -> f64 {
    let break_energy = wall.compressive_strength_pa
        * wall.thickness_m
        * wall.thickness_m
        * wall.density_kgm3
        * 0.001;

    let energy_ratio = energy_j / break_energy.max(1.0);
    let mass_efficiency = 1.0 / (1.0 + projectile_mass_kg / 200.0);

    let score = (damage_ratio * 60.0)
        + (energy_ratio.min(2.0) * 25.0)
        + (mass_efficiency * 15.0);

    score.min(100.0).max(0.0)
}

fn calculate_effectiveness_with_ammo(
    energy_j: f64,
    wall: &WallProperties,
    damage_ratio: f64,
    projectile_mass_kg: f64,
    ammo_type: AmmoType,
    ammo_profile: &crate::ammo::AmmoProfile,
) -> f64 {
    let base_score = calculate_effectiveness(energy_j, wall, damage_ratio, projectile_mass_kg);

    let ammo_bonus = match ammo_type {
        AmmoType::RoundStone => 0.0,
        AmmoType::GunpowderBomb => {
            let blast_effectiveness = (ammo_profile.explosive_yield_j / 1_000_000.0).min(10.0);
            blast_effectiveness * 1.5
        }
        AmmoType::CorpseShell => {
            let contamination_value = ammo_profile.contamination_radius_m * 2.0;
            contamination_value.min(8.0)
        }
    };

    (base_score + ammo_bonus).min(100.0).max(0.0)
}

fn classify_damage(damage_ratio: f64) -> String {
    if damage_ratio >= 0.9 {
        "完全摧毁".to_string()
    } else if damage_ratio >= 0.7 {
        "严重破坏".to_string()
    } else if damage_ratio >= 0.5 {
        "中等破坏".to_string()
    } else if damage_ratio >= 0.3 {
        "轻度破坏".to_string()
    } else if damage_ratio >= 0.1 {
        "表面损伤".to_string()
    } else {
        "无明显损伤".to_string()
    }
}

pub fn optimize_launch_parameters(
    projectile_mass_kg: f64,
    projectile_diameter_m: f64,
    wall: &WallProperties,
    min_velocity: f64,
    max_velocity: f64,
    min_angle: f64,
    max_angle: f64,
) -> (f64, f64, f64) {
    let mut best_score = 0.0;
    let mut best_angle = 45.0;
    let mut best_velocity = max_velocity;

    let angle_steps = 20;
    let velocity_steps = 20;

    for i in 0..=angle_steps {
        let angle = min_angle + (max_angle - min_angle) * (i as f64) / (angle_steps as f64);
        for j in 0..=velocity_steps {
            let velocity = min_velocity + (max_velocity - min_velocity) * (j as f64) / (velocity_steps as f64);

            let impact_energy = estimate_impact_energy(velocity, angle, projectile_mass_kg);

            let siege_input = SiegeInput {
                impact_energy_j: impact_energy,
                projectile_mass_kg,
                projectile_diameter_m,
                impact_angle_deg: angle * 0.8,
                wall: wall.clone(),
                ammo_type: AmmoType::RoundStone,
            };

            let assessment = assess_siege_damage(&siege_input);

            if assessment.effectiveness_score > best_score {
                best_score = assessment.effectiveness_score;
                best_angle = angle;
                best_velocity = velocity;
            }
        }
    }

    (best_angle, best_velocity, best_score)
}

fn estimate_impact_energy(velocity: f64, angle_deg: f64, mass_kg: f64) -> f64 {
    let angle_rad = angle_deg.to_radians();
    let vy = velocity * angle_rad.sin();
    let vx = velocity * angle_rad.cos();

    let max_height = vy * vy / (2.0 * 9.81);
    let impact_vy = (2.0 * 9.81 * max_height).sqrt();
    let impact_vel = (vx * vx + impact_vy * impact_vy).sqrt();

    0.5 * mass_kg * impact_vel * impact_vel
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
    fn test_siege_assessment() {
        let wall = default_wall();

        let input = SiegeInput {
            impact_energy_j: 500_000.0,
            projectile_mass_kg: 90.0,
            projectile_diameter_m: 0.4,
            impact_angle_deg: 45.0,
            wall,
            ammo_type: AmmoType::RoundStone,
        };

        let result = assess_siege_damage(&input);
        assert!(result.crater_depth_m > 0.0);
        assert!(result.damage_ratio >= 0.0 && result.damage_ratio <= 1.0);
        assert!(result.effectiveness_score >= 0.0 && result.effectiveness_score <= 100.0);
    }

    #[test]
    fn test_optimize_parameters() {
        let wall = default_wall();

        let (angle, velocity, score) = optimize_launch_parameters(
            90.0,
            0.4,
            &wall,
            30.0,
            60.0,
            30.0,
            60.0,
        );

        assert!(angle >= 30.0 && angle <= 60.0);
        assert!(velocity >= 30.0 && velocity <= 60.0);
        assert!(score > 0.0);
    }

    #[test]
    fn test_gunpowder_bomb_damage() {
        let wall = default_wall();
        let input_stone = SiegeInput {
            impact_energy_j: 500_000.0,
            projectile_mass_kg: 90.0,
            projectile_diameter_m: 0.4,
            impact_angle_deg: 45.0,
            wall: wall.clone(),
            ammo_type: AmmoType::RoundStone,
        };
        let input_gunpowder = SiegeInput {
            impact_energy_j: 500_000.0,
            projectile_mass_kg: 90.0,
            projectile_diameter_m: 0.4,
            impact_angle_deg: 45.0,
            wall,
            ammo_type: AmmoType::GunpowderBomb,
        };
        let result_stone = assess_siege_damage(&input_stone);
        let result_gunpowder = assess_siege_damage(&input_gunpowder);
        assert!(result_gunpowder.explosive_damage_ratio > 0.0,
            "gunpowder bomb should have explosive damage");
        assert_eq!(result_stone.explosive_damage_ratio, 0.0);
        assert!(result_gunpowder.damage_ratio > result_stone.damage_ratio,
            "gunpowder should deal more total damage than stone");
        assert!(result_gunpowder.blast_radius_m > 0.0);
    }

    #[test]
    fn test_corpse_shell_contamination() {
        let wall = default_wall();
        let input = SiegeInput {
            impact_energy_j: 500_000.0,
            projectile_mass_kg: 90.0,
            projectile_diameter_m: 0.4,
            impact_angle_deg: 45.0,
            wall,
            ammo_type: AmmoType::CorpseShell,
        };
        let result = assess_siege_damage(&input);
        assert!(result.contamination_damage_ratio > 0.0,
            "corpse shell should have contamination damage");
        assert!(result.contamination_radius_m > 0.0);
        assert_eq!(result.explosive_damage_ratio, 0.0);
    }

    #[test]
    fn test_ammo_type_in_assessment() {
        let wall = default_wall();
        for ammo in AmmoType::all() {
            let input = SiegeInput {
                impact_energy_j: 500_000.0,
                projectile_mass_kg: 90.0,
                projectile_diameter_m: 0.4,
                impact_angle_deg: 45.0,
                wall: wall.clone(),
                ammo_type: ammo,
            };
            let result = assess_siege_damage(&input);
            assert_eq!(result.ammo_type, ammo, "ammo type should be preserved in assessment");
        }
    }

    #[test]
    fn test_damage_ratio_bounded() {
        let wall = default_wall();
        for ammo in AmmoType::all() {
            let input = SiegeInput {
                impact_energy_j: 10_000_000.0,
                projectile_mass_kg: 300.0,
                projectile_diameter_m: 0.6,
                impact_angle_deg: 45.0,
                wall: wall.clone(),
                ammo_type: ammo,
            };
            let result = assess_siege_damage(&input);
            assert!(result.damage_ratio >= 0.0 && result.damage_ratio <= 1.0,
                "damage_ratio {:?} = {} out of [0,1]", ammo, result.damage_ratio);
        }
    }

    #[test]
    fn test_effectiveness_score_bounded() {
        let wall = default_wall();
        for ammo in AmmoType::all() {
            let input = SiegeInput {
                impact_energy_j: 500_000.0,
                projectile_mass_kg: 90.0,
                projectile_diameter_m: 0.4,
                impact_angle_deg: 45.0,
                wall: wall.clone(),
                ammo_type: ammo,
            };
            let result = assess_siege_damage(&input);
            assert!(result.effectiveness_score >= 0.0 && result.effectiveness_score <= 100.0,
                "effectiveness {:?} = {} out of [0,100]", ammo, result.effectiveness_score);
        }
    }

    #[test]
    fn test_damage_classification() {
        assert_eq!(classify_damage(0.95), "完全摧毁");
        assert_eq!(classify_damage(0.75), "严重破坏");
        assert_eq!(classify_damage(0.55), "中等破坏");
        assert_eq!(classify_damage(0.35), "轻度破坏");
        assert_eq!(classify_damage(0.15), "表面损伤");
        assert_eq!(classify_damage(0.05), "无明显损伤");
    }

    #[test]
    fn test_siege_input_default_ammo() {
        let json = r#"{"impact_energy_j":500000,"projectile_mass_kg":90,"projectile_diameter_m":0.4,"impact_angle_deg":45,"wall":{"thickness_m":3,"material":"rammed_earth","density_kgm3":1800,"compressive_strength_pa":2000000,"tensile_strength_pa":200000}}"#;
        let input: SiegeInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.ammo_type, AmmoType::RoundStone);
    }

    #[test]
    fn test_strong_wall_less_damage() {
        let weak_wall = WallProperties {
            thickness_m: 3.0,
            material: "rammed_earth".to_string(),
            density_kgm3: 1800.0,
            compressive_strength_pa: 2_000_000.0,
            tensile_strength_pa: 200_000.0,
        };
        let strong_wall = WallProperties {
            thickness_m: 6.0,
            material: "stone".to_string(),
            density_kgm3: 2400.0,
            compressive_strength_pa: 25_000_000.0,
            tensile_strength_pa: 2_000_000.0,
        };
        let input_weak = SiegeInput {
            impact_energy_j: 500_000.0,
            projectile_mass_kg: 90.0,
            projectile_diameter_m: 0.4,
            impact_angle_deg: 45.0,
            wall: weak_wall,
            ammo_type: AmmoType::RoundStone,
        };
        let input_strong = SiegeInput {
            impact_energy_j: 500_000.0,
            projectile_mass_kg: 90.0,
            projectile_diameter_m: 0.4,
            impact_angle_deg: 45.0,
            wall: strong_wall,
            ammo_type: AmmoType::RoundStone,
        };
        let r_weak = assess_siege_damage(&input_weak);
        let r_strong = assess_siege_damage(&input_strong);
        assert!(r_weak.damage_ratio > r_strong.damage_ratio,
            "weaker wall should have more damage");
    }

    #[test]
    fn test_higher_energy_more_damage() {
        let wall = default_wall();
        let input_low = SiegeInput {
            impact_energy_j: 100_000.0,
            projectile_mass_kg: 90.0,
            projectile_diameter_m: 0.4,
            impact_angle_deg: 45.0,
            wall: wall.clone(),
            ammo_type: AmmoType::RoundStone,
        };
        let input_high = SiegeInput {
            impact_energy_j: 5_000_000.0,
            projectile_mass_kg: 90.0,
            projectile_diameter_m: 0.4,
            impact_angle_deg: 45.0,
            wall,
            ammo_type: AmmoType::RoundStone,
        };
        let r_low = assess_siege_damage(&input_low);
        let r_high = assess_siege_damage(&input_high);
        assert!(r_high.damage_ratio > r_low.damage_ratio);
        assert!(r_high.effectiveness_score > r_low.effectiveness_score);
    }

    #[test]
    fn test_gunpowder_effectiveness_bonus() {
        let wall = default_wall();
        let input_stone = SiegeInput {
            impact_energy_j: 500_000.0,
            projectile_mass_kg: 90.0,
            projectile_diameter_m: 0.4,
            impact_angle_deg: 45.0,
            wall: wall.clone(),
            ammo_type: AmmoType::RoundStone,
        };
        let input_gunpowder = SiegeInput {
            impact_energy_j: 500_000.0,
            projectile_mass_kg: 90.0,
            projectile_diameter_m: 0.4,
            impact_angle_deg: 45.0,
            wall,
            ammo_type: AmmoType::GunpowderBomb,
        };
        let r_stone = assess_siege_damage(&input_stone);
        let r_gunpowder = assess_siege_damage(&input_gunpowder);
        assert!(r_gunpowder.effectiveness_score >= r_stone.effectiveness_score,
            "gunpowder should have >= effectiveness than stone");
    }

    #[test]
    fn test_zero_impact_energy() {
        let wall = default_wall();
        let input = SiegeInput {
            impact_energy_j: 0.0,
            projectile_mass_kg: 90.0,
            projectile_diameter_m: 0.4,
            impact_angle_deg: 45.0,
            wall,
            ammo_type: AmmoType::RoundStone,
        };
        let result = assess_siege_damage(&input);
        assert!(result.damage_ratio >= 0.0 && result.damage_ratio <= 1.0);
        assert!(result.crater_depth_m > 0.0 || result.damage_ratio == 0.0);
    }
}
