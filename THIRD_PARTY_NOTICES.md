# 第三方材料与许可

本项目原创内容的权利保留声明见 [LICENSE](LICENSE)。该声明不替代、不缩减第三方材料原有的许可，也不将第三方开源组件改为专有软件。

## 仓库内的第三方材料

| 材料 | 来源与许可 | 仓库中的声明 |
| --- | --- | --- |
| Noto Sans SC 字体 | Google Noto，SIL Open Font License 1.1 | [OFL 全文](web/public/licenses/Noto-Sans-SC-OFL.txt) |
| Noto Serif SC 字体 | Google Noto，SIL Open Font License 1.1 | [OFL 全文](web/public/licenses/Noto-Serif-SC-OFL.txt) |
| Gradle Wrapper 启动脚本及包装器 | Gradle，Apache License 2.0 | `apps/native/gen/android/gradlew`、`gradlew.bat` 文件头及 [Apache-2.0 全文](https://www.apache.org/licenses/LICENSE-2.0) |

## 通过包管理器安装的依赖

React、Radix UI、Motion、Lucide、Tauri、Vite、TypeScript，以及 Rust / Android 依赖由各自权利人授权。本仓库保留 npm 与 Cargo 锁文件，用于确定依赖版本；`node_modules`、Rust 工具链、构建缓存和安装包不随本源码仓库发布。

直接依赖见 [根 package.json](package.json)、[前端 package.json](web/package.json) 和各 crate 的 `Cargo.toml`。npm 锁文件中的第三方 `license` 字段描述对应依赖，不代表本项目采用相同许可。完整许可、版权及 NOTICE 要求以对应版本上游文件和安装包内声明为准；分发二进制时需另行保留相关材料。

## 音频与项目资源

背景音乐由项目脚本合成，麻将牌面由项目 SVG 代码绘制，原创部分适用根目录 [LICENSE](LICENSE)。离线中文语音由 Windows Microsoft Huihui Desktop TTS 生成；其生成来源见 [声音资源说明](web/public/audio/README.md)。本项目不授予 Microsoft 语音引擎、操作系统或其他第三方产品的许可。
