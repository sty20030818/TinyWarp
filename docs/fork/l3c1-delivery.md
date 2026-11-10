# TinyWarp L3C1：移除内置语音构建链

2026-11-10。用户确认暂停专项性能定位，直接以功能与依赖裁剪推进。首批只处理边界清晰的内置语音，不修改终端 Rust 逻辑、数据库格式或用户设置。

## 实现与实际变化

`app/Cargo.toml` 的原 `gui` feature 仅额外启用 `voice_input`，GUI 本体不由此 feature 控制。新增专用 `tinywarp_gui` 标记，不引入语音依赖；保留原 `gui`，避免改变上游其他通道的构建能力。OSS 开发入口 `script/run` 和 macOS 分发脚本的 OSS app 分支均选择新标记，CLI／TUI 分支不改。

旧语音配置键、schema、序列化类型和仓库内语音 crate 保留；它们仍供旧构建使用。不是从整个 workspace 或 Cargo.lock 删除这些包，也不只是隐藏按钮。

在 macOS arm64、默认 features 加对应 GUI 标记、normal／build 边的去重 package 图中：

| 指标 | 原 `gui` | `tinywarp_gui` |
| --- | ---: | ---: |
| 正常与构建 package 数 | 975 | 959 |
| 主程序直接系统动态依赖 | 25 | 23 |

移出 TinyWarp 图的 16 个包为：`voice_input`、`cpal`、`coreaudio-rs`、`dasp_sample`、`hound`、`mach2 0.6.0`、`objc2-audio-toolbox`、`objc2-core-audio`、`objc2-core-audio-types`、`rubato`、`realfft`、`rustfft`、`num-complex 0.4.6`、`primal-check`、`strength_reduce`、`transpose`。没有新增依赖，Cargo.lock 未变。这里按 package 身份计数，同名其他版本可能仍被其他消费者保留。

与原开发候选相比，新主程序不再直接链接 AudioToolbox 与 CoreAudio；不据此声称系统框架不会间接加载它们。真实编译指纹含 `tinywarp_gui`，不含 `voice_input`，验证了构建产物确实采用裁剪后的选择。

## 构建与验证

构建在 Warp Preview 的独立终端执行，父进程链已留证。使用 Rust 1.92、release 依赖缓存，应用包仍采用既有 opt0 快速开发覆盖；编译用时 4 分 21 秒，构建／组包退出 0。首次离线依赖图检查缺少锁定包 `addr 0.15.6`，随后联网补齐已有锁定依赖，没有升级版本。

候选为 `.note/candidates/L3C1-dev/release/bundle/osx/TinyWarp.app`。这是开发包，不是正式优化包，不能据此报告内存、速度或包体积改善。未替换日常 TinyWarp。

- 实际开发脚本在临时目录用平台／命令桩检查：OSS 选择 `tinywarp_gui`，其他本地通道仍选 `gui`，release／dont-open 参数正确转发。两个 shell 脚本语法检查通过。
- 候选身份、可执行文件、图标、严格签名、无 App Group entitlement、19 个 schema 分区、140/140 bundled 资源和 23 项系统动态依赖检查通过。
- 候选副本使用唯一测试身份、临时 HOME 与内核文件限制；真实 zsh 就绪，外部 `/bin/cat` fixture 的 stdin／stdout 字节核对通过，设置页截图确认正常绘制；另以正常终端启动，抬起测试主窗口后确认新标签／分屏依次产生第 2／第 3 个真实 shell。未观察到受保护配置／数据库被测试进程打开，测试包正常退出。
- 复用已有缓存 integration runner，执行 `test_local_terminal_external_cli_ignores_legacy_agent_default` 和 `test_local_terminal_settings_open_once_and_return_to_terminal`，均退出 0。应用 Rust 源码未变；这是旧 runner 的辅助回归，不冒充本轮候选的 GUI 测试二进制。
- 正常构建的 warpui_core 8 项、warp_terminal 2 项 warning 与 L3A 构建记录一致，无新增应用 warning；未为构建配置变更重跑全工作区测试、Clippy 或正式 release。

系统键盘自动化曾输出非预期文本或超时，冷启动设置页后的原生菜单自动化也未确认新 shell；失败记录保留，不计为通过。没有据此修改产品代码。中文 IME、真实 Codex／Claude、完整 TUI 和真实 SSH 未在新候选复验。

## 测试样本更正

复用的 L3A fixture 把透明度百分比写成了 `1.0`，而真实 schema 要求整数；共享 fixture 已改为 `100`。15 个固定配置项通过本次候选 schema 的类型／范围校验，新的设置截图不再有无效配置警告。只修改测试样本，不修改用户配置；历史基线已加注，旧数字不回填，也不重做专项基准。

## 数据保护与后续

真实 TinyWarp／Preview 配置、Application Support、原生偏好及 Group Containers 受测试进程内核文件限制保护；数据库只取文件元信息。阶段结束复核：原 8 个配置文件哈希一致，原数据库及辅助文件 inode 保留，Preview 原进程存活，测试残留进程为 0；记录位于 `.note/l3c1/`。测试 HOME、失败记录、构建产物与诊断日志不提交。

下一批继续按模块解除 AI／Agent 与云功能的专用消费者，关联初始化／数据加载一起处理；遇到终端共用代码就保留或只拆专用部分，不以删除整个 crate 为硬目标。每批先验证依赖确实退出构建及终端基本可用，再本地提交；不自动推送。
