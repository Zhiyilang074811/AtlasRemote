# Atlas Remote - Android 构建指南 (离线版)

## 目标设备

- **Host**: Lenovo Legion Y7000P (RTX 3050 4GB)
- **Client**: Redmi Note 13 5G (ARM64)

---

## 方案 A：使用 Android Studio（推荐）

### 步骤 1：安装 Android Studio
1. 下载: https://developer.android.com/studio
2. 安装时选择 "Standard" 安装
3. 安装完成后 SDK 位于: `C:\Users\支一郎\AppData\Local\Android\Sdk`

### 步骤 2：构建 APK
1. 打开 Android Studio
2. File → Open → 选择 `D:\桌面\weclaw\AtlasRemote\apps\android`
3. 等待 Gradle sync 完成
4. Build → Build Bundle(s) / APK(s) → Build APK(s)

### 步骤 3：安装到手机
1. 手机开启开发者模式和 USB 调试
2. 连接 USB 数据线
3. Android Studio 会自动安装并运行

---

## 方案 B：使用命令行工具

### 前提条件
- Java JDK 1.8+ ✅ 已安装
- Android SDK 需要手动下载

### 下载必要的组件

**方法 1：下载 Android Studio（包含所有工具）**
```
https://developer.android.com/studio#downloads
选择: Windows 版本
安装后 SDK 路径: C:\Users\支一郎\AppData\Local\Android\Sdk
```

**方法 2：仅下载命令行工具**
```powershell
# 下载 SDK 命令行工具
$url = "https://dl.google.com/android/repository/commandlinetools-win-11076708_latest.zip"
# 下载到可访问的目录
# 解压到 C:\Android\

# 安装必要组件
$env:ANDROID_HOME = "C:\Android"
$sdkmanager = "C:\Android\cmdline-tools\bin\sdkmanager.bat"

# 安装平台工具
$sdkmanager "platform-tools"
$sdkmanager "build-tools;34.0.0"
$sdkmanager "platforms;android-34"
$sdkmanager "ndk;26.1.10909125"
```

**方法 3：从其他机器复制 SDK**
```
如果你有另一台已安装 Android Studio 的电脑:
1. 复制整个 SDK 目录
2. 放到 C:\Android\sdk
3. 确保包含: platform-tools, build-tools, platforms, ndk
```

---

## 方案 C：使用预编译 APK（快速测试）

### 临时方案
由于当前无法自动下载，你可以：

1. **使用 Windows 版本测试控制链路**
```powershell
# 终端 1: 启动 Host
cd D:\桌面\weclaw\AtlasRemote
.\target\release\atlas-host.exe

# 终端 2: 启动 Client
.\target\release\atlas-client.exe 127.0.0.1:8080
```

2. **在可联网的电脑构建 APK**
```
1. 将项目复制到可联网电脑
2. 安装 Android Studio
3. 构建 APK
4. 通过微信/QQ/USB 传输到手机
```

---

## 当前项目状态

| 组件 | 状态 |
|------|------|
| Windows Host | ✅ 可运行 |
| Windows Client | ✅ 可运行 |
| Rust FFI | ✅ 编译完成 |
| Android 源码 | ✅ 完整 |
| APK | ⏳ 需要 SDK 构建 |

---

## 快速测试（无需 Android）

```powershell
# 测试完整的远程桌面链路
cd D:\桌面\weclaw\AtlasRemote

# 终端 1: Host
.\target\release\atlas-host.exe

# 终端 2: Client  
.\target\release\atlas-client.exe 192.168.x.x:8080
```

---

## 下一步行动

**立即可做:**
1. 运行 Windows 版本测试控制链路
2. 验证鼠标/键盘控制功能

**需要网络时:**
1. 下载 Android Studio
2. 构建 APK
3. 安装到 Redmi Note 13 5G

**建议:**
- 先在 Windows 上验证完整功能
- 再在手机上测试移动端体验
