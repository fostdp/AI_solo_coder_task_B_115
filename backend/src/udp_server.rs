use crate::ballistics::{
    estimate_projectile_diameter, simulate_ballistics, BallisticInput, stone_density,
};
use crate::siege::{assess_siege_damage, SiegeInput, WallProperties};
use crate::storage::Database;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

const FRAME_MAGIC: u32 = 0x53474553;
const FRAME_VERSION: u8 = 1;
const FRAME_HEADER_SIZE: usize = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpSensorMessage {
    pub trebuchet_id: u32,
    pub cable_tension_newton: f64,
    pub launch_angle_deg: f64,
    pub initial_velocity_mps: f64,
    pub wind_speed_mps: f64,
    pub wind_direction_deg: f64,
    pub temperature_c: f64,
    pub air_density_kgm3: f64,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug)]
struct FrameParser {
    buffer: Vec<u8>,
    frames_received: u64,
    frames_valid: u64,
    frames_corrupted: u64,
}

impl FrameParser {
    fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(65536),
            frames_received: 0,
            frames_valid: 0,
            frames_corrupted: 0,
        }
    }

    fn append(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    fn parse_frames(&mut self) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();

        loop {
            if self.buffer.len() < FRAME_HEADER_SIZE {
                break;
            }

            let magic = u32::from_le_bytes([
                self.buffer[0], self.buffer[1], self.buffer[2], self.buffer[3],
            ]);

            if magic != FRAME_MAGIC {
                let rewind = self.find_next_magic();
                if rewind == 0 {
                    self.buffer.clear();
                    break;
                }
                self.buffer.drain(0..rewind);
                continue;
            }

            let version = self.buffer[4];
            if version != FRAME_VERSION {
                self.buffer.drain(0..1);
                self.frames_corrupted += 1;
                continue;
            }

            let payload_len = u16::from_le_bytes([self.buffer[5], self.buffer[6]]) as usize;

            if payload_len == 0 || payload_len > 65000 {
                self.buffer.drain(0..1);
                self.frames_corrupted += 1;
                continue;
            }

            let total_frame_len = FRAME_HEADER_SIZE + payload_len;
            if self.buffer.len() < total_frame_len {
                break;
            }

            let payload_bytes = &self.buffer[FRAME_HEADER_SIZE..FRAME_HEADER_SIZE + payload_len];

            let stored_checksum = u32::from_le_bytes([
                self.buffer[7], self.buffer[8], self.buffer[9], self.buffer[10],
            ]);

            let seq_num = u32::from_le_bytes([
                self.buffer[11], self.buffer[12], self.buffer[13], self.buffer[14],
            ]);
            let _ = seq_num;

            let computed_checksum = Self::fletcher32(payload_bytes);

            self.frames_received += 1;

            if stored_checksum == computed_checksum {
                messages.push(payload_bytes.to_vec());
                self.frames_valid += 1;
            } else {
                self.frames_corrupted += 1;
                eprintln!(
                    "Checksum mismatch: stored={}, computed={}, len={}",
                    stored_checksum, computed_checksum, payload_len
                );
            }

            self.buffer.drain(0..total_frame_len);
        }

        messages
    }

    fn find_next_magic(&self) -> usize {
        let bytes = &self.buffer;
        for i in 1..bytes.len().saturating_sub(3) {
            let candidate = u32::from_le_bytes([bytes[i], bytes[i+1], bytes[i+2], bytes[i+3]]);
            if candidate == FRAME_MAGIC {
                return i;
            }
        }
        0
    }

    fn fletcher32(data: &[u8]) -> u32 {
        let mut sum1: u32 = 0;
        let mut sum2: u32 = 0;
        let len = data.len();
        let mut i = 0;

        while i < len {
            let block_end = std::cmp::min(i + 360, len);
            while i < block_end {
                sum1 = sum1.wrapping_add(data[i] as u32);
                sum2 = sum2.wrapping_add(sum1);
                i += 1;
            }
            sum1 %= 65535;
            sum2 %= 65535;
        }

        (sum2 << 16) | sum1
    }

    fn stats(&self) -> (u64, u64, u64) {
        (self.frames_received, self.frames_valid, self.frames_corrupted)
    }
}

pub fn build_frame(payload: &[u8], seq_num: u32) -> Vec<u8> {
    let payload_len = payload.len() as u16;
    let checksum = FrameParser::fletcher32(payload);

    let mut frame = Vec::with_capacity(FRAME_HEADER_SIZE + payload.len());
    frame.extend_from_slice(&FRAME_MAGIC.to_le_bytes());
    frame.push(FRAME_VERSION);
    frame.extend_from_slice(&payload_len.to_le_bytes());
    frame.extend_from_slice(&checksum.to_le_bytes());
    frame.extend_from_slice(&seq_num.to_le_bytes());
    frame.extend_from_slice(payload);

    frame
}

