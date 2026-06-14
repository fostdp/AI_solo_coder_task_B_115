#!/usr/bin/env python3
"""
古代投石机传感器模拟器 v2
每台投石机每1分钟通过UDP上报传感器数据
支持帧同步协议: MAGIC(4) + VERSION(1) + LEN(2) + CHECKSUM(4) + SEQ(4) + PAYLOAD(N)
"""

import json
import socket
import struct
import time
import random
import math
import argparse
from datetime import datetime, timezone
from collections import deque

FRAME_MAGIC = 0x53474553
FRAME_VERSION = 1
FRAME_HEADER_SIZE = 15

TREBUCHETS = [
    {"id": 1, "name": "回回炮-甲", "type": "配重式", "counterweight_kg": 3000, "projectile_kg": 90, "arm_length_m": 12.0, "max_angle_deg": 50.0},
    {"id": 2, "name": "回回炮-乙", "type": "配重式", "counterweight_kg": 5000, "projectile_kg": 150, "arm_length_m": 15.0, "max_angle_deg": 55.0},
    {"id": 3, "name": "襄阳砲-壹", "type": "配重式", "counterweight_kg": 4000, "projectile_kg": 120, "arm_length_m": 13.5, "max_angle_deg": 52.0},
    {"id": 4, "name": "人力砲-一号", "type": "人力牵引式", "counterweight_kg": 0, "projectile_kg": 30, "arm_length_m": 8.0, "max_angle_deg": 45.0},
    {"id": 5, "name": "人力砲-二号", "type": "人力牵引式", "counterweight_kg": 0, "projectile_kg": 25, "arm_length_m": 7.5, "max_angle_deg": 42.0},
    {"id": 6, "name": "旋风砲", "type": "人力牵引式", "counterweight_kg": 0, "projectile_kg": 20, "arm_length_m": 6.0, "max_angle_deg": 48.0},
    {"id": 7, "name": "虎蹲砲", "type": "配重式", "counterweight_kg": 1500, "projectile_kg": 50, "arm_length_m": 9.0, "max_angle_deg": 47.0},
    {"id": 8, "name": "无敌砲", "type": "配重式", "counterweight_kg": 6000, "projectile_kg": 200, "arm_length_m": 18.0, "max_angle_deg": 58.0},
    {"id": 9, "name": "飞云砲", "type": "人力牵引式", "counterweight_kg": 0, "projectile_kg": 15, "arm_length_m": 5.5, "max_angle_deg": 40.0},
    {"id": 10, "name": "震天雷砲", "type": "配重式", "counterweight_kg": 8000, "projectile_kg": 300, "arm_length_m": 20.0, "max_angle_deg": 60.0},
]

GRAVITY = 9.81
STONE_DENSITY = 2600.0


def fletcher32(data: bytes) -> int:
    sum1 = 0
    sum2 = 0
    length = len(data)
    i = 0
    while i < length:
        block_end = min(i + 360, length)
        while i < block_end:
            sum1 = (sum1 + data[i]) % 65535
            sum2 = (sum2 + sum1) % 65535
            i += 1
    return (sum2 << 16) | sum1


def build_frame(payload: bytes, seq_num: int) -> bytes:
    payload_len = len(payload)
    checksum = fletcher32(payload)
    header = struct.pack('<IBHII', FRAME_MAGIC, FRAME_VERSION, payload_len, checksum, seq_num)
    return header + payload


def build_frame_legacy(payload: bytes, seq_num: int = 0) -> bytes:
    return payload


def estimate_velocity(trebuchet, angle_deg):
    return estimate_velocity_with_cw(trebuchet, angle_deg, trebuchet["counterweight_kg"])


def estimate_velocity_with_cw(trebuchet, angle_deg, counterweight_kg):
    if trebuchet["type"] == "配重式":
        projectile = trebuchet["projectile_kg"]
        arm = trebuchet["arm_length_m"]
        angle_rad = math.radians(angle_deg)

        height_drop = arm * (1 - math.cos(angle_rad))
        potential_energy = counterweight_kg * GRAVITY * height_drop * 0.7
        velocity = math.sqrt(2 * potential_energy / projectile)
        return velocity * random.uniform(0.9, 1.1)
    else:
        pullers = int(trebuchet["projectile_kg"] * 3)
        force_per_puller = 500
        total_force = pullers * force_per_puller
        work = total_force * trebuchet["arm_length_m"] * 0.5
        velocity = math.sqrt(2 * work / trebuchet["projectile_kg"])
        return velocity * random.uniform(0.85, 1.15)


def estimate_tension(trebuchet, angle_deg):
    return estimate_tension_with_cw(trebuchet, angle_deg, trebuchet["counterweight_kg"])


