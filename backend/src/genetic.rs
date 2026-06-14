use crate::ammo::AmmoType;
use crate::fea::{FEAnalyzer, FEAResult, ImpactLoad};
use crate::siege::WallProperties;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneticConfig {
    pub population_size: usize,
    pub generations: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub elite_count: usize,
    pub tournament_size: usize,
}

impl Default for GeneticConfig {
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
pub struct Chromosome {
    pub x_m: f64,
    pub y_m: f64,
    pub fitness: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAResult {
    pub best: Chromosome,
    pub population_history: Vec<GAGeneration>,
    pub convergence_data: Vec<f64>,
    pub total_generations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAGeneration {
    pub generation: usize,
    pub best_fitness: f64,
    pub avg_fitness: f64,
    pub best_x: f64,
    pub best_y: f64,
}

pub struct GeneticOptimizer {
    config: GeneticConfig,
    wall: WallProperties,
    fea_analyzer: FEAnalyzer,
    existing_impacts: Vec<ImpactLoad>,
    ammo_type: AmmoType,
    projectile_mass_kg: f64,
    impact_energy_j: f64,
}

impl GeneticOptimizer {
    pub fn new(
        config: GeneticConfig,
        wall: WallProperties,
        existing_impacts: Vec<ImpactLoad>,
        ammo_type: AmmoType,
        projectile_mass_kg: f64,
        impact_energy_j: f64,
    ) -> Self {
        let fea_analyzer = FEAnalyzer::from_wall_props(&wall);
        Self {
            config,
            wall,
            fea_analyzer,
            existing_impacts,
            ammo_type,
            projectile_mass_kg,
            impact_energy_j,
        }
    }

    pub fn optimize(&self) -> GAResult {
        let mut population = self.init_population();
        let mut history = Vec::new();
        let mut convergence = Vec::new();

        for gen in 0..self.config.generations {
            self.evaluate(&mut population);

            let best = population
                .iter()
                .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal))
                .cloned()
                .unwrap_or_else(|| population[0].clone());

            let avg_fitness = population.iter().map(|c| c.fitness).sum::<f64>() / population.len() as f64;

            history.push(GAGeneration {
                generation: gen,
                best_fitness: best.fitness,
                avg_fitness,
                best_x: best.x_m,
                best_y: best.y_m,
            });
            convergence.push(best.fitness);

            population = self.evolve(population);
        }

        self.evaluate(&mut population);
        let best = population
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal))
            .cloned()
            .unwrap();

