# TinyWarp L1 交付记录

2026-11-10。基于 H2 提交 `f36e82381`，在 `sty20030818/local-terminal` 继续。状态：实现、聚焦回归、两项隔离 GUI 启动测试、严格 Clippy、正式构建、包检查与基础运行验收通过；真实旧数据逐条核对和视觉布局未穷尽。用户授权阶段结束本地提交，不自动推送。

## 范围与边界

普通 OSS 桌面启动固定进入本地终端，不增加完整版开关。集中策略在 `app/src/local_terminal.rs`，只覆盖 OSS 的 `LaunchMode::App` 和明确启用的专项测试；其他通道、CLI/SDK（含 `--gui`）、TUI 和 daemon 沿用原路径。SDK 会等待认证事件，不能把它的认证半途禁用。

- 初始化顺序保持：本地策略和设置 → 真实空认证状态与必要 provider → 完整 GUI SQLite 读取 → 工作区与终端。首次启动不需要写入 onboarding 完成标记。
- `AuthState::initialize_local` 跳过旧账号、编译期 secret 和测试假用户注入，保持 user/credentials 为 None；其 `persist_action` 固定为 DoNothing，避免上游普通空认证状态的 Remove 语义删除旧凭据。
- 普通本地应用注册不可用的 SecureStorage provider，不连接平台凭据存储；旧存储不变。这也使应用内第三方 AI/BYO 旧密钥不加载，不将其描述为 AI 运行态完全不受影响。测试使用独立内存 Spy。
- 认证、刷新、SSO/登录回调、退出入口和认证事件都在实际所有者处拒绝；不调用清工作区、停进程和删除数据库的退出路径。旧账号链接在日志、浏览器、请求或窗口创建前被截获，显示本地不可用说明。
- 隐藏账号、订阅、团队、邀请、Drive、共享和云设置入口；八类页面不再构造。原 action 注册/ID、设置 enum/slug 和持久化类型保留。旧设置目标安全回到本地 Features，动作执行给出说明。
- 设置同步无订阅/stream/retry；SyncQueue 不启动消费或重建旧任务依赖；对象和团队 manager 不 poll/refresh，listener 不因旧缓存打开 websocket。必要 singleton、缓存、未同步队列保留，不伪造云加载成功。
- 本地 MCP 设置直接打开，不等云 initial-load；云 gallery 自动安装给出不可用说明。默认敏感信息遮蔽规则独立初始化，沿用原一次性标记并保留已有自定义规则。
- 共享在底层 attempt_to_share_session 拦截，覆盖输入事件和旧 bootstrap 回调；拒绝前不改共享状态、不发启动事件。

保留命令执行、命令分块、本地历史/补全、多标签/分屏、会话恢复、布局、快捷键、普通多面板输入同步、SSH 与任意外部 CLI。没有改变 SQLite schema、migration、GUI Full 数据范围或数据身份，不删除缓存、旧表、凭据或用户配置。

AI 控制器、文件树/编辑器、代码索引及进一步加载/编译依赖裁剪留在后续阶段；L1 不代表整个应用零网络请求，也未测量或承诺内存下降。

## 验证方法与当前结果

所有 Cargo 命令在用户 Warp Preview 的独立标签执行，Rust 1.92.0、并发 2、TTY 实时进度、日志和真实退出码。没有退出/替换 Preview 或修改其设置/凭据。

逻辑测试对 warp、测试 runner 对 integration 使用临时命令行 package profile override（opt-level=0、debug=0、split-debuginfo=unpacked、debug-assertions/overflow-checks=false），避免重复优化和 dSYM；正式候选仍使用原 release profile。未修改清单或锁文件。

