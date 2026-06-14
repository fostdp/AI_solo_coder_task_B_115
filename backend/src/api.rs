use crate::ballistics::{
    estimate_projectile_diameter, simulate_ballistics, simulate_ballistics_with_ammo, BallisticInput, stone_density,
};
use crate::siege::{assess_siege_damage, optimize_launch_parameters, SiegeInput, WallProperties};
use crate::ammo::{self, AmmoType, AmmoProfile, AmmoComparison};
use crate::fea::{FEAnalyzer, FEAResult, ImpactLoad};
use crate::genetic::{GeneticOptimizer, GeneticConfig, GAResult};
use crate::coordinator::{SiegeCoordinator, CoordinatorConfig, TrebuchetState, WallRegionState, CoordinationResult};
use crate::battles::{self, BattleScenario, BattleState};
use crate::storage::Database;
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub latest_results: Arc<Mutex<HashMap<u32, crate::ballistics::BallisticResult>>>,
    pub latest_siege: Arc<Mutex<HashMap<u32, crate::siege::SiegeAssessment>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BallisticCalcQuery {
    pub velocity: f64,
    pub angle: f64,
    pub mass: f64,
    pub wind_speed: Option<f64>,
    pub wind_direction: Option<f64>,
    pub air_density: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct SiegeCalcQuery {
    pub impact_energy: f64,
    pub projectile_mass: f64,
    pub projectile_diameter: f64,
    pub impact_angle: f64,
    pub wall_thickness: f64,
    pub wall_material: Option<String>,
    pub wall_density: Option<f64>,
    pub wall_compressive_strength: Option<f64>,
    pub wall_tensile_strength: Option<f64>,
    pub ammo_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AmmoCompareQuery {
    pub velocity: f64,
    pub angle: f64,
    pub mass: f64,
    pub air_density: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct FEAQuery {
    pub wall_thickness: f64,
    pub wall_density: Option<f64>,
    pub wall_compressive_strength: Option<f64>,
    pub wall_tensile_strength: Option<f64>,
    pub wall_width: Option<f64>,
    pub wall_height: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct GAQuery {
    pub projectile_mass: f64,
    pub impact_energy: f64,
    pub wall_thickness: f64,
    pub wall_density: Option<f64>,
    pub wall_compressive_strength: Option<f64>,
    pub wall_tensile_strength: Option<f64>,
    pub ammo_type: Option<String>,
    pub population_size: Option<usize>,
    pub generations: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct CoordinateQuery {
    pub trebuchet_ids: Option<String>,
    pub wall_thickness: f64,
    pub wall_density: Option<f64>,
    pub wall_compressive_strength: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct BattleFireQuery {
    pub trebuchet_id: u32,
    pub target_x: f64,
    pub target_y: f64,
    pub ammo_type: Option<String>,
    pub damage_ratio: f64,
}

#[derive(Debug, Deserialize)]
pub struct OptimizeQuery {
    pub projectile_mass: f64,
    pub projectile_diameter: Option<f64>,
    pub wall_thickness: f64,
    pub wall_material: Option<String>,
    pub wall_density: Option<f64>,
    pub wall_compressive_strength: Option<f64>,
    pub wall_tensile_strength: Option<f64>,
    pub min_velocity: Option<f64>,
    pub max_velocity: Option<f64>,
    pub min_angle: Option<f64>,
    pub max_angle: Option<f64>,
}

fn success_response<T: Serialize>(data: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse {
        success: true,
        data: Some(data),
        error: None,
    })
}

fn error_response<T: Serialize>(msg: &str) -> (StatusCode, Json<ApiResponse<T>>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        }),
    )
}

async fn get_trebuchets(State(state): State<AppState>) -> Json<ApiResponse<Vec<crate::storage::TrebuchetInfo>>> {
    let trebuchets = state.db.get_trebuchets().await;
    success_response(trebuchets)
}

async fn get_trebuchet(
    Path(id): Path<u32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<crate::storage::TrebuchetInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.db.get_trebuchet_by_id(id).await {
        Some(t) => Ok(success_response(t)),
        None => Err(error_response("Trebuchet not found")),
    }
}

async fn get_wall_types(State(state): State<AppState>) -> Json<ApiResponse<Vec<crate::storage::WallType>>> {
    let walls = state.db.get_wall_types().await;
    success_response(walls)
}

async fn get_latest_ballistics(
    Path(id): Path<u32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<crate::ballistics::BallisticResult>>, (StatusCode, Json<ApiResponse<()>>)> {
    let results = state.latest_results.lock().await;
    match results.get(&id) {
        Some(r) => Ok(success_response(r.clone())),
        None => Err(error_response("No ballistics data for this trebuchet")),
    }
}

async fn get_latest_siege(
    Path(id): Path<u32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<crate::siege::SiegeAssessment>>, (StatusCode, Json<ApiResponse<()>>)> {
    let siege = state.latest_siege.lock().await;
    match siege.get(&id) {
        Some(r) => Ok(success_response(r.clone())),
        None => Err(error_response("No siege assessment for this trebuchet")),
    }
}

async fn get_sensor_history(
    Path(id): Path<u32>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<crate::storage::SensorData>>> {
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(100);
    let data = state.db.get_recent_sensor_data(id, limit).await;
    success_response(data)
}

async fn calculate_ballistics(
    Query(query): Query<BallisticCalcQuery>,
) -> Json<ApiResponse<crate::ballistics::BallisticResult>> {
    let diameter = estimate_projectile_diameter(query.mass, stone_density());

    let input = BallisticInput {
        initial_velocity: query.velocity,
        launch_angle_deg: query.angle,
        projectile_mass_kg: query.mass,
        projectile_diameter_m: diameter,
        air_density_kgm3: query.air_density.unwrap_or(1.225),
        wind_speed_mps: query.wind_speed.unwrap_or(0.0),
        wind_direction_deg: query.wind_direction.unwrap_or(0.0),
        launch_height_m: 5.0,
    };

    let result = simulate_ballistics(&input);
    success_response(result)
}

async fn calculate_siege(
    Query(query): Query<SiegeCalcQuery>,
) -> Json<ApiResponse<crate::siege::SiegeAssessment>> {
    let ammo_type = parse_ammo_type(query.ammo_type.as_deref());
    let wall = WallProperties {
        thickness_m: query.wall_thickness,
        material: query.wall_material.unwrap_or_else(|| "rammed_earth".to_string()),
        density_kgm3: query.wall_density.unwrap_or(1800.0),
        compressive_strength_pa: query.wall_compressive_strength.unwrap_or(2_000_000.0),
        tensile_strength_pa: query.wall_tensile_strength.unwrap_or(200_000.0),
    };

    let input = SiegeInput {
        impact_energy_j: query.impact_energy,
        projectile_mass_kg: query.projectile_mass,
        projectile_diameter_m: query.projectile_diameter,
        impact_angle_deg: query.impact_angle,
        wall,
        ammo_type,
    };

    let result = assess_siege_damage(&input);
    success_response(result)
}

fn parse_ammo_type(s: Option<&str>) -> AmmoType {
    match s.unwrap_or("round_stone") {
        "gunpowder_bomb" | "GunpowderBomb" => AmmoType::GunpowderBomb,
        "corpse_shell" | "CorpseShell" => AmmoType::CorpseShell,
        _ => AmmoType::RoundStone,
    }
}

async fn compare_ammo_endpoint(
    Query(query): Query<AmmoCompareQuery>,
) -> Json<ApiResponse<AmmoComparison>> {
    let comparison = ammo::compare_ammo(
        query.velocity,
        query.angle,
        query.mass,
        query.air_density.unwrap_or(1.225),
    );
    success_response(comparison)
}

async fn analyze_wall_stress(
    Query(query): Query<FEAQuery>,
) -> Json<ApiResponse<FEAResult>> {
    let analyzer = FEAnalyzer::new(
        query.wall_width.unwrap_or(30.0),
        query.wall_height.unwrap_or(10.0),
        query.wall_thickness,
        query.wall_density.unwrap_or(1800.0),
        query.wall_compressive_strength.unwrap_or(2_000_000.0),
        query.wall_tensile_strength.unwrap_or(200_000.0),
    );
    let result = analyzer.analyze(&[]);
    success_response(result)
}

async fn find_weak_point(
    Query(query): Query<GAQuery>,
) -> Json<ApiResponse<GAResult>> {
    let ammo_type = parse_ammo_type(query.ammo_type.as_deref());
    let wall = WallProperties {
        thickness_m: query.wall_thickness,
        material: "rammed_earth".to_string(),
        density_kgm3: query.wall_density.unwrap_or(1800.0),
        compressive_strength_pa: query.wall_compressive_strength.unwrap_or(2_000_000.0),
        tensile_strength_pa: 200_000.0,
    };
    let ga_config = GeneticConfig {
        population_size: query.population_size.unwrap_or(60),
        generations: query.generations.unwrap_or(50),
        ..Default::default()
    };
    let optimizer = GeneticOptimizer::new(
        ga_config,
        wall,
        vec![],
        ammo_type,
        query.projectile_mass,
        query.impact_energy,
    );
    let result = optimizer.optimize();
    success_response(result)
}

async fn coordinate_fire(
    Query(query): Query<CoordinateQuery>,
    State(state): State<AppState>,
) -> Json<ApiResponse<CoordinationResult>> {
    let wall = WallProperties {
        thickness_m: query.wall_thickness,
        material: "rammed_earth".to_string(),
        density_kgm3: query.wall_density.unwrap_or(1800.0),
        compressive_strength_pa: query.wall_compressive_strength.unwrap_or(2_000_000.0),
        tensile_strength_pa: 200_000.0,
    };

    let all_trebuchets = state.db.get_trebuchets().await;
    let trebuchets: Vec<TrebuchetState> = if let Some(ids_str) = query.trebuchet_ids {
        let ids: Vec<u32> = ids_str.split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        all_trebuchets.iter()
            .filter(|t| ids.contains(&t.id))
            .map(|t| TrebuchetState {
                id: t.id,
                ammo_type: AmmoType::RoundStone,
                range_m: 200.0,
                reload_time_s: 60.0,
                ready: true,
                assigned_target: None,
            })
            .collect()
    } else {
        all_trebuchets.iter().map(|t| TrebuchetState {
            id: t.id,
            ammo_type: AmmoType::RoundStone,
            range_m: 200.0,
            reload_time_s: 60.0,
            ready: true,
            assigned_target: None,
        }).collect()
    };

    let regions = vec![WallRegionState {
        x_m: 15.0,
        y_m: 5.0,
        width_m: 10.0,
        height_m: 5.0,
        damage_ratio: 0.0,
        stress_ratio: 0.5,
        strategic_value: 1.0,
    }];

    let config = CoordinatorConfig::default();
    let mut coordinator = SiegeCoordinator::new(config, wall);
    let result = coordinator.coordinate(&trebuchets, &regions, &[]);
    success_response(result)
}

async fn list_battles() -> Json<ApiResponse<Vec<BattleScenario>>> {
    let battles = battles::get_historical_battles();
    success_response(battles)
}

async fn get_battle(
    Path(id): Path<u32>,
) -> Result<Json<ApiResponse<BattleScenario>>, (StatusCode, Json<ApiResponse<()>>)> {
    match battles::get_battle_by_id(id) {
        Some(b) => Ok(success_response(b)),
        None => Err(error_response("Battle not found")),
    }
}

async fn start_battle(
    Path(id): Path<u32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<BattleState>>, (StatusCode, Json<ApiResponse<()>>)> {
    match battles::get_battle_by_id(id) {
        Some(scenario) => {
            let battle_state = BattleState::new(&scenario);
            state.db.set_battle_state(id, battle_state.clone()).await;
            Ok(success_response(battle_state))
        }
        None => Err(error_response("Battle not found")),
    }
}

async fn battle_fire(
    Path(id): Path<u32>,
    Query(query): Query<BattleFireQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<BattleState>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut battle_state = state.db.get_battle_state(id).await
        .ok_or_else(|| error_response("Battle not started"))?;

    let ammo_type = parse_ammo_type(query.ammo_type.as_deref());
    battle_state.record_impact(
        query.trebuchet_id,
        query.target_x,
        query.target_y,
        ammo_type,
        query.damage_ratio,
    );
    state.db.set_battle_state(id, battle_state.clone()).await;
    Ok(success_response(battle_state))
}

async fn battle_advance(
    Path(id): Path<u32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<BattleState>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut battle_state = state.db.get_battle_state(id).await
        .ok_or_else(|| error_response("Battle not started"))?;
    battle_state.advance_day();
    state.db.set_battle_state(id, battle_state.clone()).await;
    Ok(success_response(battle_state))
}

#[derive(Debug, Serialize)]
struct OptimizeResult {
    optimal_angle_deg: f64,
    optimal_velocity_mps: f64,
    max_effectiveness_score: f64,
}

async fn optimize_parameters(
    Query(query): Query<OptimizeQuery>,
) -> Json<ApiResponse<OptimizeResult>> {
    let wall = WallProperties {
        thickness_m: query.wall_thickness,
        material: query.wall_material.unwrap_or_else(|| "rammed_earth".to_string()),
        density_kgm3: query.wall_density.unwrap_or(1800.0),
        compressive_strength_pa: query.wall_compressive_strength.unwrap_or(2_000_000.0),
        tensile_strength_pa: query.wall_tensile_strength.unwrap_or(200_000.0),
    };

    let diameter = query
        .projectile_diameter
        .unwrap_or_else(|| estimate_projectile_diameter(query.projectile_mass, stone_density()));

    let (angle, velocity, score) = optimize_launch_parameters(
        query.projectile_mass,
        diameter,
        &wall,
        query.min_velocity.unwrap_or(20.0),
        query.max_velocity.unwrap_or(80.0),
        query.min_angle.unwrap_or(30.0),
        query.max_angle.unwrap_or(60.0),
    );

    success_response(OptimizeResult {
        optimal_angle_deg: angle,
        optimal_velocity_mps: velocity,
        max_effectiveness_score: score,
    })
}

#[derive(Debug, Serialize)]
struct OverviewItem {
    trebuchet: crate::storage::TrebuchetInfo,
    ballistics: Option<crate::ballistics::BallisticResult>,
    siege: Option<crate::siege::SiegeAssessment>,
}

async fn get_overview(State(state): State<AppState>) -> Json<ApiResponse<Vec<OverviewItem>>> {
    let trebuchets = state.db.get_trebuchets().await;
    let results = state.latest_results.lock().await;
    let siege = state.latest_siege.lock().await;

    let overview = trebuchets
        .into_iter()
        .map(|t| OverviewItem {
            ballistics: results.get(&t.id).cloned(),
            siege: siege.get(&t.id).cloned(),
            trebuchet: t,
        })
        .collect();

    success_response(overview)
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/metrics", get(get_metrics))
        .route("/api/trebuchets", get(get_trebuchets))
        .route("/api/trebuchets/:id", get(get_trebuchet))
        .route("/api/trebuchets/:id/ballistics", get(get_latest_ballistics))
        .route("/api/trebuchets/:id/siege", get(get_latest_siege))
        .route("/api/trebuchets/:id/sensor-history", get(get_sensor_history))
        .route("/api/walls", get(get_wall_types))
        .route("/api/calc/ballistics", get(calculate_ballistics))
        .route("/api/calc/siege", get(calculate_siege))
        .route("/api/calc/optimize", get(optimize_parameters))
        .route("/api/calc/ammo-compare", get(compare_ammo_endpoint))
        .route("/api/calc/wall-stress", get(analyze_wall_stress))
        .route("/api/calc/weak-point", get(find_weak_point))
        .route("/api/calc/coordinate", get(coordinate_fire))
        .route("/api/battles", get(list_battles))
        .route("/api/battles/:id", get(get_battle))
        .route("/api/battles/:id/start", get(start_battle))
        .route("/api/battles/:id/fire", get(battle_fire))
        .route("/api/battles/:id/advance", get(battle_advance))
        .route("/api/overview", get(get_overview))
        .with_state(state)
}

async fn get_metrics() -> impl IntoResponse {
    match crate::metrics::prometheus_handle() {
        Some(handle) => {
            let body = handle.render();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
                body,
            )
                .into_response()
        }
        None => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
