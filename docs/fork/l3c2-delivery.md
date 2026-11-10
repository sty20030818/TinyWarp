# TinyWarp L3C2：AWS／Bedrock 专用 SDK 与模型打包裁剪

2026-11-10，基于 C1 `057304a17`。本批按用户要求继续裁剪 AI／Agent／云功能的专用依赖，保留终端共用代码、配置和数据。本批双构建检查、隔离候选和数据保护复核已完成，结果如下。

## 范围与依赖边界

AWS SDK 仅由应用层内置 AI／Bedrock 凭据加载与 STS 交换使用，原有 SDK 类型还经由主动刷新代码向外传递。本批将 SDK 类型收敛在凭据模块内，交换结果继续使用 `ai::api_keys::AwsCredentials` 共用数据类型；终端、历史、快照和凭据配置字段不删除。

- `aws-config`、`aws-credential-types`、`aws-sdk-sts`、`aws-types` 改为可选，由 `aws_llm_credentials` 集中启用。
- 原默认 feature 列表只维护一份，归入 `app_defaults`；完整默认为 `app_defaults + aws_llm_credentials`，TinyWarp 为 `--no-default-features --features tinywarp_gui`，由 `tinywarp_gui` 启用共享能力。
- SDK 缺席时，本地链加载和 STS 交换明确返回不可用错误，不将缺少能力伪装成“未配置凭据”或成功。OIDC 刷新在请求服务端身份 token 前检查能力；专用终端事件／设置订阅不注册，Agent 驱动不启动 Bedrock 后台刷新循环。
- 其他通道完整默认构建保留。workspace 应用依赖允许关闭默认，TUI 显式保留完整默认；integration 的正常默认仍完整，新增精简组合用于实际验证 TinyWarp 分支。
- TinyWarp 的开发、macOS 组包和 OSS app 分发入口传递 `--no-default-features`；分发脚本不再为 TinyWarp 强制启用 `nld_classifier_v3`／`nld_heuristic_v2`。其他通道及独立 CLI／TUI 分发选择保持原路径。

在当前 macOS arm64、normal／build 边去重 package 身份中，C1 的 959 项降为 921 项，移除 38 项，没有新增。完整默认构建的包集合与 C1 一致。`aws-lc` 属于共用 TLS，继续保留；`input_classifier` 的共用输入类型、AI／云模型中的必要数据类型及其他仍有消费者的模块也保留。

本批并未删除整个 `ai`、Agent 或云对象库，不宣称所有 AI 代码已退出二进制。内置 Bedrock 请求在 TinyWarp GUI 二进制中不可用；独立完整构建仍保留原能力。系统 shell 中的外部 `aws`／Codex／Claude CLI 不依赖本次移除的 Rust SDK，产品启动／环境传递代码未改。

## 可持续的构建检查

新增 `script/check_tinywarp_features`，检查真实产品依赖图：语音、AWS 模型 SDK、Candle／tokenizers 不应回流，终端、GUI 与持久化核心必须保留。真实图检查通过；以命令桩注入 SDK 残留／终端核心缺失两种失败场景，均能正确拒绝。

实际 `script/run` 和 `script/macos/run` 在临时影子目录通过 4 组参数转发检查，验证 TinyWarp 传递无默认 feature 参数，旧 GUI／本地完整构建不被误改。三个 shell 脚本语法检查通过。

## 已运行验证

全部编译、Clippy 和候选组包从 Warp Preview 的独立构建窗口执行，父进程链留证。Rust 1.92，复用依赖缓存，应用及 GUI runner 使用既有 opt0 开发覆盖，不做正式优化构建或性能比较。

- 精简构建定向单元回归：57 项通过，包含两项“SDK 缺席时明确失败且不请求凭据”的新回归。
- 完整构建定向单元回归：61 项通过，保留 AWS 错误映射／安全错误消息，并检查空身份 token 在 STS 请求前被拒绝。
- 新编译的精简 GUI runner：`test_local_terminal_first_launch_without_account`、`test_local_terminal_external_cli_ignores_legacy_agent_default`、`test_local_terminal_settings_open_once_and_return_to_terminal` 均退出 0；本批不复用旧 runner 充当新候选回归。
- 精简与完整 GUI 构建的 `cargo clippy --lib --tests ... -- -D warnings` 均通过。
- 开发候选在 Preview 编译及组包退出 0；候选编译 4 分 20 秒。严格签名、身份、图标、无 App Group entitlement、19 个 schema 分区、140/140 bundled 资源及 23 项系统动态依赖检查通过；设置 schema 与 C1 的 JSON 语义完全一致。
- 真实编译指纹含 `app_defaults`／`tinywarp_gui`，不含 `default`／`aws_llm_credentials`／`voice_input`。5,795 项编译前源码输入哈希核对一致。
- 当前候选的临时副本通过真实 zsh 就绪、外部 `/bin/cat` stdin／stdout 字节检查，以及新标签／分屏的 shell 数 1 → 2 → 3 检查。设置截图确认正常绘制；测试实例均正常退出。
- 全部必要验证后，仅定向格式化四个改动 Rust 文件并修正两条构建注释；按仓库规则不因纯格式化重跑测试、Clippy 或构建。

首次精简测试通过后，修正了仅 SDK 才产生的瞬时错误枚举分支警告，再对修正版本重新验证。该错误类型未序列化，不改变旧数据格式。首次记录单列留存，不与最终候选证据混合。

## 保护与交付记录

测试使用临时 HOME、专用身份与文件访问限制，拒绝真实 TinyWarp／Preview 的配置、Application Support、原生偏好、Group Containers 和 `~/.aws`。测试子进程不继承 AWS 环境变量；未读取真实 AWS 配置、密钥或调用真实登录／云 API。真实数据库只取文件元信息。最终复核：原 8 个配置文件哈希一致、原数据库 inode 保留、原 Preview 进程存活、测试残留进程为 0；Cargo.lock 未变。

候选目标为 `.note/candidates/L3C2-dev/release/bundle/osx/TinyWarp.app`；本批生成可审阅的开发候选，日常 TinyWarp 不在自动切换范围。原始构建、测试、依赖图、保护快照和失败记录在 Git 忽略目录 `.note/l3c2/`，不提交用户数据、测试数据库或构建产物。阶段结束本地提交，不推送。

未做长期内存基准、真实中文 IME、真实 SSH 或已登录外部 Agent CLI 验收；不把依赖减少换算成内存／速度收益。
