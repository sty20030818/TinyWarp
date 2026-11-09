# TinyWarp L2：停止内置 AI、Agent 与外围运行

2026-11-10。基于 L1 `1e388f779`，分支 `sty20030818/local-terminal`。用户授权本阶段实现、在 Warp Preview 构建并本地提交，不自动推送。实现、自动验证和正式构建通过；本机启动验收正在等待 macOS 桌面访问授权，未将其写为通过。

## 实现范围

沿用 `local_terminal` 集中策略，仅普通 OSS 桌面和显式专项测试启用。Preview、CLI/SDK（包括 `--gui`）、TUI、daemon 保留原路径，未增加用户开关。

- 原生菜单、命令搜索、快捷键、工作区工具栏、设置页及 URI 入口停用内置 AI／Agent、独立代码编辑器、文件树和相关视图；保留原 action ID、设置 slug、配置枚举与序列化值。未知 action、自定义标签和稳定选择身份保留。
- 停止内置对话发送／恢复、模型目录、配额刷新、自动输入识别、下一命令／被动建议、编排与任务同步、内置语音及对应订阅、轮询、凭据刷新。必要模型保持真实闲置状态，缺少运行能力的请求返回明确错误。
- 旧 AI 偏好不改写。默认 Agent／Cloud／TabConfig／Docker 模式在本地运行时解析为 Terminal，输入始终按 Shell 处理。
- 停用 Warp 对 Agent CLI 的富输入、专用工具栏、自动 composer、增强通知 listener 和建议。外部 CLI 使用普通 PTY。创建本地子进程时不宣告增强协议，并移除继承或覆盖的同名广告变量，不改 shell 配置或全局环境。
- MCP 不发现、启动、恢复或重连；skills 不扫描、不注册 Home／项目 watcher。代码与规则不建立或恢复索引，LSP 安装、探测和启动入口停用。保留旧描述、缓存和数据库行。
- 不构造内置独立代码／Markdown pane；保留终端、搜索和设置共用的 editor、经典 PS1 prompt 编辑。目录新终端、外部编辑器、系统打开文件继续可用。设置文件通过系统打开。
- 附加本地 HTTP 服务不启动；正常 LocalControl、PTY 和 shell IPC 保留。LocalControl 元数据明确报告已停用 surface 不可用，Agent tab／内置编辑器请求返回既有 `UnsupportedAction`；目录新标签仍创建普通终端。

命令分块、本地历史／补全、多标签／分屏、会话恢复、快捷键、普通多面板输入同步、SSH 和外部 CLI 保留。Universal／Classic 输入布局与 `honor_ps1` 偏好不改写。L1 账号／云同步策略继续生效；本阶段不宣称整个应用零网络。

## 数据与兼容

旧不可用 pane 在构造功能视图前转入只读占位 pane，持有原 `LeafContents`，再次 snapshot 返回原描述；保留标签名称和焦点。用户主动关闭标签仍遵循正常关闭语义。普通终端和可用本地设置页正常恢复。

TerminalSnapshot 原 AI 字段独立保留，恢复时不激活对话、LLM 或执行配置。命令块原 `ai_metadata` 字符串保留，运行时按普通终端块显示，后续 SaveBlock 优先使用保留字段。`agent_view_visibility` 已在既有读取路径类型化，因此这里保留已支持的枚举语义与 Option 形态，不承诺未知／损坏 JSON 字节不变。旧 MCP 启动 IDs 只作为元数据保存，不表示服务正在运行。

SQLite 只补充既有编辑器 kind 的读取分支；写端、schema、migrations 不变。旧磁盘中 CustomRouterEditor 与 ExecutionProfileEditor 原本共用 kind，规范解码为 ExecutionProfileEditor，无法还原二者原区分；内存快照分别保留。Settings search_query 原本未持久化，SQLite fixture 使用 None。

没有删除旧表、云对象、AI 历史、凭据、队列、索引文件或配置；没有迁移或清理真实用户数据。未实施 L3 加载范围或 Cargo feature／crate／依赖裁剪，未修改 Cargo.lock。

PTY 参数新增当前构建内部能力字段。JSON 缺字段默认保留原增强能力；二进制 helper 协议要求父子为同一构建，不声称兼容旧 bincode wire。

低优先级的“退出后保存 Tab 偶发消失”问题仍在待办，本阶段不增加 Tab 保存产品功能，也不承诺恢复已退出的 shell／SSH／CLI 进程。

