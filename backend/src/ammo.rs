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

    pub fn drag_modifier(&self) -> f64 {
        match self {
            AmmoType::RoundStone => 1.0,
            AmmoType::GunpowderBomb => 1.35,
            AmmoType::CorpseShell => 1.8,
        }
    }

    pub fn shape_factor(&self) -> f64 {
        match self {
            AmmoType::RoundStone => 1.0,
            AmmoType::GunpowderBomb => 0.85,
            AmmoType::CorpseShell => 0.65,
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
}
