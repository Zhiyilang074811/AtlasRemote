# Atlas Remote - D 盘安装操作指南

## 当前状态

| 组件 | 状态 | 位置 |
|------|------|------|
| Java JDK 1.8 | ✅ 已安装 | D:\Program Files\Java\jdk1.8.0_261 |
| Rust 1.97.1 | ✅ 已安装 | D:\支一郎\.rust |
| Android Studio | ✅ 已下载 | D:\桌面\android-studio.zip (214MB) |
| Gradle | ✅ 已下载 | D:\桌面\gradle.zip |
| Android SDK | ⏳ 待安装 | 需要 Android Studio 或手动下载 |

---

## 操作步骤

### 步骤 1：解压 Android Studio（已完成部分）

```powershell
# 检查解压状态
Get-ChildItem "D:\桌面\Android\android-studio" | Select-Object Name
```

如果缺少文件，手动解压：
```powershell
# 方法 1：右键解压
# 在 D:\桌面\android-studio.zip 上右键 → 解压到 "android-studio"

# 方法 2：使用 7-Zip（如果已安装）
& "C:\Program Files\7-Zip\7z.exe" x "D:\桌面\android-studio.zip" -o"D:\桌面\Android"
```

### 步骤 2：运行 Android Studio 首次配置

```powershell
& "D:\桌面\Android\android-studio\bin\studio64.exe"
```

首次运行时会：
1. 安装 SDK 组件
2. 选择 SDK 路径（建议：`D:\Android\sdk`）
3. 安装模拟器（可选）

### 步骤 3：配置环境变量

```powershell
# 添加到用户环境变量
[Environment]::SetEnvironmentVariable("JAVA_HOME", "D:\Program Files\Java\jdk1.8.0_261", "User")
[Environment]::SetEnvironmentVariable("ANDROID_HOME", "D:\Android\sdk", "User")

# 刷新当前会话
$env:JAVA_HOME = "D:\Program Files\Java\jdk1.8.0_261"
$env:ANDROID_HOME = "D:\Android\sdk"
```

### 步骤 4：构建 APK

```powershell
cd "D:\桌面\weclaw\AtlasRemote\apps\android"

# 方式 1：使用 Android Studio IDE
# 打开 File → Open → 选择 D:\桌面\weclaw\AtlasRemote\apps\android
# Build → Build Bundle(s) / APK(s) → Build APK(s)

# 方式 2：使用命令行（需要 Gradle）
& "D:\桌面\Android\gradle\gradle-8.2\bin\gradle.bat" assembleRelease
```

### 步骤 5：安装到 Redmi Note 13 5G

```powershell
# 开启手机开发者模式和 USB 调试
# 连接 USB 数据线

# 检查设备连接
adb devices

# 安装 APK
adb install "D:\桌面\weclaw\AtlasRemote\apps\android\app\build\outputs\apk\release\app-release.apk"
```

---

## 快速测试（无需 Android）

```powershell
# 终端 1：启动 Host
cd "D:\桌面\weclaw\AtlasRemote"
.\target\release\atlas-host.exe

# 终端 2：启动 Client
.\target\release\atlas-client.exe 127.0.0.1:8080
```

---

## 下载镜像（如果官方源慢）

**Android Studio:**
- 清华镜像: https://mirrors.tuna.tsinghua.edu.cn/meta/cdimage/android-studio/
- 阿里镜像: https://mirrors.aliyun.com/android-studio/

**SDK 命令行工具:**
- 清华: https://mirrors.tuna.tsinghua.edu.cn/android/repository/
- 华为: https://repo.huaweicloud.com/android/repository/

---

## 文件位置汇总

| 项目 | 路径 |
|------|------|
| 项目代码 | D:\桌面\weclaw\AtlasRemote |
| Android 源码 | D:\桌面\weclaw\AtlasRemote\apps\android |
| Android Studio | D:\桌面\Android\android-studio |
| Gradle | D:\桌面\Android\gradle |
| Windows 主机 | D:\桌面\weclaw\AtlasRemote\target\release\atlas-host.exe |
| Windows 客户端 | D:\桌面\weclaw\AtlasRemote\target\release\atlas-client.exe |
