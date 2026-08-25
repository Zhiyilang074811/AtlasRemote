# Atlas Remote - Phase 5.5 Android 真机测试报告

## 目标设备

| 项目 | 规格 |
|------|------|
| 设备 | Redmi Note 13 5G |
| CPU | MediaTek Dimensity 6080 (ARM64) |
| Android | Android 13 / HyperOS |
| 架构 | arm64-v8a |
| 屏幕 | 1080×2400 AMOLED |

## Host 设备

| 项目 | 规格 |
|------|------|
| 设备 | Lenovo Legion Y7000P 2022 |
| CPU | Intel i7-12700H |
| GPU | NVIDIA RTX 3050 Laptop 4GB |
| OS | Windows 11 |

---

## 当前状态

### ✅ 已完成

| 组件 | 状态 | 说明 |
|------|------|------|
| Rust FFI 层 | ✅ | `crates/ffi-android/` |
| Android 源码 | ✅ | `apps/android/` |
| Windows 主机 | ✅ | `atlas-host.exe` |
| Windows 客户端 | ✅ | `atlas-client.exe` |
| 单元测试 | ✅ | 22 tests passed |
| Release 构建 | ✅ | 编译成功 |

### ⏳ 待完成

| 组件 | 状态 | 阻塞原因 |
|------|------|----------|
| Android SDK | ⏳ | 未安装 |
| ARM64 .so | ⏳ | 需 SDK 构建 |
| APK | ⏳ | 需 SDK + Gradle |
| 真机测试 | ⏳ | 需 APK |

---

## 构建步骤

### 1. 安装 Android SDK

**方法 A: Android Studio (推荐)**
1. 下载: https://developer.android.com/studio
2. 安装后 SDK 路径: `C:\Users\支一郎\AppData\Local\Android\Sdk`

**方法 B: 命令行工具**
```powershell
# 下载命令行工具
$url = "https://dl.google.com/android/repository/commandlinetools-win-11076708_latest.zip"
Invoke-WebRequest -Uri $url -OutFile "C:\Android\cmdline-tools.zip"

# 解压
Expand-Archive "C:\Android\cmdline-tools.zip" -DestinationPath "C:\Android\" -Force

# 安装 SDK 组件
$env:ANDROID_HOME = "C:\Android\sdk"
& "C:\Android\cmdline-tools\latest\bin\sdkmanager.bat" ^
  "platform-tools" ^
  "build-tools;34.0.0" ^
  "platforms;android-34" ^
  "ndk;26.1.10909125"
```

### 2. 配置环境变量
```powershell
$env:ANDROID_HOME = "C:\Android\sdk"
$env:NDK_HOME = "C:\Android\sdk\ndk\26.1.10909125"
$env:JAVA_HOME = "D:\Program Files\Java\jdk1.8.0_261"
```

### 3. 构建 ARM64 .so
```powershell
cd D:\桌面\weclaw\AtlasRemote
cargo build --release --target aarch64-linux-android -p atlas-ffi-android
```

### 4. 部署 .so
```powershell
New-Item -ItemType Directory -Force -Path "apps\android\app\src\main\jniLibs\arm64-v8a"
Copy-Item "target\aarch64-linux-android\release\libatlas_ffi_android.so" `
  "apps\android\app\src\main\jniLibs\arm64-v8a\"
```

### 5. 构建 APK
```powershell
cd apps\android
.\gradlew.bat assembleRelease
```

---

## 测试流程

### 1. 连接设备
```powershell
# 开启开发者模式和 USB 调试
adb devices
```

### 2. 安装 APK
```powershell
adb install app\build\outputs\apk\release\app-release.apk
```

### 3. 启动测试
1. 打开 Host: `cargo run -p atlas-host`
2. 打开 Android 应用
3. 输入 Host IP: `192.168.x.x:8080`
4. 点击连接
5. 测试鼠标/键盘控制

### 4. 收集指标
- 连接时间
- FPS
- 延迟
- 带宽
- 电量消耗

---

## 文件位置

| 组件 | 路径 |
|------|------|
| Rust FFI | `crates/ffi-android/` |
| Android 源码 | `apps/android/` |
| 构建指南 | `docs/ANDROID_BUILD_GUIDE.md` |
| Host 可执行文件 | `target/release/atlas-host.exe` |
| Client 可执行文件 | `target/release/atlas-client.exe` |

---

## 下一步

1. **安装 Android SDK** (需用户操作)
2. **运行构建脚本**
3. **真机测试验证**
4. **生成性能报告**