def estimate_tension_with_cw(trebuchet, angle_deg, counterweight_kg):
    if trebuchet["type"] == "配重式":
        base_tension = counterweight_kg * GRAVITY * 1.5
    else:
        pullers = int(trebuchet["projectile_kg"] * 3)
        base_tension = pullers * 500

    angle_factor = math.sin(math.radians(angle_deg)) + 0.5
    return base_tension * angle_factor * random.uniform(0.95, 1.05)


def generate_sensor_data(trebuchet, base_angle=None, counterweight_ratio=1.0):
    if base_angle is None:
        base_angle = trebuchet["max_angle_deg"] * random.uniform(0.7, 0.95)

    angle = base_angle + random.uniform(-2, 2)
    angle = max(20, min(angle, trebuchet["max_angle_deg"]))

    effective_cw = trebuchet["counterweight_kg"] * counterweight_ratio
    velocity = estimate_velocity_with_cw(trebuchet, angle, effective_cw)
    tension = estimate_tension_with_cw(trebuchet, angle, effective_cw)

    wind_speed = random.uniform(0, 15)
    wind_direction = random.uniform(0, 360)
    temperature = random.uniform(5, 35)
    air_density = 1.293 * (273.15 / (273.15 + temperature)) * 0.98

    return {
        "trebuchet_id": trebuchet["id"],
        "cable_tension_newton": round(tension, 2),
        "launch_angle_deg": round(angle, 2),
        "initial_velocity_mps": round(velocity, 2),
        "wind_speed_mps": round(wind_speed, 2),
        "wind_direction_deg": round(wind_direction, 2),
        "temperature_c": round(temperature, 2),
        "air_density_kgm3": round(air_density, 4),
        "counterweight_kg": round(effective_cw, 1),
        "counterweight_ratio": round(counterweight_ratio, 3),
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }


