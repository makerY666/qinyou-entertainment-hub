# 原生主机与构建

Windows 和 Android 共用 `hub-server` Rust 库。`apps/native/src/host.rs` 使用独立 OS 线程与 Tokio 运行时拥有服务，不依赖 WebView 的事件循环。关闭 Windows 窗口会隐藏到托盘，托盘菜单可返回、停服或退出。

Android 的 `HostService` 在 `connectedDevice` 前台服务内通过 JNI 启停相同 Rust 库。服务创建常驻通知、CPU 和 Wi-Fi 锁；用户主动停止后释放锁。服务使用 `START_NOT_STICKY`，进程被终止后需要用户重新打开应用并启动，数据库恢复由共享服务器处理。厂商省电策略仍须真机验证。

## 开发依赖

- Node.js 22+ 与 npm；Rust stable。
- Windows：推荐 Microsoft C++ Build Tools + Windows SDK；本机开发也支持 MinGW GNU 目标。
- Android：JDK 21、SDK platform 37（SDK Manager 包 `platforms;android-37.0`）、Build Tools 37、NDK 29.0.14206865、Rust `aarch64-linux-android` 目标。
- 设置 `JAVA_HOME`、`ANDROID_HOME`、`NDK_HOME`。构建脚本优先使用工作区 `.tools`，不修改全局环境。

```powershell
npm install
npm --prefix web install
npm --prefix web run build
rustup target add aarch64-linux-android
./scripts/native.ps1 android-init
./scripts/native.ps1 android
./scripts/native.ps1 windows
```

`android-init` 生成 Gradle 项目后自动同步 `android-src` 中的 Kotlin 代码与权限。修改 Kotlin 源后运行构建脚本会重新同步。Windows NSIS 包配置 `offlineInstaller`，将 WebView2 离线安装器附带进包，首次打包需要网络下载运行时。APK 发布签名由 Tauri/Gradle 的正式签名配置提供；调试 APK 仅用于测试，不等同于发行包。

## 原生接口

仅内置界面可调用 Tauri IPC：

- `start_host({port?: number})`：默认 18765，返回 `{port, addresses, adminToken}`；已启动时返回当前状态。
- `host_info()`：返回同样的数据，未启动返回 null。地址列表每次查询重新读取，可识别网卡变化。
- `stop_host()`：等待服务器退出。普通局域网网页没有这些权限。
- `set_table_orientation({landscape: boolean})`：Android 进入牌桌时使用应用内 `sensorLandscape`，退出恢复进入前的 Activity 方向；Windows 上安全空操作。不修改系统自动旋转设置。

原生界面通过 `http://127.0.0.1:<port>` 连接服务器，邀请链接使用 `addresses` 内的局域网地址。管理员令牌不得放进分享链接。APK 在 Android 17+ 请求 `ACCESS_LOCAL_NETWORK`，Android 13+ 请求通知权限；通知被拒绝不阻止前台服务启动。Android 禁止系统备份数据库和凭据。

## 局域网排障

- Windows 第一次启动后，系统防火墙提示选择“专用网络”；若亲友无法连接，检查端口和专用网络入站规则。脚本不自动修改防火墙。
- 各设备连接同一 Wi-Fi，关闭访客网络的客户端隔离；安卓热点不要求移动数据开启。
- Android 拒绝局域网权限时给出错误，用户可在系统设置允许后重新启动。
- 端口占用会由服务器反馈，选择另一端口重试；地址改变后重新分享二维码。
- 锁屏至少 30 分钟、切换应用、关闭 WebView、主机进程强制终止、两个厂商手机热点分别属于必须人工实测项目，不用构建成功代替。

## 官方参考

