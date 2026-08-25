# Atlas Remote - 硬件基准报告

## Host 设备

| 项目 | 规格 |
|------|------|
| 型号 | Lenovo Legion Y7000P |
| CPU | Intel i7-12700H |
| GPU | NVIDIA RTX 3050 Laptop GPU 4GB |
| 显存 | 4GB GDDR6 |
| 内存 | 32GB DDR4 |
| 系统 | Windows 11 |
| 驱动 | 560.94, CUDA 12.6 |
| DirectX | DirectX 12 |

## Client 设备

| 项目 | 规格 |
|------|------|
| 型号 | Redmi Note 13 5G |
| CPU | MediaTek Dimensity 6080 |
| 架构 | ARM64 (aarch64) |
| Android | 13 / HyperOS |
| 屏幕 | 1080×2400 AMOLED |
| 网络 | WiFi 5/6, 5G |

---

## 目标配置

| 参数 | 值 |
|------|-----|
| 分辨率 | 1920×1080 |
| 帧率 | 30 FPS |
| 编码 | H264 NVENC |
| 码率 | 5-8 Mbps |
| 颜色 | NV12 |
| 传输 | 当前 TCP → 最终 QUIC |
| Android 解码 | MediaCodec 硬解 |

---

## 性能目标

| 指标 | 目标 |
|------|------|
| 视频延迟 | <80ms |
| 输入延迟 | <50ms |
| CPU 占用 | <20% |
| GPU 占用 | <30% |
| 电池消耗 | 可长期运行 |

---

## 测试拓扑

```
Lenovo Legion Y7000P (Host)
        |
        | WiFi LAN
        |
Redmi Note 13 5G (Client)
```

---

## 功能清单

- [x] 桌面采集 (DXGI)
- [x] 视频编码 (NVENC)
- [x] 网络传输 (TCP/QUIC)
- [x] 输入控制 (鼠标/键盘)
- [x] 设备配对
- [x] Windows 客户端
- [ ] Android 客户端 (待 SDK)
- [ ] WAN 中继 (未来)