- 翻译模块 8 项通过，新增三条本地不可用说明，目录共 1094 项。
- 最终应用聚焦回归 168 项通过，含无凭据持久化、认证/退出零副作用、账号 action/slug、旧队列/云缓存不触发请求、共享状态、默认及自定义遮蔽规则、GUI/SDK 边界与 URI 别名。
- URI 首轮发现 teams 路径与设置 slug 不同；复用原 URI 路由映射修复，保留原断言并复验通过。编译同步两处旧 Test 构造的显式非本地字段，没有弱化旧断言。
- 全新无账号真实 GUI 首启已通过：终端 bootstrap、printf 命令与输出、真实 None、User 凭据读写删为零、onboarding 标记未被写入。
- 升级夹具最初缺 shell 历史文件，三个标签和认证检查通过后，历史断言失败。已补完整 shell 历史和 SQLite 元数据，不改生产恢复逻辑，保留原断言并增加目录关联、真实历史搜索和接受到输入框验证；修正后原断言、原活动标签、命令执行、真实历史搜索和接受验证均已通过。
- 翻译模块、应用（含测试）、认证/核心与 integration runner 的严格 Clippy 均通过、无 warning。正式 release、签名/包检查与真实 shell 运行均已通过，具体结果见下方。

两项专项 GUI 测试在初始化前配置 Empty/LegacyUser，使用独立 OSS AppId `dev.warp.WarpL1Integration`、driver 临时 HOME、人工 SQLite/histfile 和内存凭据 Spy；integration 构建明确禁止 Foundation App Group 探测。旧的 integration 默认假用户行为保留。Spy 检查 User 键零读写删和人工旧凭据原字节；HTTP mock/事件回归检验对应 owner 的请求与启动活动，黑洞地址本身不被当作零请求证据。

一次性脚本、源文件校验和日志在 Git 忽略的 `.note/l1/`；不包含真实账号凭据或数据库内容。候选目标为 `.note/candidates/L1/release/bundle/osx/TinyWarp.app`，H2 包保留。已确认旧 H2 无项目任务后正常退出，等待旧 PID 结束才启动 L1，没有并发使用同一 OSS 数据库。

尚未运行完整 presubmit、真实账号联调、全平台回归、长时间稳定性或性能基准。按仓库顺序，在全部必要检查后仅对本阶段 Rust 文件执行最终格式化，再本地提交。

## 正式候选与实际运行

- 候选：`/Users/stonefish/Desktop/Warp/.note/candidates/L1/release/bundle/osx/TinyWarp.app`。正式 release 使用仓库原 profile，Cargo 用时 14 分 10 秒，构建/打包退出码 0；保留上游 cfg 相关编译提示和经典图标，未升级依赖。
- 签名、身份、可执行文件、图标、无 App Group 权限均通过；19 个 schema 分区、140/140 bundled 资源 SHA-256、25 项系统动态依赖通过。
- 构建父进程链确认来自 `/Applications/WarpPreview.app`。旧 H2 只有空闲 shell，正常退出并等待后切换；不强退项目，Preview 未退出或被替换。
- 按完整路径确认 L1 PID，实际 shell 执行 pwd、中文输出、uname -m，marker 父链归属 L1。只读 AX 确认账号与云盘菜单缺席，“窗口”、⌘N、⌘Q 保留。
- 当前屏幕窗口计数为 0，全窗口列表含该进程 6 个窗口；没有据此冒充截图视觉验收。未发送键鼠检查用户真实历史，未读取/逐条核对实际数据库内容。
- 用户指出此前已登录；因此“这次没弹登录框”没有被当作本地模式验收证据。真正的无账号首启、旧凭据不使用和恢复兼容由独立 fixture 验证，真实旧标签/历史仍未获得明确逐项人工确认。
- 升级 fixture 两个问题均已查明并修正：补 shell 历史与 SQLite metadata join；保留原活动标签 2 后显式切到执行 helper 检查的标签 0。原断言全部保留，额外验证真实搜索与普通 Enter 只接受到输入框，没有执行旧历史命令。未验证 Ctrl-R 按键绑定本身。

本阶段仅本地提交，不推送；提交号以同阶段 git log 为准。未做完整工作区测试、真实账号/SSH 联调、全平台回归、长期稳定性或性能基准；AI 控制器和依赖裁剪留在后续阶段。

已在全部必要检查结束后，仅对本阶段 53 个 Rust 文件执行一次最终 Rustfmt 1.92.0 格式化；按仓库要求不因纯格式化重复运行测试/Clippy，提交前执行差异空白检查。