pub async fn run_udp_server(
    bind_addr: &str,
    db: Arc<Database>,
    latest_results: Arc<Mutex<HashMap<u32, crate::ballistics::BallisticResult>>>,
    latest_siege: Arc<Mutex<HashMap<u32, crate::siege::SiegeAssessment>>>,
) -> Result<(), String> {
    let socket = UdpSocket::bind(bind_addr)
        .await
        .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

    println!("UDP server listening on {} (Frame Protocol v{})", bind_addr, FRAME_VERSION);
    println!("Frame format: MAGIC(4) + VERSION(1) + LEN(2) + CHECKSUM(4) + SEQ(4) + PAYLOAD(N)");

    let mut buf = [0u8; 65535];
    let mut parser = FrameParser::new();
    let mut stats_timer = std::time::Instant::now();
    let mut sequence_numbers: HashMap<SocketAddrKey, u32> = HashMap::new();

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, addr)) => {
                let data = &buf[..len];

                if data.len() >= 4 {
                    let magic_check = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                    if magic_check == FRAME_MAGIC {
                        parser.append(data);
                    } else {
                        if let Ok(text) = std::str::from_utf8(data) {
                            if let Ok(msg) = serde_json::from_str::<UdpSensorMessage>(text) {
                                process_sensor_message(
                                    &db,
                                    msg,
                                    latest_results.clone(),
                                    latest_siege.clone(),
                                )
                                .await;
                                continue;
                            }
                        }
                        parser.append(data);
                    }
                } else {
                    parser.append(data);
                }

                let messages = parser.parse_frames();
                for msg_bytes in messages {
                    if let Ok(text) = std::str::from_utf8(&msg_bytes) {
                        if let Ok(msg) = serde_json::from_str::<UdpSensorMessage>(text) {
                            let key = SocketAddrKey::from_socketaddr(&addr);
                            let counter = sequence_numbers.entry(key).or_insert(0);
                            *counter = counter.wrapping_add(1);

                            process_sensor_message(
                                &db,
                                msg,
                                latest_results.clone(),
                                latest_siege.clone(),
                            )
                            .await;
                        }
                    }
                }

                if stats_timer.elapsed() > std::time::Duration::from_secs(30) {
                    let (received, valid, corrupted) = parser.stats();
                    if received > 0 {
                        println!(
                            "[UDP Stats] 接收: {}, 有效: {}, 损坏: {}",
                            received, valid, corrupted
                        );
                    }
                    stats_timer = std::time::Instant::now();
                }
            }
            Err(e) => {
                eprintln!("UDP receive error: {}", e);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SocketAddrKey {
    ip: std::net::IpAddr,
    port: u16,
}

impl SocketAddrKey {
    fn from_socketaddr(addr: &std::net::SocketAddr) -> Self {
        Self {
            ip: addr.ip(),
            port: addr.port(),
        }
    }
}

async fn process_sensor_message(
    db: &Database,
    msg: UdpSensorMessage,
    latest_results: Arc<Mutex<HashMap<u32, crate::ballistics::BallisticResult>>>,
    latest_siege: Arc<Mutex<HashMap<u32, crate::siege::SiegeAssessment>>>,
) {
    let timestamp = msg.timestamp.unwrap_or_else(Utc::now);

    let sensor_data = crate::storage::SensorData {
        timestamp,
        trebuchet_id: msg.trebuchet_id,
        cable_tension_newton: msg.cable_tension_newton,
        launch_angle_deg: msg.launch_angle_deg,
        initial_velocity_mps: msg.initial_velocity_mps,
        wind_speed_mps: msg.wind_speed_mps,
        wind_direction_deg: msg.wind_direction_deg,
        temperature_c: msg.temperature_c,
        air_density_kgm3: msg.air_density_kgm3,
    };

    if let Err(e) = db.insert_sensor_data(sensor_data).await {
        eprintln!("Failed to insert sensor data: {}", e);
        return;
    }

    if let Some(trebuchet) = db.get_trebuchet_by_id(msg.trebuchet_id).await {
        let projectile_diameter =
            estimate_projectile_diameter(trebuchet.projectile_kg, stone_density());

        let ballistic_input = BallisticInput {
            initial_velocity: msg.initial_velocity_mps,
            launch_angle_deg: msg.launch_angle_deg,
            projectile_mass_kg: trebuchet.projectile_kg,
            projectile_diameter_m: projectile_diameter,
            air_density_kgm3: msg.air_density_kgm3,
            wind_speed_mps: msg.wind_speed_mps,
            wind_direction_deg: msg.wind_direction_deg,
            launch_height_m: trebuchet.arm_length_m * 0.4,
        };

        let result = simulate_ballistics(&ballistic_input);

        {
            let mut results = latest_results.lock().await;
            results.insert(msg.trebuchet_id, result.clone());
        }

        if let Err(e) = db
            .insert_ballistics_result(
                msg.trebuchet_id,
                timestamp,
                msg.initial_velocity_mps,
                msg.launch_angle_deg,
                &result,
            )
            .await
        {
            eprintln!("Failed to insert ballistics result: {}", e);
        }

        let default_wall = WallProperties {
            thickness_m: 3.0,
            material: "rammed_earth".to_string(),
            density_kgm3: 1800.0,
            compressive_strength_pa: 2_000_000.0,
            tensile_strength_pa: 200_000.0,
        };

        let siege_input = SiegeInput {
            impact_energy_j: result.impact_kinetic_energy_j,
            projectile_mass_kg: trebuchet.projectile_kg,
            projectile_diameter_m: projectile_diameter,
            impact_angle_deg: result.impact_angle_deg,
            wall: default_wall.clone(),
            ammo_type: crate::ammo::AmmoType::RoundStone,
        };

        let assessment = assess_siege_damage(&siege_input);

        {
            let mut siege = latest_siege.lock().await;
            siege.insert(msg.trebuchet_id, assessment.clone());
        }

        if let Err(e) = db
            .insert_siege_assessment(
                msg.trebuchet_id,
                default_wall.thickness_m,
                &default_wall.material,
                default_wall.density_kgm3,
                default_wall.compressive_strength_pa,
                result.impact_kinetic_energy_j,
                &assessment,
                45.0,
                msg.initial_velocity_mps,
            )
            .await
        {
            eprintln!("Failed to insert siege assessment: {}", e);
        }
    }
}
