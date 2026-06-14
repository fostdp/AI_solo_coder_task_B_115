use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmmoType {
    RoundStone,
    GunpowderBomb,
    CorpseShell,
}

impl Default for AmmoType {
    fn default() -> Self {
        AmmoType::RoundStone
    }
}

impl AmmoType {
    pub fn density_kgm3(&self) -> f64 {
        match self {
            AmmoType::RoundStone => 2600.0,
            AmmoType::GunpowderBomb => 1800.0,
            AmmoType::CorpseShell => 900.0,
        }
    }

    pub fn base_drag_coefficient(&self) -> f64 {
        match self {
            AmmoType::RoundStone => 0.47,
            AmmoType::GunpowderBomb => 0.72,
            AmmoType::CorpseShell => 1.15,
        }
    }

    pub fn drag_modifier(&self) -> f64 {
        self.base_drag_coefficient() / 0.47
    }

    pub fn shape_factor(&self) -> f64 {
        match self {
            AmmoType::RoundStone => 1.0,
            AmmoType::GunpowderBomb => 0.88,
            AmmoType::CorpseShell => 0.62,
        }
    }

    pub fn explosive_yield_j(&self, mass_kg: f64) -> f64 {
        match self {
            AmmoType::RoundStone => 0.0,
            AmmoType::GunpowderBomb => mass_kg * 0.3 * 3_000_000.0,
            AmmoType::CorpseShell => 0.0,
        }
    }

    pub fn blast_radius_m(&self, mass_kg: f64) -> f64 {
        match self {
            AmmoType::RoundStone => 0.0,
            AmmoType::GunpowderBomb => (mass_kg * 0.3).powf(1.0 / 3.0) * 2.0,
            AmmoType::CorpseShell => 0.0,
        }
    }

    pub fn contamination_radius_m(&self, mass_kg: f64) -> f64 {
        match self {
            AmmoType::RoundStone => 0.0,
            AmmoType::GunpowderBomb => 0.0,
            AmmoType::CorpseShell => mass_kg.powf(0.4) * 0.5,
        }
    }

    pub fn contamination_duration_hours(&self) -> f64 {
        match self {
            AmmoType::RoundStone => 0.0,
            AmmoType::GunpowderBomb => 0.0,
            AmmoType::CorpseShell => 72.0,
        }
    }

    pub fn name_zh(&self) -> &str {
        match self {
            AmmoType::RoundStone => "圆石弹",
            AmmoType::GunpowderBomb => "火药弹",
            AmmoType::CorpseShell => "腐尸弹",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            AmmoType::RoundStone => "标准石质弹丸，球形低阻，纯动能打击",
            AmmoType::GunpowderBomb => "内装火药的爆炸弹，命中后产生冲击波和破片",
            AmmoType::CorpseShell => "装载腐尸的生物弹，气动力学差但造成持续污染",
        }
    }

    pub fn all() -> Vec<AmmoType> {
        vec![AmmoType::RoundStone, AmmoType::GunpowderBomb, AmmoType::CorpseShell]
    }

    pub fn drag_coefficient_at_reynolds(&self, reynolds: f64) -> f64 {
        let base_cd = self.base_drag_coefficient();
        let re = reynolds.max(1.0);

        match self {
            AmmoType::RoundStone => {
                if re < 1.0 {
                    base_cd * (24.0 / re)
                } else if re < 1000.0 {
                    base_cd * (1.0 + 6.0 / re.sqrt() + 12.0 / re)
                } else if re < 300_000.0 {
                    base_cd
                } else if re < 500_000.0 {
                    let trans = (re - 300_000.0) / 200_000.0;
                    base_cd * (1.0 - trans * 0.5)
                } else {
                    base_cd * 0.5
                }
            }
            AmmoType::GunpowderBomb => {
                let shape_factor = 1.15;
                if re < 1000.0 {
                    base_cd * shape_factor * (1.0 + 3.0 / re.sqrt())
                } else if re < 200_000.0 {
                    base_cd * shape_factor
                } else {
                    base_cd * shape_factor * 0.9
                }
            }
            AmmoType::CorpseShell => {
                let roughness_factor = 1.3;
                let irregularity_factor = 1.2;
                if re < 500.0 {
                    base_cd * roughness_factor * irregularity_factor * (1.0 + 2.0 / re.sqrt())
                } else if re < 100_000.0 {
                    base_cd * roughness_factor * irregularity_factor
                } else if re < 400_000.0 {
                    let transition = (re - 100_000.0) / 300_000.0;
                    base_cd * roughness_factor * irregularity_factor * (1.0 - transition * 0.15)
                } else {
                    base_cd * roughness_factor * irregularity_factor * 0.85
                }
            }
        }
    }

