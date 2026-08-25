# AtlasRemote

> **免费、不限流、自托管、跨平台远程桌面控制系统**
> Rust 核心 · WebRTC-ready · 完全开源

<div align="center">

![Rust](https://img.shields.io/badge/Rust-2026+-orange)
![License](https://img.shields.io/badge/License-MIT-blue)
![Status](https://img.shields.io/badge/Status-Alpha-yellow)

</div>

## 功能

- [x] Windows 主机端（DXGI 捕获 + H.264 编码）
- [x] ATLS 二进制协议（加密帧传输）
- [x] Ed25519 + X25519 + AES-GCM 安全加密
- [x] Web 浏览器远程桌面（已完成）
- [ ] Android 客户端（开发中）
- [ ] QUIC 传输层（开发中）
- [x] WebSocket 中继服务器（已完成）
- [ ] 设备 ID 系统（开发中）

## 架构

\\\
                    ┌──────────────────────────────┐
                    │         Web Client           │
                    │    (Vue 3 + Vite + TS)       │
                    │    ws://host:8080            │
                    └──────────────┬───────────────┘
                                   │ WebSocket
                    ┌──────────────▼───────────────┐
                    │       Web Relay Server        │
                    │     (warp + tokio)            │
                    │     ws://host:8080           │
                    └──────────────┬───────────────┘
                                   │ TCP
                    ┌──────────────▼───────────────┐
                    │        Windows Host           │
                    │   (DXGI + H.264 + ATLS)       │
                    │   tcp://host:9090             │
                    └──────────────────────────────┘
\\\

## 快速开始

### 前提条件

- Rust 1.75+ (rustup)
- Windows SDK (用于 DXGI 捕获)
- Node.js 20+ (用于 Web Client)

### 构建主机端

\\\ash
cargo build -p atlas-host --release
\\\

### 启动 Web Client

\\\ash
cd apps/web
npm install
npm run dev
# 访问 http://localhost:3000
\\\

### 启动 WebSocket Relay

\\\ash
cargo run -p atlas-web-relay -- 9090 8080
\\\

## 项目结构

\\\
AtlasRemote/
├── crates/           # Rust 核心库
│   ├── protocol/     # ATLS 协议定义
│   ├── crypto/       # 加密 (Ed25519/X25519/AES-GCM)
│   ├── capture/      # 屏幕捕获 (DXGI)
│   ├── codec/        # 视频编码 (H.264/NVENC)
│   ├── input/        # 输入模拟 (鼠标/键盘)
│   ├── session/      # 加密会话管理
│   ├── auth/         # 设备认证 (Device ID + Pin Code)
│   ├── transport/    # 传输层
│   └── ffi-android/  # Android FFI
├── apps/
│   ├── host/         # Windows 主机
│   ├── client/       # Android 客户端
│   └── web/          # Web 远程桌面 (Vue 3)
├── services/
│   ├── relay/        # 中继服务器
│   ├── signaling/    # 信令服务器
│   └── web-relay/    # WebSocket 桥接
└── docs/             # 开发文档
\\\

## 技术栈

| 组件 | 技术 |
|------|------|
| 主机端 | Rust + Windows SDK (DXGI, NVENC) |
| Web 客户端 | Vue 3 + Vite + TypeScript |
| 协议 | ATLS (Atlas Transport Layer Protocol) |
| 加密 | Ed25519 + X25519 + AES-GCM |
| 传输 | TCP (当前) → WebSocket → QUIC (规划) |
| 视频编码 | H.264 (NVENC 硬件加速) |

## 安全特性

- **端到端加密**: 每次连接使用独立的 X25519 会话密钥
- **设备认证**: Ed25519 签名验证设备身份
- **AES-GCM**: 所有数据传输使用 AES-GCM 加密
- **配对码**: 6 位 PIN 码配对，5 分钟过期

## 开发路线图

### Phase 1: 基础体验 (S-009)
- [x] Web Client 基础框架
- [x] WebSocket 中继服务器
- [ ] Windows Installer
- [ ] 设备 ID 系统
- [ ] 完整 ATLS 协议桥接

### Phase 2: 通信套件 (S-010)
- [ ] 文件传输
- [ ] 剪贴板同步
- [ ] 远程聊天

### Phase 3: 性能优化 (S-011)
- [ ] 1080p @ 60fps
- [ ] <50ms 延迟
- [ ] ABR 自适应码率

### Phase 4: v1.0 开源发布 (S-012)
- [ ] 完整文档
- [ ] Docker 部署
- [ ] CI/CD 流水线

## 贡献

欢迎 PR 和 Issue！请阅读 [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md)

## 许可证

MIT License - 详见 [LICENSE](LICENSE)

---

**Made with ❤️ by Atlas Remote Team**