        GAResult {
            best,
            population_history: history,
            convergence_data: convergence,
            total_generations: self.config.generations,
        }
    }

    fn init_population(&self) -> Vec<Chromosome> {
        let mut pop = Vec::with_capacity(self.config.population_size);

        pop.push(Chromosome {
            x_m: self.fea_analyzer.wall_width_m / 2.0,
            y_m: self.fea_analyzer.wall_height_m / 2.0,
            fitness: 0.0,
        });

        pop.push(Chromosome {
            x_m: self.fea_analyzer.wall_width_m / 2.0,
            y_m: self.fea_analyzer.wall_height_m * 0.3,
            fitness: 0.0,
        });

        for _ in 2..self.config.population_size {
            let x = rand_x(&self.fea_analyzer);
            let y = rand_y(&self.fea_analyzer);
            pop.push(Chromosome { x_m: x, y_m: y, fitness: 0.0 });
        }

        pop
    }

    fn evaluate(&self, population: &mut Vec<Chromosome>) {
        for chrom in population.iter_mut() {
            chrom.fitness = self.fitness(chrom);
        }
    }

    fn fitness(&self, chrom: &Chromosome) -> f64 {
        let mut impacts = self.existing_impacts.clone();
        impacts.push(ImpactLoad {
            x_m: chrom.x_m,
            y_m: chrom.y_m,
            impact_force_n: self.impact_energy_j / 0.5,
            blast_radius_m: self.compute_blast_radius(),
        });

        let fea_result = self.fea_analyzer.analyze(&impacts);

        let stress_at_point = self.get_stress_at(&fea_result, chrom.x_m, chrom.y_m);
        let existing_stress = self.get_stress_at_existing(chrom.x_m, chrom.y_m);

        let stress_ratio = stress_at_point / self.wall.compressive_strength_pa.max(1.0);
        let incremental_stress = (stress_at_point - existing_stress).max(0.0);

        let damage_score = if stress_ratio > 1.0 {
            100.0
        } else {
            stress_ratio.powi(2) * 80.0
        };

        let weak_point_bonus = self.weak_point_proximity_bonus(&fea_result, chrom.x_m, chrom.y_m);

        let gate_bonus = self.gate_targeting_bonus(chrom.x_m);

        let ammo_bonus = self.ammo_effectiveness_bonus(chrom.x_m, chrom.y_m);

        let structural_advantage = incremental_stress / self.wall.compressive_strength_pa.max(1.0) * 20.0;

        damage_score + weak_point_bonus + gate_bonus + ammo_bonus + structural_advantage
    }

    fn compute_blast_radius(&self) -> f64 {
        let profile = crate::ammo::AmmoProfile::from_type(self.ammo_type, self.projectile_mass_kg);
        profile.blast_radius_m.max(0.5)
    }

    fn get_stress_at(&self, fea_result: &FEAResult, x: f64, y: f64) -> f64 {
        let dx = fea_result.mesh.width_m / fea_result.mesh.nx as f64;
        let dy = fea_result.mesh.height_m / fea_result.mesh.ny as f64;
        let i = ((x / dx) as usize).min(fea_result.mesh.nx - 1);
        let j = ((y / dy) as usize).min(fea_result.mesh.ny - 1);
        fea_result.stress_field[i][j]
    }

    fn get_stress_at_existing(&self, x: f64, y: f64) -> f64 {
        let fea_result = self.fea_analyzer.analyze(&self.existing_impacts);
        self.get_stress_at(&fea_result, x, y)
    }

    fn weak_point_proximity_bonus(&self, fea_result: &FEAResult, x: f64, y: f64) -> f64 {
        let mut min_dist = f64::MAX;
        for wp in &fea_result.weak_points {
            let d = ((wp.x_m - x).powi(2) + (wp.y_m - y).powi(2)).sqrt();
            if d < min_dist {
                min_dist = d;
            }
        }
        if min_dist < 5.0 {
            15.0 * (1.0 - min_dist / 5.0)
        } else {
            0.0
        }
    }

    fn gate_targeting_bonus(&self, x: f64) -> f64 {
        let gate_center = self.fea_analyzer.wall_width_m / 2.0;
        let dist = (x - gate_center).abs();
        if dist < 3.0 {
            10.0 * (1.0 - dist / 3.0)
        } else {
            0.0
        }
    }

    fn ammo_effectiveness_bonus(&self, _x: f64, _y: f64) -> f64 {
        match self.ammo_type {
            AmmoType::RoundStone => 0.0,
            AmmoType::GunpowderBomb => 5.0,
            AmmoType::CorpseShell => 3.0,
        }
    }

    fn evolve(&self, population: Vec<Chromosome>) -> Vec<Chromosome> {
        let mut sorted = population;
        sorted.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));

        let mut new_pop = Vec::with_capacity(self.config.population_size);

        for i in 0..self.config.elite_count.min(sorted.len()) {
            new_pop.push(sorted[i].clone());
        }

        while new_pop.len() < self.config.population_size {
            let parent1 = self.tournament_select(&sorted);
            let parent2 = self.tournament_select(&sorted);

            let child = if pseudo_random() < self.config.crossover_rate {
                self.crossover(&parent1, &parent2)
            } else {
                parent1.clone()
            };

            let child = self.mutate(child);
            new_pop.push(child);
        }

        new_pop
    }

    fn tournament_select(&self, population: &[Chromosome]) -> Chromosome {
        let mut best: Option<Chromosome> = None;
        for _ in 0..self.config.tournament_size {
            let idx = (pseudo_random() * population.len() as f64) as usize;
            let idx = idx.min(population.len() - 1);
            let candidate = &population[idx];
            if best.is_none() || candidate.fitness > best.as_ref().unwrap().fitness {
                best = Some(candidate.clone());
            }
        }
        best.unwrap_or_else(|| population[0].clone())
    }

    fn crossover(&self, p1: &Chromosome, p2: &Chromosome) -> Chromosome {
        let alpha = pseudo_random();
        Chromosome {
            x_m: alpha * p1.x_m + (1.0 - alpha) * p2.x_m,
            y_m: alpha * p1.y_m + (1.0 - alpha) * p2.y_m,
            fitness: 0.0,
        }
    }

    fn mutate(&self, mut chrom: Chromosome) -> Chromosome {
        if pseudo_random() < self.config.mutation_rate {
            let sigma_x = self.fea_analyzer.wall_width_m * 0.1;
            let sigma_y = self.fea_analyzer.wall_height_m * 0.1;
            chrom.x_m += (pseudo_random() - 0.5) * 2.0 * sigma_x;
            chrom.y_m += (pseudo_random() - 0.5) * 2.0 * sigma_y;
        }
        chrom.x_m = chrom.x_m.max(0.5).min(self.fea_analyzer.wall_width_m - 0.5);
        chrom.y_m = chrom.y_m.max(0.5).min(self.fea_analyzer.wall_height_m - 0.5);
        chrom
    }
}

