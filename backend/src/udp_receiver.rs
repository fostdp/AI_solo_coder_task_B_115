use crate::config::UdpConfig;
use crate::metrics;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;
use tracing::{debug, info, warn, error};

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

#[derive(Debug, Clone)]
pub struct SensorEnvelope {
    pub source_addr: SocketAddr,
    pub received_at: DateTime<Utc>,
    pub message: UdpSensorMessage,
}

#[derive(Debug)]
struct FrameParser {
    buffer: Vec<u8>,
    frames_received: u64,
    frames_valid: u64,
    frames_corrupted: u64,
    config: Arc<UdpConfig>,
}

impl FrameParser {
    fn new(config: Arc<UdpConfig>) -> Self {
        Self {
            buffer: Vec::with_capacity(config.buffer_capacity),
            frames_received: 0,
            frames_valid: 0,
            frames_corrupted: 0,
            config,
        }
    }

    fn append(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    fn parse_frames(&mut self) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();
        let header_size = self.config.frame_header_size;
        let magic = self.config.frame_magic;
        let version = self.config.frame_version;
        let max_payload = self.config.max_payload_size;

        loop {
            if self.buffer.len() < header_size {
                break;
            }

            let candidate_magic = u32::from_le_bytes([
                self.buffer[0], self.buffer[1], self.buffer[2], self.buffer[3],
            ]);

            if candidate_magic != magic {
                let rewind = self.find_next_magic();
                if rewind == 0 {
                    self.buffer.clear();
                    break;
                }
                self.buffer.drain(0..rewind);
                continue;
            }

            if self.buffer[4] != version {
                self.buffer.drain(0..1);
                self.frames_corrupted += 1;
                continue;
            }

            let payload_len =
                u16::from_le_bytes([self.buffer[5], self.buffer[6]]) as usize;

            if payload_len == 0 || payload_len > max_payload {
                self.buffer.drain(0..1);
                self.frames_corrupted += 1;
                continue;
            }

            let total_frame_len = header_size + payload_len;
            if self.buffer.len() < total_frame_len {
                break;
            }

            let payload_bytes = &self.buffer[header_size..header_size + payload_len];

            let stored_checksum = u32::from_le_bytes([
                self.buffer[7], self.buffer[8], self.buffer[9], self.buffer[10],
            ]);

            let _seq_num = u32::from_le_bytes([
                self.buffer[11], self.buffer[12], self.buffer[13], self.buffer[14],
            ]);

            let computed_checksum = Self::fletcher32(payload_bytes);

            self.frames_received += 1;

            if stored_checksum == computed_checksum {
                messages.push(payload_bytes.to_vec());
                self.frames_valid += 1;
                metrics::increment_udp_frames_valid();
            } else {
                self.frames_corrupted += 1;
                metrics::increment_udp_frames_corrupted();
                warn!(
                    "[UDP] Checksum mismatch: stored={}, computed={}, len={}",
                    stored_checksum, computed_checksum, payload_len
                );
            }

            self.buffer.drain(0..total_frame_len);
        }

        messages
    }

    fn find_next_magic(&self) -> usize {
        let bytes = &self.buffer;
        let magic = self.config.frame_magic;
        for i in 1..bytes.len().saturating_sub(3) {
            let candidate =
                u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
            if candidate == magic {
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

pub fn build_frame(payload: &[u8], seq_num: u32, config: &UdpConfig) -> Vec<u8> {
    let payload_len = payload.len() as u16;
    let checksum = FrameParser::fletcher32(payload);

    let mut frame = Vec::with_capacity(config.frame_header_size + payload.len());
    frame.extend_from_slice(&config.frame_magic.to_le_bytes());
    frame.push(config.frame_version);
    frame.extend_from_slice(&payload_len.to_le_bytes());
    frame.extend_from_slice(&checksum.to_le_bytes());
    frame.extend_from_slice(&seq_num.to_le_bytes());
    frame.extend_from_slice(payload);

    frame
}

pub async fn run_udp_receiver(
    config: Arc<UdpConfig>,
    tx: Sender<SensorEnvelope>,
) -> Result<(), String> {
    let socket = UdpSocket::bind(&config.bind_addr)
        .await
        .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

    println!(
        "[UDP Receiver] listening on {} (Frame Protocol v{})",
        config.bind_addr, config.frame_version
    );
    println!(
        "[UDP Receiver] frame: MAGIC(4) + VERSION(1) + LEN(2) + CHECKSUM(4) + SEQ(4) + PAYLOAD(N)"
    );

    let mut buf = vec![0u8; 65535];
    let mut parser = FrameParser::new(config.clone());
    let mut stats_timer = std::time::Instant::now();
    let mut _sequence_numbers: HashMap<SocketAddrKey, u32> = HashMap::new();

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, addr)) => {
                let data = &buf[..len];
                let received_at = Utc::now();
                metrics::increment_udp_packets();

                let mut payloads: Vec<Vec<u8>> = Vec::new();

                if data.len() >= 4 {
                    let magic_check =
                        u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                    if magic_check == config.frame_magic {
                        parser.append(data);
                        payloads = parser.parse_frames();
                    } else if let Ok(text) = std::str::from_utf8(data) {
                        if let Ok(_msg) =
                            serde_json::from_str::<UdpSensorMessage>(text)
                        {
                            payloads.push(data.to_vec());
                        } else {
                            parser.append(data);
                            payloads = parser.parse_frames();
                        }
                    } else {
                        parser.append(data);
                        payloads = parser.parse_frames();
                    }
                } else {
                    parser.append(data);
                    payloads = parser.parse_frames();
                }

                for msg_bytes in payloads {
                    let parsed_msg = if let Ok(text) = std::str::from_utf8(&msg_bytes)
                    {
                        serde_json::from_str::<UdpSensorMessage>(text).ok()
                    } else {
                        None
                    };

                    if let Some(msg) = parsed_msg {
                        let key = SocketAddrKey::from_socketaddr(&addr);
                        let counter =
                            _sequence_numbers.entry(key).or_insert(0);
                        *counter = counter.wrapping_add(1);

                        let envelope = SensorEnvelope {
                            source_addr: addr,
                            received_at,
                            message: msg,
                        };

                        if let Err(e) = tx.send(envelope).await {
                            warn!(
                                "[UDP Receiver] failed to send to ballistic channel: {}",
                                e
                            );
                        }
                        metrics::gauge_udp_channel_depth(tx.capacity());
                    }
                }

                if stats_timer.elapsed()
                    > std::time::Duration::from_secs(config.stats_interval_secs)
                {
                    let (received, valid, corrupted) = parser.stats();
                    if received > 0 {
                        println!(
                            "[UDP Stats] recv={}, valid={}, corrupt={}, channel_len={}",
                            received,
                            valid,
                            corrupted,
                            "n/a".to_string()
                        );
                    }
                    stats_timer = std::time::Instant::now();
                }
            }
            Err(e) => {
                eprintln!("[UDP Receiver] recv error: {}", e);
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
    fn from_socketaddr(addr: &SocketAddr) -> Self {
        Self {
            ip: addr.ip(),
            port: addr.port(),
        }
    }
}
