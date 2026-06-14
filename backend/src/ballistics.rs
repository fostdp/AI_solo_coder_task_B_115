use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallisticInput {
    pub initial_velocity: f64,
    pub launch_angle_deg: f64,
    pub projectile_mass_kg: f64,
    pub projectile_diameter_m: f64,
    pub air_density_kgm3: f64,
    pub wind_speed_mps: f64,
    pub wind_direction_deg: f64,
    pub launch_height_m: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TrajectoryPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub velocity: f64,
    pub time_s: f64,
    pub mach_number: f64,
    pub drag_coefficient: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallisticResult {
    pub max_height_m: f64,
    pub range_m: f64,
    pub flight_time_s: f64,
    pub impact_velocity_mps: f64,
    pub impact_kinetic_energy_j: f64,
    pub trajectory: Vec<TrajectoryPoint>,
    pub impact_angle_deg: f64,
    pub max_mach_number: f64,
    pub avg_drag_coefficient: f64,
    pub solver_steps: u32,
    pub adaptive_steps: u32,
}

const GRAVITY: f64 = 9.81;

const INITIAL_DT: f64 = 0.0001;
const MIN_DT: f64 = 1.0e-7;
const MAX_DT: f64 = 0.05;
const ERROR_TOLERANCE: f64 = 1.0e-6;

#[derive(Debug, Clone, Copy)]
struct State {
    x: f64,
    y: f64,
    z: f64,
    vx: f64,
    vy: f64,
    vz: f64,
}

impl State {
    fn add_deriv(&self, d: &Derivative, scale: f64) -> State {
        State {
            x: self.x + d.x * scale,
            y: self.y + d.y * scale,
            z: self.z + d.z * scale,
            vx: self.vx + d.vx * scale,
            vy: self.vy + d.vy * scale,
            vz: self.vz + d.vz * scale,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Derivative {
    x: f64,
    y: f64,
    z: f64,
    vx: f64,
    vy: f64,
    vz: f64,
}

fn reynolds_drag(re: f64) -> f64 {
    if re < 1.0 {
        return 24.0 / re.max(1.0);
    }
    let cd_laminar = 24.0 / re * (1.0 + 3.0 / 16.0 * re);
    let cd_turbulent = 0.47;
    let transition = 1.0 / (1.0 + (-0.003 * (re - 1000.0)).exp());
    cd_laminar * (1.0 - transition) + cd_turbulent * transition
}

fn compressible_drag_coefficient(mach: f64, reynolds: f64) -> f64 {
    let cd_incompressible = reynolds_drag(reynolds);

    if mach < 0.3 {
        return cd_incompressible;
    }

    let mach_sq = mach * mach;
    let prandtl_glauert_factor = 1.0 / (1.0 - mach_sq).sqrt().max(0.1);
    let mut cd_compressible = cd_incompressible * prandtl_glauert_factor;

    if mach > 0.6 {
        let denom = (1.0 - mach_sq).max(0.01);
        let wave_drag = 0.005 * ((mach - 0.6) / denom).powi(2);
        cd_compressible += wave_drag.min(2.0);
    }

    cd_compressible.min(2.5)
}

fn reynolds_number(velocity_mag: f64, diameter: f64, air_density: f64, dynamic_viscosity: f64) -> f64 {
    (air_density * velocity_mag * diameter) / dynamic_viscosity.max(1.0e-10)
}

fn sutherland_viscosity(temperature_c: f64) -> f64 {
    let t_k = temperature_c + 273.15;
    let t0 = 291.15;
    let mu0 = 1.827e-5;
    let s = 120.0;
    mu0 * (t_k / t0).powf(1.5) * (t0 + s) / (t_k + s)
}

fn air_temperature_from_density(air_density: f64) -> f64 {
    let p0 = 101325.0;
    let r_specific = 287.058;
    (p0 / (r_specific * air_density.max(0.01))) - 273.15
}

fn compute_flow_state(
    state: &State,
    input: &BallisticInput,
    wind_x: f64,
    wind_z: f64,
    diameter: f64,
) -> (f64, f64, f64) {
    let rel_vx = state.vx - wind_x;
    let rel_vy = state.vy;
    let rel_vz = state.vz - wind_z;
    let rel_vel_mag = (rel_vx * rel_vx + rel_vy * rel_vy + rel_vz * rel_vz).sqrt();

    let temp_c = air_temperature_from_density(input.air_density_kgm3);
    let temp_k = temp_c + 273.15;
    let speed_of_sound = (1.4 * 287.058 * temp_k).sqrt();
    let mach_number = rel_vel_mag / speed_of_sound.max(0.001);

    let dynamic_viscosity = sutherland_viscosity(temp_c.max(-50.0));
    let reynolds = reynolds_number(rel_vel_mag, diameter, input.air_density_kgm3, dynamic_viscosity);
    let cd = compressible_drag_coefficient(mach_number, reynolds);

    (rel_vel_mag, mach_number, cd)
}

fn compute_derivatives(
    state: &State,
    input: &BallisticInput,
    wind_x: f64,
    wind_z: f64,
    cross_section: f64,
    diameter: f64,
) -> Derivative {
    compute_derivatives_internal(state, input, wind_x, wind_z, cross_section, diameter, 1.0)
}

fn compute_derivatives_internal(
    state: &State,
    input: &BallisticInput,
    wind_x: f64,
    wind_z: f64,
    cross_section: f64,
    diameter: f64,
    drag_modifier: f64,
) -> Derivative {
    let rel_vx = state.vx - wind_x;
    let rel_vy = state.vy;
    let rel_vz = state.vz - wind_z;
    let (rel_vel_mag, _, cd) = compute_flow_state(state, input, wind_x, wind_z, diameter);

    let drag_mag = if rel_vel_mag > 0.001 {
        0.5 * input.air_density_kgm3 * rel_vel_mag * rel_vel_mag * cd * drag_modifier * cross_section / input.projectile_mass_kg
    } else {
        0.0
    };

    let inv_rel = 1.0 / rel_vel_mag.max(0.001);
    let ax = -drag_mag * rel_vx * inv_rel;
    let ay = -drag_mag * rel_vy * inv_rel - GRAVITY;
    let az = -drag_mag * rel_vz * inv_rel;

    Derivative {
        x: state.vx,
        y: state.vy,
        z: state.vz,
        vx: ax,
        vy: ay,
        vz: az,
    }
}

fn rk4_step(
    state: &State,
    input: &BallisticInput,
    wind_x: f64,
    wind_z: f64,
    cross_section: f64,
    diameter: f64,
    dt: f64,
) -> (State, f64, f64, f64) {
    let k1 = compute_derivatives(state, input, wind_x, wind_z, cross_section, diameter);

    let s2 = state.add_deriv(&k1, dt * 0.5);
    let k2 = compute_derivatives(&s2, input, wind_x, wind_z, cross_section, diameter);

    let s3 = state.add_deriv(&k2, dt * 0.5);
    let k3 = compute_derivatives(&s3, input, wind_x, wind_z, cross_section, diameter);

    let s4 = state.add_deriv(&k3, dt);
    let k4 = compute_derivatives(&s4, input, wind_x, wind_z, cross_section, diameter);

    let new_state = State {
        x: state.x + dt / 6.0 * (k1.x + 2.0 * k2.x + 2.0 * k3.x + k4.x),
        y: state.y + dt / 6.0 * (k1.y + 2.0 * k2.y + 2.0 * k3.y + k4.y),
        z: state.z + dt / 6.0 * (k1.z + 2.0 * k2.z + 2.0 * k3.z + k4.z),
        vx: state.vx + dt / 6.0 * (k1.vx + 2.0 * k2.vx + 2.0 * k3.vx + k4.vx),
        vy: state.vy + dt / 6.0 * (k1.vy + 2.0 * k2.vy + 2.0 * k3.vy + k4.vy),
        vz: state.vz + dt / 6.0 * (k1.vz + 2.0 * k2.vz + 2.0 * k3.vz + k4.vz),
    };

    let s_half_step = State {
        x: state.x + dt / 2.0 * (k1.x + k2.x + k3.x) / 3.0,
        y: state.y + dt / 2.0 * (k1.y + k2.y + k3.y) / 3.0,
        z: state.z + dt / 2.0 * (k1.z + k2.z + k3.z) / 3.0,
        vx: state.vx + dt / 2.0 * (k1.vx + k2.vx + k3.vx) / 3.0,
        vy: state.vy + dt / 2.0 * (k1.vy + k2.vy + k3.vy) / 3.0,
        vz: state.vz + dt / 2.0 * (k1.vz + k2.vz + k3.vz) / 3.0,
    };

    let dx = (new_state.x - s_half_step.x).abs();
    let dy = (new_state.y - s_half_step.y).abs();
    let dz = (new_state.z - s_half_step.z).abs();
    let dvx = (new_state.vx - s_half_step.vx).abs();
    let dvy = (new_state.vy - s_half_step.vy).abs();
    let dvz = (new_state.vz - s_half_step.vz).abs();

    let err_pos = (dx * dx + dy * dy + dz * dz).sqrt();
    let err_vel = (dvx * dvx + dvy * dvy + dvz * dvz).sqrt();
    let error = err_pos.max(err_vel);

    let (_, mach, cd) = compute_flow_state(&new_state, input, wind_x, wind_z, diameter);

    (new_state, error, mach, cd)
}

fn rk4_step_with_drag(
    state: &State,
    input: &BallisticInput,
    wind_x: f64,
    wind_z: f64,
    cross_section: f64,
    diameter: f64,
    dt: f64,
    drag_modifier: f64,
) -> (State, f64, f64, f64) {
    let k1 = compute_derivatives_internal(state, input, wind_x, wind_z, cross_section, diameter, drag_modifier);

    let s2 = state.add_deriv(&k1, dt * 0.5);
    let k2 = compute_derivatives_internal(&s2, input, wind_x, wind_z, cross_section, diameter, drag_modifier);

    let s3 = state.add_deriv(&k2, dt * 0.5);
    let k3 = compute_derivatives_internal(&s3, input, wind_x, wind_z, cross_section, diameter, drag_modifier);

    let s4 = state.add_deriv(&k3, dt);
    let k4 = compute_derivatives_internal(&s4, input, wind_x, wind_z, cross_section, diameter, drag_modifier);

    let new_state = State {
        x: state.x + dt / 6.0 * (k1.x + 2.0 * k2.x + 2.0 * k3.x + k4.x),
        y: state.y + dt / 6.0 * (k1.y + 2.0 * k2.y + 2.0 * k3.y + k4.y),
        z: state.z + dt / 6.0 * (k1.z + 2.0 * k2.z + 2.0 * k3.z + k4.z),
        vx: state.vx + dt / 6.0 * (k1.vx + 2.0 * k2.vx + 2.0 * k3.vx + k4.vx),
        vy: state.vy + dt / 6.0 * (k1.vy + 2.0 * k2.vy + 2.0 * k3.vy + k4.vy),
        vz: state.vz + dt / 6.0 * (k1.vz + 2.0 * k2.vz + 2.0 * k3.vz + k4.vz),
    };

    let s_half_step = State {
        x: state.x + dt / 2.0 * (k1.x + k2.x + k3.x) / 3.0,
        y: state.y + dt / 2.0 * (k1.y + k2.y + k3.y) / 3.0,
        z: state.z + dt / 2.0 * (k1.z + k2.z + k3.z) / 3.0,
        vx: state.vx + dt / 2.0 * (k1.vx + k2.vx + k3.vx) / 3.0,
        vy: state.vy + dt / 2.0 * (k1.vy + k2.vy + k3.vy) / 3.0,
        vz: state.vz + dt / 2.0 * (k1.vz + k2.vz + k3.vz) / 3.0,
    };

    let dx = (new_state.x - s_half_step.x).abs();
    let dy = (new_state.y - s_half_step.y).abs();
    let dz = (new_state.z - s_half_step.z).abs();
    let dvx = (new_state.vx - s_half_step.vx).abs();
    let dvy = (new_state.vy - s_half_step.vy).abs();
    let dvz = (new_state.vz - s_half_step.vz).abs();

    let err_pos = (dx * dx + dy * dy + dz * dz).sqrt();
    let err_vel = (dvx * dvx + dvy * dvy + dvz * dvz).sqrt();
    let error = err_pos.max(err_vel);

    let (_, mach, cd) = compute_flow_state(&new_state, input, wind_x, wind_z, diameter);

    (new_state, error, mach, cd)
}

pub fn simulate_ballistics(input: &BallisticInput) -> BallisticResult {
    simulate_ballistics_internal(
        input.initial_velocity,
        input.launch_angle_deg,
        input.projectile_mass_kg,
        input.projectile_diameter_m,
        input.air_density_kgm3,
        input.wind_speed_mps,
        input.wind_direction_deg,
        input.launch_height_m,
        None,
    )
}

pub fn simulate_ballistics_with_ammo(
    initial_velocity: f64,
    launch_angle_deg: f64,
    projectile_mass_kg: f64,
    air_density_kgm3: f64,
    wind_speed_mps: f64,
    wind_direction_deg: f64,
    launch_height_m: f64,
    ammo_profile: &crate::ammo::AmmoProfile,
) -> BallisticResult {
    let effective_diameter = ammo_profile.effective_diameter(projectile_mass_kg);
    let input = BallisticInput {
        initial_velocity,
        launch_angle_deg,
        projectile_mass_kg,
        projectile_diameter_m: effective_diameter,
        air_density_kgm3,
        wind_speed_mps,
        wind_direction_deg,
        launch_height_m,
    };
    simulate_ballistics_internal(
        initial_velocity,
        launch_angle_deg,
        projectile_mass_kg,
        effective_diameter,
        air_density_kgm3,
        wind_speed_mps,
        wind_direction_deg,
        launch_height_m,
        Some(ammo_profile.drag_modifier),
    )
}

fn simulate_ballistics_internal(
    initial_velocity: f64,
    launch_angle_deg: f64,
    projectile_mass_kg: f64,
    projectile_diameter_m: f64,
    air_density_kgm3: f64,
    wind_speed_mps: f64,
    wind_direction_deg: f64,
    launch_height_m: f64,
    drag_modifier: Option<f64>,
) -> BallisticResult {
    let angle_rad = launch_angle_deg.to_radians();
    let wind_rad = wind_direction_deg.to_radians();

    let wind_x = wind_speed_mps * wind_rad.cos();
    let wind_z = wind_speed_mps * wind_rad.sin();

    let radius = projectile_diameter_m / 2.0;
    let cross_section = std::f64::consts::PI * radius * radius;
    let diameter = projectile_diameter_m;
    let drag_mod = drag_modifier.unwrap_or(1.0);

    let mut state = State {
        x: 0.0,
        y: launch_height_m,
        z: 0.0,
        vx: initial_velocity * angle_rad.cos(),
        vy: initial_velocity * angle_rad.sin(),
        vz: 0.0,
    };

    let mut t = 0.0;
    let mut dt = INITIAL_DT;

    let mut trajectory = Vec::new();
    let mut max_height = launch_height_m;
    let mut max_mach = 0.0;
    let mut cd_sum = 0.0;
    let mut cd_count = 0;
    let mut steps = 0u32;
    let mut adaptive_adjustments = 0u32;
    let mut impact_angle = 45.0;

    let initial_vel_mag = (state.vx * state.vx + state.vy * state.vy + state.vz * state.vz).sqrt();

    let fake_input = BallisticInput {
        initial_velocity,
        launch_angle_deg,
        projectile_mass_kg,
        projectile_diameter_m,
        air_density_kgm3,
        wind_speed_mps,
        wind_direction_deg,
        launch_height_m,
    };
    let (_, initial_mach, initial_cd) = compute_flow_state(&state, &fake_input, wind_x, wind_z, diameter);

    trajectory.push(TrajectoryPoint {
        x: state.x,
        y: state.y,
        z: state.z,
        velocity: initial_vel_mag,
        time_s: 0.0,
        mach_number: initial_mach,
        drag_coefficient: initial_cd,
    });
    max_mach = initial_mach;
    cd_sum += initial_cd;
    cd_count += 1;

    let record_interval = 0.01;
    let mut last_record_t = 0.0;
    let mut prev_point: Option<TrajectoryPoint> = None;

    while state.y >= -1.0 {
        let (new_state, error, mach, cd) = rk4_step_with_drag(
            &state, &fake_input, wind_x, wind_z, cross_section, diameter, dt, drag_mod
        );

        let dt_new = if error > ERROR_TOLERANCE * 10.0 && dt > MIN_DT {
            adaptive_adjustments += 1;
            (dt * 0.5).max(MIN_DT)
        } else if error < ERROR_TOLERANCE * 0.1 && dt < MAX_DT {
            adaptive_adjustments += 1;
            (dt * 1.5).min(MAX_DT)
        } else {
            dt
        };

        if error <= ERROR_TOLERANCE || dt <= MIN_DT {
            prev_point = Some(TrajectoryPoint {
                x: state.x,
                y: state.y,
                z: state.z,
                velocity: (state.vx * state.vx + state.vy * state.vy + state.vz * state.vz).sqrt(),
                time_s: t,
                mach_number: mach,
                drag_coefficient: cd,
            });

            state = new_state;
            t += dt;
            steps += 1;

            if state.y > max_height {
                max_height = state.y;
            }

            if mach > max_mach {
                max_mach = mach;
            }

            cd_sum += cd;
            cd_count += 1;

            if t - last_record_t >= record_interval {
                let vel_mag = (state.vx * state.vx + state.vy * state.vy + state.vz * state.vz).sqrt();
                trajectory.push(TrajectoryPoint {
                    x: state.x,
                    y: state.y,
                    z: state.z,
                    velocity: vel_mag,
                    time_s: t,
                    mach_number: mach,
                    drag_coefficient: cd,
                });
                last_record_t = t;
            }
        } else {
            steps += 1;
        }

        dt = dt_new;

        if t > 120.0 {
            break;
        }
    }

    if let Some(prev) = prev_point {
        impact_angle = if (state.x - prev.x).abs() > 0.001 {
            ((state.y - prev.y) / (state.x - prev.x)).atan().to_degrees().abs()
        } else {
            90.0
        };
    }

    let vel_mag = (state.vx * state.vx + state.vy * state.vy + state.vz * state.vz).sqrt();
    let range = (state.x * state.x + state.z * state.z).sqrt();
    let impact_kinetic_energy = 0.5 * projectile_mass_kg * vel_mag * vel_mag;
    let avg_cd = if cd_count > 0 { cd_sum / cd_count as f64 } else { 0.47 };

    BallisticResult {
        max_height_m: max_height,
        range_m: range,
        flight_time_s: t,
        impact_velocity_mps: vel_mag,
        impact_kinetic_energy_j: impact_kinetic_energy,
        trajectory,
        impact_angle_deg: impact_angle,
        max_mach_number: max_mach,
        avg_drag_coefficient: avg_cd,
        solver_steps: steps,
        adaptive_steps: adaptive_adjustments,
    }
}

pub fn estimate_projectile_diameter(mass_kg: f64, density_kgm3: f64) -> f64 {
    let volume = mass_kg / density_kgm3.max(1.0);
    let radius = (3.0 * volume / (4.0 * std::f64::consts::PI)).powf(1.0 / 3.0);
    2.0 * radius
}

pub fn stone_density() -> f64 {
    2600.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_ballistics() {
        let input = BallisticInput {
            initial_velocity: 50.0,
            launch_angle_deg: 45.0,
            projectile_mass_kg: 90.0,
            projectile_diameter_m: 0.4,
            air_density_kgm3: 1.225,
            wind_speed_mps: 0.0,
            wind_direction_deg: 0.0,
            launch_height_m: 5.0,
        };

        let result = simulate_ballistics(&input);
        assert!(result.range_m > 100.0);
        assert!(result.max_height_m > 50.0);
        assert!(result.flight_time_s > 5.0);
        assert!(result.impact_kinetic_energy_j > 1000.0);
        assert!(result.solver_steps > 0);
    }

    #[test]
    fn test_subsonic_compressibility() {
        let input = BallisticInput {
            initial_velocity: 300.0,
            launch_angle_deg: 45.0,
            projectile_mass_kg: 300.0,
            projectile_diameter_m: 0.6,
            air_density_kgm3: 1.225,
            wind_speed_mps: 0.0,
            wind_direction_deg: 0.0,
            launch_height_m: 8.0,
        };

        let result = simulate_ballistics(&input);
        assert!(result.max_mach_number > 0.3);
    }

    #[test]
    fn test_diameter_calculation() {
        let d = estimate_projectile_diameter(90.0, 2600.0);
        assert!(d > 0.3 && d < 0.5);
    }
}