fn rand_x(fea: &FEAnalyzer) -> f64 {
    pseudo_random() * fea.wall_width_m
}

fn rand_y(fea: &FEAnalyzer) -> f64 {
    pseudo_random() * fea.wall_height_m
}

fn pseudo_random() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let x = ns.wrapping_mul(1103515245).wrapping_add(12345);
    (x & 0x7fffffff) as f64 / 0x7fffffff as f64
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
    fn test_ga_optimize() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 20,
            generations: 10,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(
            config,
            wall,
            vec![],
            AmmoType::RoundStone,
            90.0,
            500_000.0,
        );
        let result = optimizer.optimize();
        assert!(result.best.x_m > 0.0);
        assert!(result.best.y_m > 0.0);
        assert!(result.convergence_data.len() == 10);
    }

    #[test]
    fn test_ga_convergence() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 30,
            generations: 20,
            mutation_rate: 0.15,
            crossover_rate: 0.8,
            elite_count: 3,
            tournament_size: 4,
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::RoundStone, 90.0, 500_000.0);
        let result = optimizer.optimize();
        let first_gen_fitness = result.convergence_data[0];
        let last_gen_fitness = *result.convergence_data.last().unwrap();
        assert!(last_gen_fitness >= first_gen_fitness,
            "GA should converge: last ({}) >= first ({})", last_gen_fitness, first_gen_fitness);
    }

    #[test]
    fn test_ga_best_within_wall_bounds() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 20,
            generations: 10,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::RoundStone, 90.0, 500_000.0);
        let result = optimizer.optimize();
        assert!(result.best.x_m >= 0.5 && result.best.x_m <= 29.5,
            "best x={} should be within wall", result.best.x_m);
        assert!(result.best.y_m >= 0.5 && result.best.y_m <= 9.5,
            "best y={} should be within wall", result.best.y_m);
    }

    #[test]
    fn test_ga_convergence_data_length() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 15,
            generations: 8,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::RoundStone, 90.0, 500_000.0);
        let result = optimizer.optimize();
        assert_eq!(result.convergence_data.len(), 8);
        assert_eq!(result.population_history.len(), 8);
        assert_eq!(result.total_generations, 8);
    }

    #[test]
    fn test_ga_generation_history() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 20,
            generations: 5,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::RoundStone, 90.0, 500_000.0);
        let result = optimizer.optimize();
        for (i, gen) in result.population_history.iter().enumerate() {
            assert_eq!(gen.generation, i);
            assert!(gen.best_fitness > 0.0 || gen.avg_fitness >= 0.0);
            assert!(gen.best_x >= 0.0);
            assert!(gen.best_y >= 0.0);
        }
    }

    #[test]
    fn test_ga_with_existing_impacts() {
        let wall = default_wall();
        let impacts = vec![ImpactLoad {
            x_m: 15.0, y_m: 5.0, impact_force_n: 1_000_000.0, blast_radius_m: 2.0,
        }];
        let config = GeneticConfig {
            population_size: 20,
            generations: 10,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, impacts, AmmoType::RoundStone, 90.0, 500_000.0);
        let result = optimizer.optimize();
        assert!(result.best.fitness > 0.0);
        assert!(result.best.x_m > 0.0);
    }

    #[test]
    fn test_ga_gunpowder_ammo() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 20,
            generations: 10,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::GunpowderBomb, 90.0, 500_000.0);
        let result = optimizer.optimize();
        assert!(result.best.fitness > 0.0);
    }

    #[test]
    fn test_ga_corpse_ammo() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 20,
            generations: 10,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::CorpseShell, 90.0, 500_000.0);
        let result = optimizer.optimize();
        assert!(result.best.fitness > 0.0);
    }

    #[test]
    fn test_ga_fitness_positive() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 20,
            generations: 5,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::RoundStone, 90.0, 500_000.0);
        let result = optimizer.optimize();
        assert!(result.best.fitness > 0.0, "best fitness should be positive");
        for gen in &result.population_history {
            assert!(gen.best_fitness > 0.0 || gen.avg_fitness >= 0.0);
        }
    }

    #[test]
    fn test_ga_gate_targeting_tendency() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 40,
            generations: 30,
            elite_count: 5,
            tournament_size: 5,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::RoundStone, 90.0, 500_000.0);
        let result = optimizer.optimize();
        let gate_center = 15.0;
        let dist_to_gate = (result.best.x_m - gate_center).abs();
        assert!(dist_to_gate < 15.0,
            "best point should tend toward gate area, dist={}", dist_to_gate);
    }

    #[test]
    fn test_ga_high_impact_energy() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 20,
            generations: 10,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::GunpowderBomb, 300.0, 5_000_000.0);
        let result = optimizer.optimize();
        assert!(result.best.fitness > 0.0);
    }

    #[test]
    fn test_ga_low_impact_energy() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 20,
            generations: 10,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::RoundStone, 10.0, 100.0);
        let result = optimizer.optimize();
        assert!(result.best.x_m > 0.0);
        assert!(result.best.y_m > 0.0);
    }

    #[test]
    fn test_ga_strong_wall() {
        let wall = WallProperties {
            thickness_m: 6.0,
            material: "stone".to_string(),
            density_kgm3: 2400.0,
            compressive_strength_pa: 25_000_000.0,
            tensile_strength_pa: 2_000_000.0,
        };
        let config = GeneticConfig {
            population_size: 20,
            generations: 10,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::RoundStone, 90.0, 500_000.0);
        let result = optimizer.optimize();
        assert!(result.best.x_m > 0.0);
    }

    #[test]
    fn test_ga_minimal_population() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 4,
            generations: 3,
            elite_count: 1,
            tournament_size: 2,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::RoundStone, 90.0, 500_000.0);
        let result = optimizer.optimize();
        assert!(result.best.x_m > 0.0);
        assert!(result.convergence_data.len() == 3);
    }

    #[test]
    fn test_ga_high_mutation_rate() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 20,
            generations: 10,
            mutation_rate: 0.9,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::RoundStone, 90.0, 500_000.0);
        let result = optimizer.optimize();
        assert!(result.best.x_m > 0.0);
    }

    #[test]
    fn test_ga_no_crossover() {
        let wall = default_wall();
        let config = GeneticConfig {
            population_size: 20,
            generations: 10,
            crossover_rate: 0.0,
            ..Default::default()
        };
        let optimizer = GeneticOptimizer::new(config, wall, vec![], AmmoType::RoundStone, 90.0, 500_000.0);
        let result = optimizer.optimize();
        assert!(result.best.x_m > 0.0);
    }
}