class TrebuchetState:
    """单台投石机的状态机，用于不同模式下的参数生成

    支持模式:
      - random: 完全随机
      - angle_sweep: 角度从 min 扫到 max
      - cw_sweep: 配重从 min 扫到 max
      - combined: 角度+配重联合扫描
      - oscillation: 振荡模式
    """

    def __init__(self, trebuchet, mode="random", angle_min=None, angle_max=None,
                 cw_min_ratio=0.5, cw_max_ratio=1.5, sweep_steps=10):
        self.trebuchet = trebuchet
        self.mode = mode
        self.angle_min = angle_min if angle_min is not None else 30.0
        self.angle_max = angle_max if angle_max is not None else trebuchet["max_angle_deg"]
        self.cw_min_ratio = cw_min_ratio
        self.cw_max_ratio = cw_max_ratio
        self.sweep_steps = sweep_steps
        self._step = 0
        self._angle_direction = 1
        self._cw_direction = 1

    def next(self):
        if self.mode == "random":
            angle = random.uniform(self.angle_min, self.angle_max)
            cw_ratio = random.uniform(self.cw_min_ratio, self.cw_max_ratio)

        elif self.mode == "angle_sweep":
            t = (self._step % (self.sweep_steps * 2)) / (self.sweep_steps * 2)
            angle = self.angle_min + (self.angle_max - self.angle_min) * (1 - abs(2 * t - 1))
            cw_ratio = 1.0
            self._step += 1

        elif self.mode == "cw_sweep":
            t = (self._step % (self.sweep_steps * 2)) / (self.sweep_steps * 2)
            cw_ratio = self.cw_min_ratio + (self.cw_max_ratio - self.cw_min_ratio) * (1 - abs(2 * t - 1))
            angle = self.trebuchet["max_angle_deg"] * 0.8
            self._step += 1

        elif self.mode == "combined":
            t_angle = (self._step % (self.sweep_steps * 2)) / (self.sweep_steps * 2)
            angle = self.angle_min + (self.angle_max - self.angle_min) * (1 - abs(2 * t_angle - 1))
            t_cw = ((self._step + self.sweep_steps // 2) % (self.sweep_steps * 2)) / (self.sweep_steps * 2)
            cw_ratio = self.cw_min_ratio + (self.cw_max_ratio - self.cw_min_ratio) * (1 - abs(2 * t_cw - 1))
            self._step += 1

        elif self.mode == "oscillation":
            t = self._step / self.sweep_steps
            angle = (self.angle_min + self.angle_max) / 2 + \
                    math.sin(t * math.pi * 2) * (self.angle_max - self.angle_min) / 2
            cw_ratio = 1.0 + math.sin(t * math.pi * 1.5) * 0.3
            cw_ratio = max(self.cw_min_ratio, min(self.cw_max_ratio, cw_ratio))
            self._step += 1

        else:
            angle = self.trebuchet["max_angle_deg"] * 0.8
            cw_ratio = 1.0

        return angle, cw_ratio

    def generate(self):
        angle, cw_ratio = self.next()
        return generate_sensor_data(self.trebuchet, base_angle=angle, counterweight_ratio=cw_ratio)


def send_udp(sock, host, port, data, seq_num, use_framed=True, corrupt_prob=0.0):
    payload = json.dumps(data).encode("utf-8")

    if use_framed:
        if random.random() < corrupt_prob:
            payload = bytearray(payload)
            idx = random.randint(0, len(payload) - 1)
            payload[idx] ^= random.randint(1, 255)
            payload = bytes(payload)

        frame = build_frame(payload, seq_num)
    else:
        frame = build_frame_legacy(payload)

    if random.random() < 0.001 and corrupt_prob > 0:
        if len(frame) > 20:
            cut = random.randint(10, len(frame) - 1)
            frame = frame[:cut]

    sock.sendto(frame, (host, port))


class SlidingWindowSender:
    def __init__(self, window_size=5):
        self.window_size = window_size
        self.next_seq = 0
        self.send_buffer = deque()

    def get_next_seq(self):
        seq = self.next_seq
        self.next_seq = (self.next_seq + 1) % (2**32)
        return seq

    def confirm(self, seq):
        while self.send_buffer and self.send_buffer[0][0] <= seq:
            self.send_buffer.popleft()


def run_simulation(host, port, interval_seconds, count, use_framed=True, burst_mode=False,
                   corrupt_prob=0.0, mode="random", angle_min=None, angle_max=None,
                   cw_min_ratio=0.5, cw_max_ratio=1.5, sweep_steps=10):
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sender = SlidingWindowSender(window_size=8)

    states = [
        TrebuchetState(t, mode=mode, angle_min=angle_min, angle_max=angle_max,
                       cw_min_ratio=cw_min_ratio, cw_max_ratio=cw_max_ratio,
                       sweep_steps=sweep_steps)
        for t in TREBUCHETS
    ]

    protocol = f"帧协议 v{FRAME_VERSION}" if use_framed else "原始JSON协议"
    mode_names = {
        "random": "完全随机",
        "angle_sweep": "角度扫描",
        "cw_sweep": "配重扫描",
        "combined": "角度+配重联合扫描",
        "oscillation": "振荡模式",
    }
    mode_desc = mode_names.get(mode, mode)

    print(f"投石机模拟器启动 v3")
    print(f"目标: {host}:{port}")
    print(f"协议: {protocol}")
    print(f"帧头格式: MAGIC(4B) + VERSION(1B) + LEN(2B) + FLETCHER32(4B) + SEQ(4B) + PAYLOAD")
    print(f"模式: {mode_desc}")
    print(f"间隔: {interval_seconds}秒" + (f" [突发模式]" if burst_mode else ""))
    print(f"投石机数量: {len(TREBUCHETS)}")
    if mode != "random":
        if angle_min is not None or angle_max is not None:
            print(f"角度范围: {angle_min or 'min'}° ~ {angle_max or 'max'}°")
        print(f"配重倍率范围: {cw_min_ratio}x ~ {cw_max_ratio}x")
        print(f"扫描步数: {sweep_steps}")
    if corrupt_prob > 0:
        print(f"损坏注入概率: {corrupt_prob * 100}%")
    print("=" * 70)

    iteration = 0
    total_sent = 0
    start_time = time.time()
    last_stats = start_time

    try:
        while True:
            iteration += 1
            ts = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
            sent_this_round = 0

            indices = list(range(len(states)))
            if burst_mode:
                random.shuffle(indices)

            for idx in indices:
                state = states[idx]
                trebuchet = state.trebuchet
                data = state.generate()
                seq = sender.get_next_seq()

                send_udp(sock, host, port, data, seq, use_framed, corrupt_prob)
                sent_this_round += 1
                total_sent += 1

                if burst_mode and random.random() < 0.3:
                    time.sleep(random.uniform(0.001, 0.01))

                if idx < 3:
                    cw_info = f" 配重: {data['counterweight_kg']:.0f}kg ({data['counterweight_ratio']:.2f}x)" if mode != "random" else ""
                    print(f"  [{trebuchet['name']}] 角度: {data['launch_angle_deg']}° "
                          f"初速: {data['initial_velocity_mps']}m/s "
                          f"张力: {data['cable_tension_newton']:.0f}N{cw_info} "
                          f"SEQ:{seq}")

            elapsed = time.time() - last_stats
            if elapsed >= 60:
                total_elapsed = time.time() - start_time
                rate = total_sent / total_elapsed if total_elapsed > 0 else 0
                print(f"\n[统计] 总发送: {total_sent} | 速率: {rate:.2f}/s | 运行: {total_elapsed:.0f}s")
                last_stats = time.time()

            if count > 0 and iteration >= count:
                print(f"\n完成 {count} 轮模拟，共发送 {total_sent} 帧")
                break

            sleep_time = interval_seconds
            if burst_mode and sleep_time > 1:
                sleep_time = sleep_time * 0.7
            time.sleep(sleep_time)

    except KeyboardInterrupt:
        total_elapsed = time.time() - start_time
        rate = total_sent / total_elapsed if total_elapsed > 0 else 0
        print(f"\n\n模拟已停止")
        print(f"总发送: {total_sent} 帧 | 运行: {total_elapsed:.1f}s | 平均速率: {rate:.2f}/s")
    finally:
        sock.close()


def run_stress_test(host, port, duration_seconds=10, rate_per_second=1000):
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sender = SlidingWindowSender()

    print(f"压力测试启动")
    print(f"目标: {host}:{port} | 目标速率: {rate_per_second}/s | 时长: {duration_seconds}s")
    print("=" * 70)

    total_sent = 0
    start_time = time.time()
    end_time = start_time + duration_seconds
    last_stats = start_time
    sent_in_window = 0

    try:
        while time.time() < end_time:
            window_start = time.time()
            batch = min(rate_per_second // 10, 100)

            for _ in range(batch):
                t = random.choice(TREBUCHETS)
                data = generate_sensor_data(t)
                seq = sender.get_next_seq()
                send_udp(sock, host, port, data, seq, use_framed=True)
                sent_in_window += 1
                total_sent += 1

            elapsed_in_window = time.time() - window_start
            sleep_time = 0.1 - elapsed_in_window
            if sleep_time > 0:
                time.sleep(sleep_time)

            if time.time() - last_stats >= 1.0:
                actual_rate = sent_in_window / (time.time() - last_stats)
                remaining = max(0, end_time - time.time())
                print(f"  [{remaining:.0f}s] 速率: {actual_rate:.0f}/s | 累计: {total_sent}")
                sent_in_window = 0
                last_stats = time.time()

    finally:
        total_elapsed = time.time() - start_time
        rate = total_sent / total_elapsed if total_elapsed > 0 else 0
        print(f"\n压力测试完成")
        print(f"总发送: {total_sent} 帧 | 运行: {total_elapsed:.1f}s | 实际速率: {rate:.0f}/s")
        sock.close()


def main():
    parser = argparse.ArgumentParser(description="古代投石机传感器模拟器 v3 (帧同步协议)")
    parser.add_argument("--host", default="127.0.0.1", help="UDP目标主机")
    parser.add_argument("--port", type=int, default=9001, help="UDP目标端口")
    parser.add_argument("--interval", type=int, default=60, help="发送间隔(秒)")
    parser.add_argument("--count", type=int, default=0, help="发送轮数(0=无限)")
    parser.add_argument("--fast", action="store_true", help="快速模式(2秒间隔)")
    parser.add_argument("--burst", action="store_true", help="突发模式(模拟网络拥塞)")
    parser.add_argument("--legacy", action="store_true", help="使用旧的原始JSON协议(无帧头)")
    parser.add_argument("--corrupt", type=float, default=0.0, help="人为注入损坏帧概率(0-1)")
    parser.add_argument("--stress", action="store_true", help="运行压力测试模式")
    parser.add_argument("--stress-duration", type=int, default=10, help="压力测试时长(秒)")
    parser.add_argument("--stress-rate", type=int, default=1000, help="压力测试目标帧率/秒")

    parser.add_argument("--mode", default="random",
                        choices=["random", "angle_sweep", "cw_sweep", "combined", "oscillation"],
                        help="参数生成模式: random(默认)|angle_sweep|cw_sweep|combined|oscillation")
    parser.add_argument("--angle-min", type=float, default=None, help="最小投射角(度)")
    parser.add_argument("--angle-max", type=float, default=None, help="最大投射角(度)")
    parser.add_argument("--cw-min-ratio", type=float, default=0.5, help="最小配重倍率")
    parser.add_argument("--cw-max-ratio", type=float, default=1.5, help="最大配重倍率")
    parser.add_argument("--sweep-steps", type=int, default=10, help="扫描步数/周期")

    args = parser.parse_args()

    if args.stress:
        run_stress_test(args.host, args.port, args.stress_duration, args.stress_rate)
        return

    interval = 2 if args.fast else args.interval
    use_framed = not args.legacy
    run_simulation(args.host, args.port, interval, args.count, use_framed, args.burst,
                   args.corrupt, args.mode, args.angle_min, args.angle_max,
                   args.cw_min_ratio, args.cw_max_ratio, args.sweep_steps)


if __name__ == "__main__":
    main()
