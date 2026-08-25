# AtlasRemote Architecture

## 系统架构

`
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
`

## ATLS 协议

Atlas Transport Layer Protocol (ATLS) v1

### 帧格式

`
┌──────────┬─────────┬──────────┬────────────┬─────────┬─────────┬──────────┬────────────┬────────┐
│ MAGIC(4) │ VERSION │  TYPE    │ LENGTH(4)  │ WIDTH(4)│ HEIGHT(4)│ TIMESTAMP│  CODEC(2)  │ CRC(4) │
│ 0x414T4C │  0x0001 │ PacketType│ PayloadLen │ Width  │ Height  │ Timestamp │  Codec     │ CRC32  │
└──────────┴─────────┴──────────┴────────────┴─────────┴─────────┴──────────┴────────────┴────────┘
  Header = 30 bytes
`

### 数据包类型

| 类型 | Value | 说明 |
|------|-------|------|
| Frame | 1 | 视频帧 (H.264/BGRA) |
| Input | 2 | 输入事件 (鼠标/键盘) |
| Control | 3 | 控制命令 |
| PairRequest | 4 | 配对请求 |
| PairResponse | 5 | 配对响应 |
| Ping | 6 | 心跳 |
| Pong | 7 | 心跳响应 |

### 编解码器

| Codec | Value | 说明 |
|-------|-------|------|
| BGRA | 0 | 原始位图 |
| H264 | 2 | H.264 视频 |
| H265 | 3 | H.265 视频 |

## 加密方案

`
设备身份: Ed25519 密钥对
密钥交换: X25519 (ECDH)
数据加密: AES-256-GCM
会话派生: HKDF-SHA256
`

## 模块说明

| 模块 | 说明 |
|------|------|
| crates/protocol | ATLS 协议定义和编解码 |
| crates/crypto | 加密工具 (Ed25519, X25519, AES-GCM) |
| crates/capture | 屏幕捕获 (DXGI Desktop Duplication) |
| crates/codec | 视频编码 (NVENC H.264) |
| crates/input | 输入模拟 (SendInput API) |
| crates/session | 加密会话管理 |
| services/web-relay | WebSocket 桥接服务 |
| pps/web | Web 远程桌面客户端 |
