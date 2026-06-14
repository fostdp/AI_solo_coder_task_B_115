mod api;
mod ballistics;
mod siege;
mod storage;
mod config;
mod metrics;
mod udp_receiver;
mod ballistic_simulator;
mod siege_evaluator;
mod udp_server;
mod ammo;
mod fea;
mod genetic;
mod coordinator;
mod battles;
mod ammo_comparator;
mod wall_weakness_finder;
mod salvo_optimizer;
mod battle_simulator;

use api::{AppState, create_router};
use config::AppConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Initializing siege simulation backend...");

    metrics::init_metrics();
    info!("Metrics subsystem initialized (Prometheus exporter)");

    let app_config = Arc::new(AppConfig::load());
    info!("Configuration loaded");

    let db = Arc::new(storage::Database::new_with_config(&app_config.storage));

    if let Err(e) = db.load_trebuchets().await {
        error!("Failed to load trebuchets: {}", e);
    }

    if let Err(e) = db.load_wall_types().await {
        error!("Failed to load wall types: {}", e);
    }

    info!("Loaded {} trebuchets, {} wall types",
        db.get_trebuchets().await.len(),
        db.get_wall_types().await.len()
    );
    metrics::gauge_active_trebuchets(db.get_trebuchets().await.len());

    let latest_results = Arc::new(Mutex::new(
        HashMap::<u32, ballistics::BallisticResult>::new(),
    ));
    let latest_siege = Arc::new(Mutex::new(
        HashMap::<u32, siege::SiegeAssessment>::new(),
    ));

    let (udp_tx, ballistic_rx) =
        mpsc::channel::<udp_receiver::SensorEnvelope>(app_config.channel.udp_to_ballistic_capacity);
    let (ballistic_tx, siege_rx) =
        mpsc::channel::<ballistic_simulator::BallisticEnvelope>(app_config.channel.ballistic_to_siege_capacity);

    info!("Channel pipeline: UDP→Ballistic (cap={}), Ballistic→Siege (cap={})",
        app_config.channel.udp_to_ballistic_capacity,
        app_config.channel.ballistic_to_siege_capacity
    );

    let udp_cfg = app_config.udp.clone();
    let db_for_udp = db.clone();
    let results_for_legacy = latest_results.clone();
    let siege_for_legacy = latest_siege.clone();

    tokio::spawn(async move {
        if let Err(e) = udp_server::run_udp_server(
            &udp_cfg.bind_addr,
            db_for_udp,
            results_for_legacy,
            siege_for_legacy,
        )
        .await
        {
            warn!("Legacy UDP server stopped: {}", e);
        }
    });

    let udp_cfg_new = Arc::new(app_config.udp.clone());
    let udp_tx_clone = udp_tx.clone();
    tokio::spawn(async move {
        info!("UDP Receiver starting on {}", udp_cfg_new.bind_addr);
        if let Err(e) = udp_receiver::run_udp_receiver(udp_cfg_new, udp_tx_clone).await {
            error!("UDP Receiver crashed: {}", e);
        }
    });

    let cfg_for_ballistic = app_config.clone();
    let db_for_ballistic = db.clone();
    let results_for_ballistic = latest_results.clone();
    let ballistic = ballistic_simulator::BallisticSimulator::new(
        cfg_for_ballistic,
        db_for_ballistic,
        results_for_ballistic,
        ballistic_rx,
        ballistic_tx,
    );
    tokio::spawn(async move {
        info!("Ballistic simulator started");
        ballistic.run().await;
    });

    let cfg_for_siege = app_config.clone();
    let db_for_siege = db.clone();
    let siege_for_siege = latest_siege.clone();
    let evaluator = siege_evaluator::SiegeEvaluator::new(
        cfg_for_siege,
        db_for_siege,
        siege_for_siege,
        siege_rx,
    );
    tokio::spawn(async move {
        info!("Siege evaluator started");
        evaluator.run().await;
    });

    let state = AppState {
        db: db.clone(),
        latest_results: latest_results.clone(),
        latest_siege: latest_siege.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = create_router(state).layer(cors);

    let http_addr = "0.0.0.0:8080";

    info!("=============================================");
    info!("  Trebuchet Siege Simulation Backend");
    info!("  Architecture: UDP Recv → Ballistic → Siege");
    info!("  Channel sizes: UDP→Ballistic={}, Ballistic→Siege={}",
        app_config.channel.udp_to_ballistic_capacity,
        app_config.channel.ballistic_to_siege_capacity
    );
    info!("=============================================");
    info!("HTTP  server: http://{}", http_addr);
    info!("UDP   server: udp://{}", app_config.udp.bind_addr);
    info!("Metrics:      http://{}/metrics", http_addr);
    info!("ClickHouse:   disabled (memory buffer)");
    info!("=============================================");

    let addr: std::net::SocketAddr = http_addr.parse().unwrap();
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
