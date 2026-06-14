use metrics::{counter, gauge, histogram, describe_counter, describe_gauge, describe_histogram, Unit};
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::layers::{PrefixLayer, Layer};
use std::time::Instant;
use std::sync::OnceLock;

static METRICS_INIT: OnceLock<()> = OnceLock::new();
static PROMETHEUS_HANDLE: OnceLock<metrics_exporter_prometheus::PrometheusHandle> = OnceLock::new();

pub fn init_metrics() -> &'static metrics_exporter_prometheus::PrometheusHandle {
    METRICS_INIT.get_or_init(|| {
        let builder = PrometheusBuilder::new();
        let handle = builder
            .install_recorder()
            .expect("Failed to install Prometheus recorder");

        let _ = PROMETHEUS_HANDLE.set(handle);

        describe_counter!("udp.packets_total", Unit::Count, "Total UDP packets received");
        describe_counter!("udp.frames_valid_total", Unit::Count, "Valid frames parsed");
        describe_counter!("udp.frames_corrupted_total", Unit::Count, "Corrupted frames received");
        describe_gauge!("udp.channel_depth", Unit::Count, "UDP→Ballistic channel depth");

        describe_counter!("ballistic.simulations_total", Unit::Count, "Total ballistic simulations run");
        describe_histogram!("ballistic.simulation_duration_seconds", Unit::Seconds, "Ballistic simulation duration");
        describe_gauge!("ballistic.channel_depth", Unit::Count, "Ballistic→Siege channel depth");
        describe_gauge!("ballistic.solver_steps", Unit::Count, "RK4 solver steps per simulation");

        describe_counter!("siege.assessments_total", Unit::Count, "Total siege assessments run");
        describe_histogram!("siege.assessment_duration_seconds", Unit::Seconds, "Siege assessment duration");

        describe_counter!("http.requests_total", Unit::Count, "Total HTTP requests");
        describe_histogram!("http.request_duration_seconds", Unit::Seconds, "HTTP request duration");

        describe_gauge!("trebuchet.active_count", Unit::Count, "Number of active trebuchets");
        describe_gauge!("storage.sensor_buffer_size", Unit::Count, "Sensor data buffer size");
        describe_gauge!("storage.ballistics_buffer_size", Unit::Count, "Ballistics result buffer size");
        describe_gauge!("storage.siege_buffer_size", Unit::Count, "Siege assessment buffer size");

        describe_counter!("ammo.comparisons_total", Unit::Count, "Total ammo comparisons performed");
        describe_counter!("fea.analyses_total", Unit::Count, "Total FEA analyses performed");
        describe_histogram!("fea.analysis_duration_seconds", Unit::Seconds, "FEA analysis duration");
        describe_counter!("ga.optimizations_total", Unit::Count, "Total GA optimizations performed");
        describe_histogram!("ga.optimization_duration_seconds", Unit::Seconds, "GA optimization duration");
        describe_gauge!("ga.best_fitness", Unit::Count, "Best fitness from last GA run");
        describe_counter!("coordinator.assignments_total", Unit::Count, "Total coordination assignments");
        describe_counter!("battles.started_total", Unit::Count, "Total battles started");
        describe_counter!("battles.impacts_total", Unit::Count, "Total battle impacts recorded");
        describe_gauge!("battles.active_count", Unit::Count, "Active battle scenarios");
    });
    PROMETHEUS_HANDLE.get().expect("Prometheus handle not initialized")
}

pub fn prometheus_handle() -> Option<&'static metrics_exporter_prometheus::PrometheusHandle> {
    PROMETHEUS_HANDLE.get()
}

pub fn increment_udp_packets() {
    counter!("udp.packets_total", 1);
}

pub fn increment_udp_frames_valid() {
    counter!("udp.frames_valid_total", 1);
}

pub fn increment_udp_frames_corrupted() {
    counter!("udp.frames_corrupted_total", 1);
}

pub fn gauge_udp_channel_depth(depth: usize) {
    gauge!("udp.channel_depth", depth as f64);
}

pub fn increment_ballistic_simulations() {
    counter!("ballistic.simulations_total", 1);
}

pub fn record_ballistic_duration(start: Instant) {
    histogram!("ballistic.simulation_duration_seconds", start.elapsed().as_secs_f64());
}

pub fn gauge_ballistic_channel_depth(depth: usize) {
    gauge!("ballistic.channel_depth", depth as f64);
}

pub fn gauge_ballistic_solver_steps(steps: u32) {
    gauge!("ballistic.solver_steps", steps as f64);
}

pub fn increment_siege_assessments() {
    counter!("siege.assessments_total", 1);
}

pub fn record_siege_duration(start: Instant) {
    histogram!("siege.assessment_duration_seconds", start.elapsed().as_secs_f64());
}

pub fn increment_http_requests(method: &str, path: &str) {
    counter!("http.requests_total", 1, "method" => method.to_string(), "path" => path.to_string());
}

pub fn record_http_duration(method: &str, path: &str, start: Instant) {
    histogram!("http.request_duration_seconds", start.elapsed().as_secs_f64(),
        "method" => method.to_string(), "path" => path.to_string());
}

pub fn gauge_active_trebuchets(count: usize) {
    gauge!("trebuchet.active_count", count as f64);
}

pub fn gauge_sensor_buffer_size(size: usize) {
    gauge!("storage.sensor_buffer_size", size as f64);
}

pub fn gauge_ballistics_buffer_size(size: usize) {
    gauge!("storage.ballistics_buffer_size", size as f64);
}

pub fn gauge_siege_buffer_size(size: usize) {
    gauge!("storage.siege_buffer_size", size as f64);
}

pub fn increment_ammo_comparisons() {
    counter!("ammo.comparisons_total", 1);
}

pub fn increment_fea_analyses() {
    counter!("fea.analyses_total", 1);
}

pub fn record_fea_duration(start: Instant) {
    histogram!("fea.analysis_duration_seconds", start.elapsed().as_secs_f64());
}

pub fn increment_ga_optimizations() {
    counter!("ga.optimizations_total", 1);
}

pub fn record_ga_duration(start: Instant) {
    histogram!("ga.optimization_duration_seconds", start.elapsed().as_secs_f64());
}

pub fn gauge_ga_best_fitness(fitness: f64) {
    gauge!("ga.best_fitness", fitness);
}

pub fn increment_coordinator_assignments(count: usize) {
    counter!("coordinator.assignments_total", count as u64);
}

pub fn increment_battles_started() {
    counter!("battles.started_total", 1);
}

pub fn increment_battle_impacts() {
    counter!("battles.impacts_total", 1);
}

pub fn gauge_active_battles(count: usize) {
    gauge!("battles.active_count", count as f64);
}
