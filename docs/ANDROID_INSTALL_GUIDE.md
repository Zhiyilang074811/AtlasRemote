# Atlas Remote - Android 完整安装指南 (D 盘)

## 当前状态

| 组件 | 状态 |
|------|------|
| Java JDK 1.8 | ✅ D:\Program Files\Java\jdk1.8.0_261 |
| Rust 1.97.1 | ✅ 已安装 |
| Android SDK 工具 | ⚠️ 下载超时 |
| Android Studio | ⚠️ 下载超时 |

---

## 方案 1：使用国内镜像下载（推荐）

### 步骤 1：下载 Android Studio
访问清华镜像：https://mirrors.tuna.tsinghua.edu.cn/meta/cdimage/android-studio/
选择最新版本，下载完整安装包

### 步骤 2：安装到 D 盘
1. 运行安装程序
2. 选择安装路径：`D:\Android\AndroidStudio`
3. SDK 安装路径：`D:\Android\sdk`

### 步骤 3：配置环境变量
```powershell
# 添加到用户环境变量
[Environment]::SetEnvironmentVariable("JAVA_HOME", "D:\Program Files\Java\jdk1.8.0_261", "User")
[Environment]::SetEnvironmentVariable("ANDROID_HOME", "D:\Android\sdk", "User")

# 刷新当前会话
$env:JAVA_HOME = "D:\Program Files\Java\jdk1.8.0_261"
$env:ANDROID_HOME = "D:\Android\sdk"
$env:PATH = "$env:ANDROID_HOME\platform-tools;$env:ANDROID_HOME\tools;$env:PATH"
```

---

## 方案 2：使用命令行工具（轻量版）

### 下载 SDK 命令行工具
1. 访问：https://developer.android.com/studio#command-tools
2. 下载 Windows 版本
3. 解压到：`D:\Android\cmdline-tools`

### 安装 SDK 组件
```powershell
$env:ANDROID_HOME = "D:\Android\sdk"
& "D:\Android\cmdline-tools\bin\sdkmanager.bat" `
  "platform-tools" `
  "build-tools;34.0.0" `
  "platforms;android-34" `
  "ndk;26.1.10909125"
```

---

## 方案 3：使用已存在的文件

### 当前已下载
- `D:\桌面\android-studio.zip` (214 MB)
- `D:\桌面\gradle.zip`

### 手动解压
```powershell
# 解压 Android Studio
Expand-Archive -Path "D:\桌面\android-studio.zip" -DestinationPath "D:\Android\AndroidStudio" -Force

# 解压 Gradle
Expand-Archive -Path "D:\桌面\gradle.zip" -DestinationPath "D:\Android\Gradle" -Force
```

---

## 构建 APK 步骤

### 1. 配置环境变量
```powershell
$env:JAVA_HOME = "D:\Program Files\Java\jdk1.8.0_261"
$env:ANDROID_HOME = "D:\Android\sdk"
$env:PATH = "$env:ANDROID_HOME\platform-tools;$env:PATH"
```

### 2. 使用 Android Studio 构建
1. 打开 Android Studio
2. File → Open → `D:\桌面\weclaw\AtlasRemote\apps\android`
3. 等待 Gradle sync
4. Build → Build Bundle(s) / APK(s) → Build APK(s)

### 3. 使用命令行构建
```powershell
cd D:\桌面\weclaw\AtlasRemote\apps\android
.\gradlew.bat assembleRelease
```

### 4. 安装到手机
```powershell
adb install app\build\outputs\apk\release\app-release.apk
```

---

## 真机测试

### 连接设备
```powershell
# 开启手机开发者模式和 USB 调试
adb devices
```

### 运行测试
```powershell
# 启动 Host
cd D:\桌面\weclaw\AtlasRemote
.\target\release\atlas-host.exe

# 另一台设备运行 Client
# 或使用 Windows Client 测试
.\target\release\atlas-client.exe 192.168.x.x:8080
```

---

## 目标配置

| 参数 | 值 |
|------|-----|
| Host | Lenovo Legion Y7000P |
| Client | Redmi Note 13 5G |
| 分辨率 | 1920×1080 |
| 帧率 | 30 FPS |
| 编码 | H264 NVENC |
| 传输 | TCP (当前) → QUIC (最终) |