    pub fn reynolds_number(&self, velocity_m_s: f64, characteristic_length_m: f64, kinematic_viscosity_m2_s: f64) -> f64 {
        (velocity_m_s * characteristic_length_m) / kinematic_viscosity_m2_s.max(1e-9)
    }

    pub fn drag_estimation_notes(&self) -> &str {
        match self {
            AmmoType::RoundStone => "球形标准阻力系数 Cd=0.47（亚音速），参考Hoerner《流体动力阻力》",
            AmmoType::GunpowderBomb => "柱球形带引信，Cd≈0.72，基于军械弹道学手册数据",
            AmmoType::CorpseShell => "不规则生物形状，表面粗糙度高，Cd≈1.15，基于人体坠落实验和不规则物体风洞数据外推",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmoProfile {
    pub ammo_type: AmmoType,
    pub density_kgm3: f64,
    pub drag_modifier: f64,
    pub shape_factor: f64,
    pub explosive_yield_j: f64,
    pub blast_radius_m: f64,
    pub contamination_radius_m: f64,
    pub contamination_duration_hours: f64,
    pub ballistic_coefficient_modifier: f64,
}

impl AmmoProfile {
    pub fn from_type(ammo: AmmoType, mass_kg: f64) -> Self {
        let density = ammo.density_kgm3();
        let volume = mass_kg / density;
        let diameter = 2.0 * (3.0 * volume / (4.0 * std::f64::consts::PI)).powf(1.0 / 3.0);
        let cross_section = std::f64::consts::PI * (diameter / 2.0).powi(2);
        let ballistic_coeff_modifier = (density * ammo.shape_factor()) / (2600.0 * 1.0);

        Self {
            ammo_type: ammo,
            density_kgm3: density,
            drag_modifier: ammo.drag_modifier(),
            shape_factor: ammo.shape_factor(),
            explosive_yield_j: ammo.explosive_yield_j(mass_kg),
            blast_radius_m: ammo.blast_radius_m(mass_kg),
            contamination_radius_m: ammo.contamination_radius_m(mass_kg),
            contamination_duration_hours: ammo.contamination_duration_hours(),
            ballistic_coefficient_modifier: ballistic_coeff_modifier,
        }
    }

    pub fn effective_diameter(&self, mass_kg: f64) -> f64 {
        let volume = mass_kg / self.density_kgm3.max(1.0);
        let radius = (3.0 * volume / (4.0 * std::f64::consts::PI)).powf(1.0 / 3.0);
        2.0 * radius / self.shape_factor.sqrt()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmoComparison {
    pub round_stone: AmmoComparisonItem,
    pub gunpowder_bomb: AmmoComparisonItem,
    pub corpse_shell: AmmoComparisonItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmoComparisonItem {
    pub ammo_type: AmmoType,
    pub estimated_range_m: f64,
    pub estimated_max_height_m: f64,
    pub estimated_impact_energy_j: f64,
    pub explosive_energy_j: f64,
    pub total_damage_potential: f64,
    pub blast_radius_m: f64,
    pub contamination_radius_m: f64,
}

pub fn compare_ammo(
    velocity: f64,
    angle_deg: f64,
    mass_kg: f64,
    air_density_kgm3: f64,
) -> AmmoComparison {
    let types = AmmoType::all();
    let mut items: Vec<AmmoComparisonItem> = types
        .iter()
        .map(|at| {
            let profile = AmmoProfile::from_type(*at, mass_kg);
            let result = super::ballistics::simulate_ballistics_with_ammo(
                velocity,
                angle_deg,
                mass_kg,
                air_density_kgm3,
                0.0,
                0.0,
                5.0,
                &profile,
            );
            let total_damage = result.impact_kinetic_energy_j
                + profile.explosive_yield_j * 0.3
                + profile.contamination_radius_m * 1000.0;

            AmmoComparisonItem {
                ammo_type: *at,
                estimated_range_m: result.range_m,
                estimated_max_height_m: result.max_height_m,
                estimated_impact_energy_j: result.impact_kinetic_energy_j,
                explosive_energy_j: profile.explosive_yield_j,
                total_damage_potential: total_damage,
                blast_radius_m: profile.blast_radius_m,
                contamination_radius_m: profile.contamination_radius_m,
            }
        })
        .collect();

    AmmoComparison {
        round_stone: items.remove(0),
        gunpowder_bomb: items.remove(0),
        corpse_shell: items.remove(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ammo_profiles() {
        for at in AmmoType::all() {
            let profile = AmmoProfile::from_type(at, 90.0);
            assert!(profile.density_kgm3 > 0.0);
            assert!(profile.drag_modifier > 0.0);
            assert!(profile.effective_diameter(90.0) > 0.0);
        }
    }

    #[test]
    fn test_gunpowder_explosive_yield() {
        let yield_j = AmmoType::GunpowderBomb.explosive_yield_j(90.0);
        assert!(yield_j > 0.0);
        let yield_stone = AmmoType::RoundStone.explosive_yield_j(90.0);
        assert_eq!(yield_stone, 0.0);
    }

    #[test]
    fn test_corpse_contamination() {
        let radius = AmmoType::CorpseShell.contamination_radius_m(90.0);
        assert!(radius > 0.0);
        let radius_stone = AmmoType::RoundStone.contamination_radius_m(90.0);
        assert_eq!(radius_stone, 0.0);
    }

    #[test]
    fn test_density_ordering() {
        let d_stone = AmmoType::RoundStone.density_kgm3();
        let d_gunpowder = AmmoType::GunpowderBomb.density_kgm3();
        let d_corpse = AmmoType::CorpseShell.density_kgm3();
        assert!(d_stone > d_gunpowder, "stone({}) should be denser than gunpowder({})", d_stone, d_gunpowder);
        assert!(d_gunpowder > d_corpse, "gunpowder({}) should be denser than corpse({})", d_gunpowder, d_corpse);
    }

    #[test]
    fn test_drag_modifier_ordering() {
        let drag_stone = AmmoType::RoundStone.drag_modifier();
        let drag_gunpowder = AmmoType::GunpowderBomb.drag_modifier();
        let drag_corpse = AmmoType::CorpseShell.drag_modifier();
        assert_eq!(drag_stone, 1.0, "round stone is baseline drag");
        assert!(drag_gunpowder > drag_stone, "gunpowder bomb should have higher drag than stone");
        assert!(drag_corpse > drag_gunpowder, "corpse shell should have highest drag");
    }

    #[test]
    fn test_shape_factor_range() {
        for at in AmmoType::all() {
            let sf = at.shape_factor();
            assert!(sf > 0.0 && sf <= 1.0, "shape_factor {:?} = {} out of range", at, sf);
        }
        assert!(AmmoType::RoundStone.shape_factor() > AmmoType::CorpseShell.shape_factor());
    }

    #[test]
    fn test_explosive_yield_only_gunpowder() {
        assert_eq!(AmmoType::RoundStone.explosive_yield_j(100.0), 0.0);
        assert_eq!(AmmoType::CorpseShell.explosive_yield_j(100.0), 0.0);
        let yield_val = AmmoType::GunpowderBomb.explosive_yield_j(100.0);
        let expected = 100.0 * 0.3 * 3_000_000.0;
        assert!((yield_val - expected).abs() < 1.0, "yield should be mass*0.3*3MJ");
    }

    #[test]
    fn test_blast_radius_only_gunpowder() {
        assert_eq!(AmmoType::RoundStone.blast_radius_m(100.0), 0.0);
        assert_eq!(AmmoType::CorpseShell.blast_radius_m(100.0), 0.0);
        let r = AmmoType::GunpowderBomb.blast_radius_m(100.0);
        assert!(r > 0.0, "gunpowder blast radius should be positive");
    }

    #[test]
    fn test_contamination_only_corpse() {
        assert_eq!(AmmoType::RoundStone.contamination_radius_m(50.0), 0.0);
        assert_eq!(AmmoType::GunpowderBomb.contamination_radius_m(50.0), 0.0);
        let r = AmmoType::CorpseShell.contamination_radius_m(50.0);
        assert!(r > 0.0, "corpse contamination radius should be positive");
        assert_eq!(AmmoType::RoundStone.contamination_duration_hours(), 0.0);
        assert_eq!(AmmoType::GunpowderBomb.contamination_duration_hours(), 0.0);
        assert_eq!(AmmoType::CorpseShell.contamination_duration_hours(), 72.0);
    }

    #[test]
    fn test_ammo_profile_from_type_effective_diameter() {
        let mass = 90.0;
        let profile_stone = AmmoProfile::from_type(AmmoType::RoundStone, mass);
        let profile_corpse = AmmoProfile::from_type(AmmoType::CorpseShell, mass);
        let d_stone = profile_stone.effective_diameter(mass);
        let d_corpse = profile_corpse.effective_diameter(mass);
        assert!(d_stone > 0.0);
        assert!(d_corpse > 0.0);
        assert!(d_corpse > d_stone, "corpse (lower density) should have larger effective diameter");
    }

    #[test]
    fn test_ammo_profile_ballistic_coeff_modifier() {
        let profile_stone = AmmoProfile::from_type(AmmoType::RoundStone, 90.0);
        assert!((profile_stone.ballistic_coefficient_modifier - 1.0).abs() < 0.01,
            "stone should have ballistic_coeff_modifier ~1.0, got {}", profile_stone.ballistic_coefficient_modifier);
        let profile_corpse = AmmoProfile::from_type(AmmoType::CorpseShell, 90.0);
        assert!(profile_corpse.ballistic_coefficient_modifier < profile_stone.ballistic_coefficient_modifier,
            "corpse should have lower ballistic coeff than stone");
    }

    #[test]
    fn test_explosive_yield_scales_with_mass() {
        let y1 = AmmoType::GunpowderBomb.explosive_yield_j(50.0);
        let y2 = AmmoType::GunpowderBomb.explosive_yield_j(100.0);
        assert!((y2 / y1 - 2.0).abs() < 0.01, "yield should scale linearly with mass");
    }

    #[test]
    fn test_blast_radius_scales_sublinearly() {
        let r1 = AmmoType::GunpowderBomb.blast_radius_m(50.0);
        let r2 = AmmoType::GunpowderBomb.blast_radius_m(400.0);
        assert!(r2 < r1 * 4.0, "blast radius should scale sublinearly (cube root)");
    }

    #[test]
    fn test_ammo_profile_extreme_mass() {
        let profile = AmmoProfile::from_type(AmmoType::RoundStone, 0.001);
        assert!(profile.effective_diameter(0.001) > 0.0);
        assert!(profile.explosive_yield_j == 0.0);
        let profile2 = AmmoProfile::from_type(AmmoType::RoundStone, 10000.0);
        assert!(profile2.effective_diameter(10000.0) > 0.0);
    }

    #[test]
    fn test_ammo_profile_gunpowder_extreme() {
        let tiny = AmmoProfile::from_type(AmmoType::GunpowderBomb, 0.01);
        assert!(tiny.blast_radius_m >= 0.0);
        assert!(tiny.explosive_yield_j >= 0.0);
        let huge = AmmoProfile::from_type(AmmoType::GunpowderBomb, 50000.0);
        assert!(huge.blast_radius_m > 0.0);
        assert!(huge.explosive_yield_j > 0.0);
    }

    #[test]
    fn test_ammo_type_default() {
        assert_eq!(AmmoType::default(), AmmoType::RoundStone);
    }

    #[test]
    fn test_ammo_type_all_returns_three() {
        assert_eq!(AmmoType::all().len(), 3);
    }

    #[test]
    fn test_ammo_type_equality() {
        assert_eq!(AmmoType::RoundStone, AmmoType::RoundStone);
        assert_ne!(AmmoType::RoundStone, AmmoType::GunpowderBomb);
    }

    #[test]
    fn test_compare_ammo_returns_different_ranges() {
        let comparison = compare_ammo(50.0, 45.0, 90.0, 1.225);
        assert!(comparison.round_stone.estimated_range_m > 0.0);
        assert!(comparison.gunpowder_bomb.estimated_range_m > 0.0);
        assert!(comparison.corpse_shell.estimated_range_m > 0.0);
        assert!(comparison.round_stone.estimated_range_m >= comparison.corpse_shell.estimated_range_m,
            "stone should fly further than corpse due to lower drag");
    }

    #[test]
    fn test_compare_ammo_damage_potential() {
        let comparison = compare_ammo(50.0, 45.0, 90.0, 1.225);
        assert!(comparison.gunpowder_bomb.explosive_energy_j > 0.0);
        assert_eq!(comparison.round_stone.explosive_energy_j, 0.0);
        assert!(comparison.corpse_shell.contamination_radius_m > 0.0);
        assert_eq!(comparison.round_stone.contamination_radius_m, 0.0);
    }

    #[test]
    fn test_compare_ammo_extreme_velocity() {
        let comparison = compare_ammo(1.0, 45.0, 90.0, 1.225);
        assert!(comparison.round_stone.estimated_range_m > 0.0);
        let comparison2 = compare_ammo(200.0, 45.0, 90.0, 1.225);
        assert!(comparison2.round_stone.estimated_range_m > comparison.round_stone.estimated_range_m);
    }

    #[test]
    fn test_compare_ammo_zero_angle() {
        let comparison = compare_ammo(50.0, 0.1, 90.0, 1.225);
        assert!(comparison.round_stone.estimated_range_m > 0.0);
        assert!(comparison.round_stone.estimated_max_height_m < 10.0);
    }

    #[test]
    fn test_compare_ammo_high_angle() {
        let comparison = compare_ammo(50.0, 85.0, 90.0, 1.225);
        assert!(comparison.round_stone.estimated_max_height_m > comparison.round_stone.estimated_range_m,
            "at 85deg, max_height should exceed range");
    }

    #[test]
    fn test_effective_diameter_density_relationship() {
        let d_stone = AmmoProfile::from_type(AmmoType::RoundStone, 90.0).effective_diameter(90.0);
        let d_corpse = AmmoProfile::from_type(AmmoType::CorpseShell, 90.0).effective_diameter(90.0);
        assert!(d_corpse > d_stone,
            "lower density → larger volume → larger diameter");
    }
}