- [Tauri 移动插件与 JNI](https://v2.tauri.app/develop/plugins/develop-mobile/)
- [Android 前台服务类型](https://developer.android.com/develop/background-work/services/fgs/service-types)
- [Android 局域网权限](https://developer.android.com/privacy-and-security/local-network-permission)
- [Tauri Windows 打包配置](https://v2.tauri.app/reference/config/#windowsconfig)

## 本机 Windows 构建入口

本地辅助脚本需要 PowerShell 7.1 或更新版本（`pwsh`）。当前项目目录含中文，GNU linker、NDK 与 Kotlin 的路径处理各有兼容问题。使用已准备的本地工具链时，运行：

```powershell
pwsh -File ./scripts/native-build-local.ps1 -Target both
```

该脚本复制构建源到 `%LOCALAPPDATA%/QinyouHubBuild` 下的英文路径，保留原目录，使用单独的 Cargo 输出目录，生成的文件复制回项目 `artifacts`。`-Target windows` 或 `-Target android` 可单独构建。所有依赖准备好后，生成的应用完全离线运行；构建过程本身可能下载依赖。

Windows 不允许文件符号链接时，脚本在 Rust 库成功编译后改用文件复制，再由 Gradle 正常打包；不会修改 Windows 开发者模式。安卓预览包只移除调试符号，保留 JNI 导出，重新做 16KB 对齐并使用 Android 调试证书签名。正式分发前应配置由用户保管的发布签名证书，不能将调试签名当作发布签名。

## 当前本机验收记录（2026-10-03）

- `cargo fmt -p hub-native -- --check` 与 Windows `cargo check -p hub-native` 通过。
- 原生主机生命周期测试通过：独立线程启动、重复启动、读取状态、停止、原目录重启、重复停止。
- Windows 优化版 EXE 构建成功；隐藏启动后进程响应正常，并创建 WebView2 子进程；测试实例已关闭。
- Windows NSIS 安装程序构建成功，直接检查安装包确认包含 `WebView2Loader.dll` 和 WebView2 离线安装器；未执行系统安装验收。
- Android ARM64 Rust 库、Kotlin 前台服务和 APK 构建成功。验证 `minSdk=28`、`targetSdk=37`、仅 ARM64、前台服务不导出、局域网与唤醒锁权限，JNI 导出存在；ELF LOAD 对齐为 16KB。
- APK 调试签名验证与 16KB ZIP 对齐检查通过。连接手机最初报告 `unauthorized`；用户在手机允许 RSA 后已可读取设备。设备为 Xiaomi 24122RKC7C（miro）、Android 16/API36、1440×3200、密度600；手机 Wi-Fi 192.168.31.38 与电脑192.168.31.118的只读 ping 验证通过（55.3ms、0丢包）。
- 通过 ADB 在手机 Chrome 打开开发页及组合 UI 树读取被工具自动审核拒绝（`blocked by policy`），未执行；锁屏、热点、双厂商兼容性和实际安装尚未因此得到验证。没有更换电脑密钥或修改手机安全设置。
- 第二轮界面 APK 已构建并尝试安装到该手机，但系统返回 `INSTALL_FAILED_USER_RESTRICTED: Install canceled by user`。此次安装没有成功；未修改 USB 安装或系统安全设置，需用户在手机允许本项目安装后继续。
- `artifacts/qinyou-hub-windows-x64-setup.exe` 和 `artifacts/qinyou-hub-android-arm64-preview.apk` 已包含第二轮全手牌、摸牌分离、碰杠副露、离线声音和原生横屏改进；分别从真实 NSIS EXE 与 APK 原生库确认嵌入 `index-B20oKdrI.js`。同次网页构建输出为 `index-CW6VM1e6.css` 与 `core-DV6XEvTN.js`，34 个离线 MP3 路径也在 ARM64 库内确认，校验详情和 SHA256 见 `artifacts/native-both-verification.json`。
- `artifacts/native-preview` 仅为重绘前的旧包归档，请使用顶层新包。APK 仍是调试签名预览版本；上述构建与启动检查不代表真机验收完成。

生成安装包后，可执行 `pwsh -File scripts/native-verify-artifacts.ps1 -Target both`。脚本从 NSIS 提取实际 EXE、从 APK 读取实际 ARM64 库，检查包含当前 `web/dist/index.html` 引用的入口文件名，并记录包 SHA256；Windows 内容检查需要 7-Zip。结果写入 `artifacts/native-both-verification.json`。此检查确认前端版本一致，不替代安装与真机测试。


第三轮两端安装包已重新构建并从实际NSIS应用与APK库确认嵌入 `index-CS6whL57.js`；两端库内均找到与当前bgm.mp3逐字节相同的新曲。Windows应用启动响应正常并创建WebView2子进程，检查后关闭。APK v3调试签名、ZIP与ELF 16KB对齐检查通过。Windows包256,557,143字节，APK91,165,645字节；详见 artifacts/native-both-verification.json 与 SHA256SUMS.txt。上述验证不替代真机安装、锁屏与热点验收。

第四轮横屏/邀请入口包已完成，Windows与APK实际嵌入 `index-CH8X86iP.js` / `index-D0ts7xOU.css`，分别为256,597,324与91,169,741字节。Windows GUI启动、APK调试签名/ZIP对齐和实际包入口检查通过；实体手机方向锁定尚未实测。最新校验清单在 artifacts/SHA256SUMS.txt。

2026-10-04 斗地主叠牌视听更新：Windows 安装包 257,720,294 字节，安卓 ARM64 APK 93,062,093 字节，实际包内应用均确认包含 `index-DGmwM5Ok.js`，对应样式 `index-BnOPvlHJ.css`。实际 NSIS 应用、APK ARM64 库和运行中 18765 服务器均逐字节包含相同的 66 条斗地主 MP3 与背景音乐。APK 调试签名和 ZIP 16KB 对齐检查通过。校验记录见 `artifacts/native-both-verification.json`、`artifacts/doudizhu-audio-verification.json`；安装包已重新生成，本轮未进行实体手机安装验收。
