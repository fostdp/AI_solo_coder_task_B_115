# 🏰 古代投石机弹道动力学仿真与攻城效能评估系统

一套完整的全栈系统，用于模拟古代投石机弹道动力学、评估攻城效能、展示三维可视化效果。

## 📐 系统架构

```
                          ┌───────────────────────────────────┐
                          │        Frontend (Nginx + Gzip)    │
                          │   trebuchet_3d.js + ballistic_panel.js │
                          └──────────────┬────────────────────┘
                                         │ HTTP /api + /metrics
                                         ▼
┌─────────────────┐  UDP  ┌──────────────────────┐  mpsc  ┌───────────────────────┐
│  Trebuchet     │ ─────▶ │   udp_receiver       │ ──────▶ │  ballistic_simulator  │
│  Simulator     │       │   (数据采集+帧同步)    │        │   (RK4变步长动力学)    │
└─────────────────┘       └──────────────────────┘        └──────────┬────────────┘
                                                                  mpsc │
                                                                       ▼
                                                              ┌───────────────────────┐
                                                              │   siege_evaluator     │
                                                              │   (弹坑/损伤/评分)     │
                                                              └──────────┬────────────┘
                                                                         │ 写入
                                                                         ▼
                                                              ┌───────────────────────┐
                                                              │     ClickHouse        │
                                                              │  TTL + 物化视图聚合    │
                                                              └───────────────────────┘
          Prometheus ◀── /metrics ──┘
```

## 🏗️ 模块说明

### 后端 (Rust + Axum + Tokio)

| 模块 | 文件 | 职责 |
|------|------|------|
| UDP 采集 | `udp_receiver.rs` | UDP 数据包接收、帧同步校验、SensorEnvelope 封装，推送到 channel |
| 弹道求解 | `ballistic_simulator.rs` | 消费 SensorEnvelope，RK4 变步长积分求解弹道，产出 BallisticEnvelope |
| 攻城评估 | `siege_evaluator.rs` | 消费 BallisticEnvelope，弹坑估算+损伤分级+效能评分，落库存储 |
| HTTP API | `api.rs` | RESTful API + `/metrics` Prometheus 端点 |
| 配置管理 | `config.rs` | 所有模型参数外置（求解器/大气/材料/攻城/优化/UDP/Channel/存储） |
| 存储 | `storage.rs` | 内存环形缓冲区（带上限），可扩展 ClickHouse |
| 指标 | `metrics.rs` | Prometheus 指标埋点（UDP/弹道/攻城/HTTP/存储 五大类） |

### 前端 (Three.js)

| 模块 | 文件 | 职责 |
|------|------|------|
| 3D 场景 | `trebuchet_3d.js` | Three.js 场景、相机、光照、投石机 3D 模型构建 |
| 控制面板 | `ballistic_panel.js` | UI 交互、API 调用、HUD 显示、参数调整 |
| 粒子系统 | `particles.js` | GPU 实例化弹道粒子效果 |

### 模拟器 (Python)

`trebuchet_simulator.py` - 投石机传感器数据模拟器，支持 5 种生成模式：

| 模式 | 说明 |
|------|------|
| `random` | 完全随机（默认） |
| `angle_sweep` | 投射角从 min 扫到 max |
| `cw_sweep` | 配重倍率从 min 扫到 max |
| `combined` | 角度+配重联合扫描（相移） |
| `oscillation` | 正弦振荡模式 |

## 🚀 快速开始

### Docker Compose 一键启动

```bash
docker-compose up -d
```

服务启动后：

| 服务 | 地址 |
|------|------|
| 前端 | http://localhost:80 |
| 后端 API | http://localhost:8080 |
| Prometheus 指标 | http://localhost:8080/metrics |
| ClickHouse HTTP | http://localhost:8123 |
| ClickHouse Native | localhost:9000 |
| UDP 采集 | localhost:9001/udp |

### 本地开发

```bash
# 后端
cd backend
cargo run

# 前端（任选一）
cd frontend
python -m http.server 8080

# 模拟器
cd simulator
python trebuchet_simulator.py --fast --mode combined
```

