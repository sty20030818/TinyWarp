# TinyWarp L3C4：内置 Agent CLI 的 JSON 过滤引擎裁剪

2026-11-11，基于 C3 `6497d535d`。本批继续裁剪专用依赖，已在 Warp Preview 完成构建、定向回归和严格检查；独立开发候选已通过隔离原生冒烟检查。

## 范围与共用边界

`jaq-all`／`jaq-json` 只由共用 CLI 库的 jq 编译器和应用的 Agent SDK 输出过滤使用。TinyWarp 继续保留 `warp_cli`，因为终端、本地控制、补全、云数据模型仍引用其共用参数类型；本批只拆内置 `--jq` 引擎，外部 shell 中的 `jq`、Codex、Claude 等正常 CLI 不依赖它。

- `warp_cli` 的 jq 引擎由可选 `jq` feature 启用，库的独立默认继续启用；workspace 引用关闭默认，由实际应用选择。
- 应用完整默认新增 `agent_cli_jq`，同时启用应用的过滤输出和 `warp_cli/jq`；TinyWarp 的参数仍为 `--no-default-features --features tinywarp_gui`。原版 GUI／TUI 通过完整应用默认保留原行为。
- 保留 `JsonOutput` 的公共形状。精简版无法构造已编译的 `JqFilter`，`--jq` 在参数解析时返回明确的能力不可用错误，不静默输出未过滤内容。若其他 Cargo 消费者统一启用了共用库的 jq，精简应用仍明确拒绝未编译的输出过滤能力。
- 普通 JSON／NDJSON／表格输出和 stdout 非阻塞标志恢复／错误处理保留；完整 jq 的标量解包、对象格式、流式输出及运行时错误传播沿用原逻辑。
- 无设置字段、数据库、持久化数据结构或文件迁移；`JqFilter` 是运行时参数类型，没有持久化格式。依赖边界脚本新增 jaq 专用库，防止回流。

当前 macOS arm64 产品 normal／build 去重 package 身份从 915 项降为 894 项，无新增，移除 21 项：`jaq-all`、`jaq-core`、`jaq-fmts`、`jaq-json`、`jaq-std`、`arraydeque`、`ciborium-io`、`ciborium-ll`、`codesnake`、`half`、`hashlink 0.10`、`hifijson`、`jiff 0.2`、`num-bigint`、`num-integer`、`regex-bites`、`saphyr-parser`、`self_cell`、`toml-span`、`typed-arena`、`xmlparser`。完整 GUI 默认的 975 项依赖集合与本批开始前一致。仍有消费者的其他版本依赖继续保留。

## 验证与交付

全部源码编译、单元回归、严格检查及组包从 Warp Preview 的独立构建窗口执行，真实父进程链留证。使用 Rust 1.92，应用沿用 opt0 开发覆盖，依赖缓存继续复用，不作为正式优化构建或性能对比。

- 共用 CLI 参数库：精简组合 286 项、完整组合 290 项全量单元测试通过，包含无引擎时明确拒绝有效／无效 `--jq` 的两项新回归，保留正常参数和无 jq 的默认行为。
- 应用定向回归：精简组合 88 项、完整组合 99 项通过。覆盖普通 JSON／NDJSON 输出、完整 jq 的过滤／格式化及部分输出后错误传播、Agent CLI 选定范围的请求映射、本地设置、旧数据往返、标签／历史与外部 CLI 参数。两种组合合计 763 项测试通过，并非整套应用全部测试。
- CLI 库与应用分别执行精简／完整 `cargo clippy --lib --tests ... -- -D warnings`，四组均通过。初轮精简应用测试包报告两处仅完整 jq 测试使用的导入／辅助函数警告；随后仅补齐这些测试代码的 cfg，未改变产品或测试行为。当前源码由严格检查重新编译验证。
- 真实 TinyWarp 产品依赖边界检查通过；实际候选编译指纹含 `app_defaults`／`tinywarp_gui`，不含 `default`、`voice_input`、`aws_llm_credentials`、`markdown_mermaid`、`agent_cli_jq`。
- 开发候选编译 4 分 19 秒，组包成功。签名、身份、图标、无 App Group、140/140 bundled 资源、19 个 schema 分区及 23 项系统动态依赖检查通过。设置 schema 与 C3 的 JSON 语义一致。
- 新候选的临时副本通过 zsh 就绪、外部 `/bin/cat` stdin／stdout 字节检查、新标签／分屏 shell 数 1 → 2 → 3 检查；设置截图人工查看确认绘制正常。测试实例均正常退出。本批没有重新构建独立 integration runner。
- 候选编译前冻结的 5,795 项源码输入在验证结束时哈希一致；最后只定向格式化五个改动 Rust 文件，按仓库规则不因纯格式化重跑测试或构建。

首次因宿主锁屏而未能在 Preview 启动，解锁后恢复；此前没有开始编译或改动用户数据。候选校验调度器第一次等待构建达到预设 20 分钟限额，日志独立保留；构建结束后重新调度校验，没有重跑构建／测试，也未将该等待超时视为产品测试失败。

## 数据保护与交付

测试使用临时 HOME、专用应用身份和文件访问限制，拒绝真实 TinyWarp／Preview 配置、Application Support、原生偏好、Group Containers 与 `~/.aws`。测试子进程不继承 AWS 环境变量，未读取真实凭据或调用真实登录／云接口。真实数据库只取元信息，没有迁移、重写或删除。

最终复核：原 8 个配置文件哈希一致，原数据库 inode 保留，原 Preview 进程存活，测试残留进程为 0，Cargo.lock 未变。Preview 正常构建窗口产生的会话活动不等于数据库文件内容冻结，未据此声称真实数据库逐字节不变。

候选位于 `.note/candidates/L3C4-dev/release/bundle/osx/TinyWarp.app`；日常 TinyWarp 未自动替换或切换。阶段结束本地提交，不推送。原始证据在 Git 忽略目录 `.note/l3c4/`；不提交用户配置、测试数据库或构建产物。

未做长期内存基准、真实中文 IME／SSH 或已登录外部 Agent CLI 验收；不把编译依赖减少换算成内存或速度收益。
