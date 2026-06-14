CREATE DATABASE IF NOT EXISTS siege_sim;

USE siege_sim;

CREATE TABLE IF NOT EXISTS trebuchets (
    id UInt32,
    name String,
    type String,
    counterweight_kg Float64,
    projectile_kg Float64,
    arm_length_m Float64,
    max_angle_deg Float64,
    created_at DateTime DEFAULT now()
) ENGINE = MergeTree()
ORDER BY id;

CREATE TABLE IF NOT EXISTS sensor_data (
    timestamp DateTime64(3),
    trebuchet_id UInt32,
    cable_tension_newton Float64,
    launch_angle_deg Float64,
    initial_velocity_mps Float64,
    wind_speed_mps Float64,
    wind_direction_deg Float64,
    temperature_c Float64,
    air_density_kgm3 Float64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (trebuchet_id, timestamp)
TTL toDateTime(timestamp) + INTERVAL 30 DAY
    DELETE
    SETTINGS index_granularity = 8192;

CREATE TABLE IF NOT EXISTS ballistics_results (
    timestamp DateTime64(3),
    trebuchet_id UInt32,
    sensor_ts DateTime64(3),
    initial_velocity_mps Float64,
    launch_angle_deg Float64,
    max_height_m Float64,
    range_m Float64,
    flight_time_s Float64,
    impact_velocity_mps Float64,
    impact_kinetic_energy_j Float64,
    trajectory_points Array(Tuple(Float64, Float64, Float64))
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (trebuchet_id, timestamp)
TTL toDateTime(timestamp) + INTERVAL 30 DAY
    DELETE
    SETTINGS index_granularity = 8192;

CREATE TABLE IF NOT EXISTS siege_assessments (
    timestamp DateTime64(3),
    trebuchet_id UInt32,
    wall_thickness_m Float64,
    wall_material String,
    wall_material_density Float64,
    wall_compressive_strength_pa Float64,
    impact_energy_j Float64,
    crater_depth_m Float64,
    crater_diameter_m Float64,
    damage_ratio Float64,
    effectiveness_score Float64,
    optimal_angle_deg Float64,
    optimal_velocity_mps Float64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (trebuchet_id, timestamp)
TTL toDateTime(timestamp) + INTERVAL 90 DAY
    DELETE
    SETTINGS index_granularity = 8192;

INSERT INTO trebuchets (id, name, type, counterweight_kg, projectile_kg, arm_length_m, max_angle_deg) VALUES
(1, '回回炮-甲', '配重式', 3000, 90, 12.0, 50.0),
(2, '回回炮-乙', '配重式', 5000, 150, 15.0, 55.0),
(3, '襄阳砲-壹', '配重式', 4000, 120, 13.5, 52.0),
(4, '人力砲-一号', '人力牵引式', 0, 30, 8.0, 45.0),
(5, '人力砲-二号', '人力牵引式', 0, 25, 7.5, 42.0),
(6, '旋风砲', '人力牵引式', 0, 20, 6.0, 48.0),
(7, '虎蹲砲', '配重式', 1500, 50, 9.0, 47.0),
(8, '无敌砲', '配重式', 6000, 200, 18.0, 58.0),
(9, '飞云砲', '人力牵引式', 0, 15, 5.5, 40.0),
(10, '震天雷砲', '配重式', 8000, 300, 20.0, 60.0);

CREATE TABLE IF NOT EXISTS wall_types (
    id UInt32,
    name String,
    material String,
    thickness_m Float64,
    density_kgm3 Float64,
    compressive_strength_pa Float64,
    tensile_strength_pa Float64
) ENGINE = MergeTree()
ORDER BY id;

INSERT INTO wall_types (id, name, material, thickness_m, density_kgm3, compressive_strength_pa, tensile_strength_pa) VALUES
(1, '夯土墙', 'rammed_earth', 3.0, 1800, 2000000, 200000),
(2, '包砖墙', 'brick_veneer', 2.5, 2000, 10000000, 800000),
(3, '石砌墙', 'stone_masonry', 4.0, 2400, 25000000, 2000000),
(4, '双层夯土墙', 'double_rammed_earth', 6.0, 1700, 1800000, 180000),
(5, '糯米灰浆墙', 'sticky_rice_lime', 3.5, 2100, 15000000, 1200000);

CREATE TABLE IF NOT EXISTS siege_assessments_monthly_mv (
    month Date,
    trebuchet_id UInt32,
    wall_material String,
    avg_effectiveness_score Float64,
    max_effectiveness_score Float64,
    min_effectiveness_score Float64,
    total_assessments UInt64,
    avg_impact_energy_j Float64
) ENGINE = SummingMergeTree()
ORDER BY (month, trebuchet_id, wall_material);

CREATE MATERIALIZED VIEW IF NOT EXISTS siege_assessments_monthly_mv_mv
TO siege_assessments_monthly_mv
AS
SELECT
    toDate(timestamp) AS month,
    trebuchet_id,
    wall_material,
    avg(effectiveness_score) AS avg_effectiveness_score,
    max(effectiveness_score) AS max_effectiveness_score,
    min(effectiveness_score) AS min_effectiveness_score,
    count() AS total_assessments,
    avg(impact_energy_j) AS avg_impact_energy_j
FROM siege_assessments
GROUP BY month, trebuchet_id, wall_material;

CREATE TABLE IF NOT EXISTS ammo_comparisons (
    timestamp DateTime64(3),
    velocity_mps Float64,
    angle_deg Float64,
    mass_kg Float64,
    round_stone_range_m Float64,
    round_stone_impact_energy_j Float64,
    gunpowder_bomb_range_m Float64,
    gunpowder_bomb_impact_energy_j Float64,
    gunpowder_bomb_explosive_energy_j Float64,
    corpse_shell_range_m Float64,
    corpse_shell_impact_energy_j Float64,
    corpse_shell_contamination_radius_m Float64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY timestamp
TTL toDateTime(timestamp) + INTERVAL 30 DAY
    DELETE
    SETTINGS index_granularity = 8192;

CREATE TABLE IF NOT EXISTS fea_analyses (
    timestamp DateTime64(3),
    wall_thickness_m Float64,
    wall_material String,
    wall_density_kgm3 Float64,
    wall_compressive_strength_pa Float64,
    max_stress_pa Float64,
    min_safety_factor Float64,
    weak_point_count UInt32,
    best_weak_point_x Float64,
    best_weak_point_y Float64,
    best_weak_point_safety_factor Float64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY timestamp
TTL toDateTime(timestamp) + INTERVAL 90 DAY
    DELETE
    SETTINGS index_granularity = 8192;

CREATE TABLE IF NOT EXISTS ga_optimizations (
    timestamp DateTime64(3),
    wall_thickness_m Float64,
    ammo_type String,
    projectile_mass_kg Float64,
    impact_energy_j Float64,
    best_x_m Float64,
    best_y_m Float64,
    best_fitness Float64,
    generations UInt32,
    population_size UInt32
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY timestamp
TTL toDateTime(timestamp) + INTERVAL 90 DAY
    DELETE
    SETTINGS index_granularity = 8192;

CREATE TABLE IF NOT EXISTS coordination_results (
    timestamp DateTime64(3),
    wall_thickness_m Float64,
    assignment_count UInt32,
    expected_total_damage Float64,
    coordination_efficiency Float64,
    q_table_size UInt64,
    episodes_trained UInt32
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY timestamp
TTL toDateTime(timestamp) + INTERVAL 90 DAY
    DELETE
    SETTINGS index_granularity = 8192;

CREATE TABLE IF NOT EXISTS battle_impacts (
    timestamp DateTime64(3),
    scenario_id UInt32,
    battle_day UInt32,
    trebuchet_id UInt32,
    target_x_m Float64,
    target_y_m Float64,
    ammo_type String,
    damage_ratio Float64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (scenario_id, battle_day, timestamp)
TTL toDateTime(timestamp) + INTERVAL 180 DAY
    DELETE
    SETTINGS index_granularity = 8192;