## 📊 Prometheus 指标

所有指标前缀为 `trebuchet_`：

| 指标名 | 类型 | 说明 |
|--------|------|------|
| `udp_packets_total` | Counter | 接收 UDP 包总数 |
| `udp_frames_valid_total` | Counter | 有效帧总数 |
| `udp_frames_corrupted_total` | Counter | 损坏帧总数 |
| `udp_channel_depth` | Gauge | UDP→Ballistic channel 深度 |
| `ballistic_simulations_total` | Counter | 弹道仿真次数 |
| `ballistic_simulation_duration_seconds` | Histogram | 单次仿真耗时 |
| `ballistic_solver_steps` | Gauge | RK4 求解步数 |
| `siege_assessments_total` | Counter | 攻城评估次数 |
| `siege_assessment_duration_seconds` | Histogram | 单次评估耗时 |
| `http_requests_total` | Counter | HTTP 请求数 |
| `http_request_duration_seconds` | Histogram | HTTP 请求耗时 |
| `trebuchet_active_count` | Gauge | 激活投石机数量 |
| `storage_sensor_buffer_size` | Gauge | 传感器缓冲区大小 |

## ⏳ TTL 数据保留策略

| 数据表 | 保留时间 |
|--------|----------|
| sensor_data | 30 天 |
| ballistics_results | 30 天 |
| siege_assessments | 90 天 |
| siege_assessments_monthly_mv | 永久（聚合视图） |

## 🧪 测试

```bash
cd backend
cargo test
```

5 个核心测试用例：
- `test_basic_ballistics` - 弹道基础计算
- `test_subsonic_compressibility` - 亚音速压缩性修正
- `test_diameter_calculation` - 弹丸直径估算
- `test_siege_assessment` - 攻城损伤评估
- `test_optimize_parameters` - 参数网格优化

## 📁 目录结构

```
.
├── backend/                 # Rust 后端
│   ├── src/
│   │   ├── main.rs          # 入口 + 管道组装
│   │   ├── config.rs        # 配置外置
│   │   ├── metrics.rs       # Prometheus 指标
│   │   ├── udp_receiver.rs  # UDP 采集模块
│   │   ├── ballistic_simulator.rs  # 弹道求解模块
│   │   ├── siege_evaluator.rs      # 攻城评估模块
│   │   ├── ballistics.rs    # 动力学核心
│   │   ├── siege.rs         # 攻城评估核心
│   │   ├── storage.rs       # 存储层
│   │   ├── api.rs           # HTTP API
│   │   └── udp_server.rs    # 旧版 UDP 服务（兼容）
│   ├── Dockerfile           # 多阶段构建
│   └── Cargo.toml
├── frontend/                # 前端
│   ├── js/
│   │   ├── trebuchet_3d.js  # 3D 场景与模型
│   │   ├── ballistic_panel.js  # UI 控制
│   │   └── particles.js     # 粒子系统
│   ├── nginx.conf           # Nginx + Gzip 配置
│   └── Dockerfile
├── simulator/               # Python 模拟器
│   ├── trebuchet_simulator.py
│   └── Dockerfile
├── clickhouse/              # ClickHouse 初始化
│   ├── init.sql             # 建表 + TTL + 物化视图
│   └── config.d/
├── docker-compose.yml       # 一键编排
└── README.md
```

## 🔧 配置参数

所有模型参数集中在 `backend/src/config.rs` 的 `AppConfig` 中：

- `solver` - RK4 变步长求解器参数（重力、初始步长、误差容限）
- `atmosphere` - 大气参数（Sutherland 粘度、空气密度、声速）
- `material` - 材料密度（石材、配重）
- `siege` - 攻城评估参数（K因子、弹坑系数、评分权重）
- `optimizer` - 参数优化器（网格搜索步数）
- `udp` - UDP 服务（绑定地址、魔数、版本、缓冲区）
- `channel` - Tokio channel 容量（两级管道）
- `storage` - 内存缓冲区上限（三类数据）

## 📝 许可证

MIT
