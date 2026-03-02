# CloudControl-Rust

CloudControl 的 Rust 重写版本 — 基于 WiFi 的手机群控监控 Web 平台。

原始 Python 版本: [CloudControl](https://github.com/ZSJnbu/CloudControl)

## 功能介绍

- 基于 WiFi 无线方式管理 Android 手机
- 手机群控管理，批量操作（点击、滑动、输入）
- 单设备远程控制，实时截图 + 触控
- NIO 风格 WebSocket 通道，低延迟实时通信
- 集成 Web 端 Terminal 控制台，实时下发命令
- 集成文件上传功能，支持 APK 自动安装
- 集成 Inspector，实时获取手机 UI 布局树
- USB / WiFi ADB 设备自动检测与注册
- 支持高并发设备连接

## 相比 Python 版本的改进

| 方面 | Python 版本 | Rust 版本 |
|------|------------|-----------|
| 运行时 | aiohttp + asyncio | actix-web + tokio |
| 类型安全 | 动态类型 | 编译时类型检查 |
| 内存安全 | GC | 所有权系统，零成本抽象 |
| 性能 | 解释执行 | 原生编译，零开销异步 |
| 数据库 | aiosqlite | sqlx（编译时查询验证） |
| 连接池 | 自定义 LRU | moka 高性能缓存 |
| 测试 | pytest | 内置 90 个测试用例 |

## 运行环境

- Rust 1.75+（2021 edition）
- ADB（Android SDK Platform Tools）
- SQLite（内置，无需额外安装）

## 快速开始

### 1. 构建

```bash
cargo build --release
```

### 2. 启动服务

```bash
cargo run
```

服务启动后访问 http://localhost:8000

### 3. 连接设备

**USB 连接（自动）：** 插入 USB 后系统自动检测并注册设备。

**WiFi 连接：**
```bash
# 先通过 USB 开启设备的 tcpip 模式
adb tcpip 5555

# 在 Web 界面使用 WiFi 连接功能，或调用 API：
curl -X POST http://localhost:8000/api/wifi-connect \
  -H "Content-Type: application/json" \
  -d '{"address": "设备IP:5555"}'
```

## 技术栈

| 组件 | 技术 |
|------|------|
| Web 框架 | actix-web 4 (tokio) |
| 数据库 | SQLite (sqlx 0.8) |
| 设备控制 | uiautomator2 (HTTP API) |
| 模板引擎 | Tera |
| 实时通信 | actix-ws (WebSocket) |
| 连接池 | moka (LRU 缓存) |
| HTTP 客户端 | reqwest 0.12 |
| 前端 | UIKit + Vue.js + jQuery |

## 项目结构

```
├── src/
│   ├── main.rs                    # 入口，启动 actix-web 服务
│   ├── lib.rs                     # 库入口，导出所有模块
│   ├── config.rs                  # YAML 配置加载
│   ├── error.rs                   # 统一错误类型
│   ├── state.rs                   # AppState（共享状态）
│   ├── db/
│   │   └── sqlite.rs              # SQLite 数据库操作（字段映射）
│   ├── device/
│   │   ├── adb.rs                 # ADB 命令封装
│   │   ├── atx_client.rs          # ATX Agent HTTP 客户端
│   │   └── atx_init.rs            # ATX Agent 初始化
│   ├── models/
│   │   ├── device.rs              # Device 数据模型
│   │   └── file.rs                # InstalledFile 数据模型
│   ├── pool/
│   │   ├── batch_processor.rs     # 异步批处理器
│   │   ├── connection_pool.rs     # 设备连接池（moka LRU）
│   │   └── screenshot_cache.rs    # 截图缓存 + 请求去重
│   ├── routes/
│   │   ├── control.rs             # HTTP 路由（截图、触控、上传等）
│   │   └── nio.rs                 # NIO WebSocket 通道
│   ├── services/
│   │   ├── phone_service.rs       # 设备生命周期管理
│   │   ├── device_service.rs      # Android 自动化
│   │   ├── file_service.rs        # 文件管理
│   │   └── device_detector.rs     # USB 设备自动检测
│   └── utils/
│       ├── hierarchy.rs           # XML→JSON UI 层级转换
│       └── host_ip.rs             # 本机 IP 获取
├── tests/
│   ├── common/mod.rs              # 共享测试工具
│   ├── test_config.rs             # 配置集成测试
│   ├── test_database.rs           # 数据库集成测试
│   ├── test_services.rs           # 服务层集成测试
│   └── test_server.rs             # E2E HTTP 端到端测试
├── config/
│   └── default_dev.yaml           # 配置文件
├── resources/
│   ├── templates/                 # Tera 页面模板
│   └── static/                    # 静态资源 (CSS/JS/图片)
└── Cargo.toml                     # Rust 依赖配置
```

## 测试

项目包含 90 个测试用例，覆盖三个层次：

```bash
# 运行全部测试
cargo test

# 仅运行单元测试（52 个）
cargo test --lib

# 仅运行集成测试
cargo test --test test_database
cargo test --test test_services
cargo test --test test_config

# 仅运行端到端测试（20 个）
cargo test --test test_server

# 显示详细输出
cargo test -- --nocapture
```

### 测试覆盖

| 类别 | 测试数 | 说明 |
|------|--------|------|
| 单元测试 | 52 | utils、config、models、device、pool、db |
| 集成测试 - 配置 | 3 | 加载真实配置文件 |
| 集成测试 - 数据库 | 8 | 完整 CRUD 生命周期、并发、分页 |
| 集成测试 - 服务 | 7 | PhoneService、FileService |
| E2E 测试 - HTTP | 20 | 页面路由、API、心跳、文件管理、截图 |
| **总计** | **90** | |

## API 概览

| 端点 | 方法 | 说明 |
|------|------|------|
| `/list` | GET | 获取在线设备列表 |
| `/inspector/{udid}/screenshot` | GET | 获取设备截图 (base64 JSON) |
| `/inspector/{udid}/screenshot/img` | GET | 获取设备截图 (JPEG) |
| `/inspector/{udid}/touch` | POST | 发送触控事件 |
| `/inspector/{udid}/input` | POST | 发送文本输入 |
| `/inspector/{udid}/keyevent` | POST | 发送按键事件 |
| `/inspector/{udid}/hierarchy` | GET | 获取 UI 层级树 |
| `/api/wifi-connect` | POST | WiFi 连接设备 |
| `/nio/{udid}/ws` | WS | NIO WebSocket 通道 |
| `/nio/stats` | GET | 性能统计 |

## 配置

服务器配置位于 `config/default_dev.yaml`，主要配置项：
- `server.port` - 服务端口（默认 8000）
- `db_configs` - 数据库配置（SQLite）

## License

MIT