## 验证

翻译模块 8 项、Unix PTY 核心 8 项、应用聚焦回归 294 项、真实隔离 GUI 测试 3 项全部通过，共 313 项。应用／PTY 核心（含测试）、翻译模块、integration runner 和认证／通用核心的严格 Clippy 全部通过，无 warning。Windows 专用分支仅静态复核，本机未编译或运行。正式 release 用时 12 分 39 秒，构建／组包退出码 0；签名、身份、无 App Group 权限、19 个 schema 分区、140/140 bundled 资源及 25 项系统动态依赖检查通过。

首次应用编译发现的三处类型／穷举接线错误已修正。初轮回归中两项新恢复测试因裸 fixture 字节与既有 ANSI／CRLF 规范化结果不同而失败；修正为普通恢复的严格字节基线，并额外核对原中文命令／输出，原 AI 元数据断言保留，产品序列化逻辑未改。随后 294 项全部通过。Clippy 发现的三处嵌套条件已等价合并，保留判断顺序，复跑严格检查通过；按仓库规则未为此重复测试。release 测试编译有上游 cfg 引起的 unused 提示；严格 dev Clippy 无 warning。GUI 用例退出清理有 SQLite 临时文件和 EPIPE 提示，最终原定脚本退出码 0；不将清理提示写成运行步骤失败。

专项 GUI 测试使用独立 OSS AppId、临时 HOME、人工数据库／histfile、内存凭据 Spy，禁用 Foundation App Group 探测；真实 Preview／OSS 数据不作测试 fixture。新增外部 CLI 测试写入旧默认 Agent 设置，验证运行时 Terminal／Shell、设置仍为 Agent；真实 `/bin/sh` fixture 验证协议广告移除、中文 TypedCharacters 加 Enter、stdin／stdout 和退出 0。

该 CLI fixture 不联网，不使用真实 Codex／Claude 账号；TypedCharacters 不等于 IME 合成验收，普通 stdin 也不覆盖完整 TUI、Ctrl-C 或真实 SSH。未运行完整 presubmit、全平台或长期稳定性测试。

一次性脚本、清单、校验和及日志在 Git 忽略的 `.note/l2/`。

## 候选和交付

目标为 `/Users/stonefish/Desktop/Warp/.note/candidates/L2/release/bundle/osx/TinyWarp.app`。正式构建沿用仓库 release profile、Rust 1.92、并发 2，在 Warp Preview 独立标签执行。独立组包保留 L1，不替换 Preview，不覆盖运行中的 OSS；切换前执行 OSS 自身 SQLite 在线备份并等待旧实例退出。

正式构建父进程链确认来自 Warp Preview。切换前旧 L1 只有空闲 shell，无项目子进程；OSS 自身 SQLite 通过只读 online backup 生成本地 0600 权限备份并通过 quick_check。正常退出旧 L1 并等待其根进程及 helper 消失后，打开精确 L2 路径；Preview 原 PID 保持存活。L1 包和备份保留，未恢复／覆盖数据库。

初次 L2 PID 66689 虽存在，但没有 shell 命令执行标记，AX 菜单不可读，不能算启动通过。两次线程采样都定位 initialize_app → AppearanceManager::set_app_icon → NSWorkspace::setIcon_forFile_options → __setattrlist；TCC 元日志明确记录 kTCCServiceSystemPolicyDesktopFolder 的 AUTHREQ_PROMPTING，新 ad-hoc 签名未匹配旧授权。已请求用户处理系统桌面访问弹窗，没有修改权限、代点授权或为环境授权问题改应用源码。已通过只读 AX 精确定位 UserNotificationCenter 中属于 TinyWarp 的桌面授权弹窗，并执行 Raise 将其置前，未点击任何授权按钮。实际 shell、菜单与人工验收结果待该弹窗处理后补齐。

全部必要代码／测试／lint／正式构建后，仅对本阶段 98 个 Rust 文件执行一次 Rustfmt 1.92.0；git diff --check 通过。按仓库要求不因纯格式化重跑测试、Clippy 或构建。本阶段按授权本地提交，不推送；提交号以同阶段 git log 为准。本机运行验收仍待系统授权。

未做同条件性能基准，不报告内存减少、体积缩减或启动提速。下一阶段依据固定 fixture 的重复测量决定进一步拆加载与依赖。
