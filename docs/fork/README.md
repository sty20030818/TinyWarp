# TinyWarp

## 已确认方向（2026-11-09）

先供自己的 Mac 使用，目标是中文、精简和降低内存占用，沿用 Warp，界面更干净紧凑。先编译并运行未修改的 OSS 基线，与现有 Warp Preview 并存，保留 Preview 及现有数据。

保留命令分块、历史搜索、本地补全、多标签、分屏、会话恢复、快捷键，以及运行项目和任意 CLI 的能力。计划移除内置 AI / Agent、Warp 为 Agent CLI 提供的增强界面、内置代码编辑器与文件树、Warp Drive、登录、云同步和团队协作入口；L1 已实施账号与云同步停用，L2 已实施 AI 与外围运行停用；各阶段验证状态见下方记录。

SSH 暂时保留。用户认为也可以移除，但如果对性能影响不大，不必为了精简而删除；以后依据实际测量决定。

## 当前技术方案

用户已确认继续基于 master，先汉化，再逐步精简登录、账号、同步与 AI。技术方案已确认，项目命名 TinyWarp；每个阶段完成验证后提交一次，不自动推送。具体实现见 [中文本地终端技术方案](2026-11-09-technical-plan.md)，H1/H2 汉化与 L1 本地启动已完成并验证，按阶段进行本地提交；L2 的 AI/外围精简已通过自动验证、正式构建与本机基础运行检查。

## 源码与构建基线

- 本地源码：`/Users/stonefish/Desktop/Warp`
- 个人 fork：<https://github.com/sty20030818/warp>
- upstream：<https://github.com/warpdotdev/warp>
- 起始提交：`abb3fe33e004f7b1de540fecd6e3eec20dbc1883`
- 开发分支：`sty20030818/local-terminal`，从上述 `master` 基线创建。2026-11-09 起步时在线复核，本地、个人 fork 与官方上游 `master` 的 HEAD 均为上述提交，时间为北京时间 2026-11-07 03:25:35。
- `master` 是开发主线，Preview 是构建/发布通道；官方另有 `preview_release/<version>` 形式的发布分支。本轮产物采用 `app/src/bin/oss.rs` 的 OSS 通道，不是按 `app/src/bin/preview.rs` 的 Preview 通道打包，也未完成与本机 Preview 二进制的精确 commit 映射。
- 依据：[上游 master](https://github.com/warpdotdev/warp/tree/master)、[Preview 入口](https://github.com/warpdotdev/warp/blob/abb3fe33e004f7b1de540fecd6e3eec20dbc1883/app/src/bin/preview.rs)、[OSS 入口](https://github.com/warpdotdev/warp/blob/abb3fe33e004f7b1de540fecd6e3eec20dbc1883/app/src/bin/oss.rs)。
- OSS 身份：`dev.warp.WarpOss`，URL scheme `warposs`，自定义配置在 `~/.warp-oss`。
- 现有 Preview：`/Applications/WarpPreview.app`，身份 `dev.warp.Warp-Preview`，URL scheme `warppreview`。
- 基线阶段已完成未修改 OSS 的编译运行。TinyWarp 首批只改显示品牌和 H1 中文界面，继续复用 OSS 数据身份；不替换 Preview、不修改其配置或迁移/清理现有数据，不自动推送。构建按用户要求在 Preview 的独立新标签中执行。
- 原始调研与证据：[起步研究](2026-11-09-research.md)。本文的后续用户决定优先于研究中的早期建议。

## 待办：本地终端免登录与移除账号入口

- 用户补充（2026-11-09）：首次进入遇到登录引导。这个 fork 只作为本地终端，不需要 sync，也不需要账号系统。
- 期望：首次启动直接进入可用的本地终端，无需登录或手动选择跳过；没有账号也不影响本地命令执行和本地数据保存。
- 允许分阶段处理：可先绕过登录引导并隐藏登录、账号和同步入口，之后结合精简范围移除相应运行逻辑；仅藏按钮不能让流程仍停在登录页。
- 属于已确认的后续精简阶段，H1 不实施。暂不退出现有账号、不清理凭证或迁移任何已有数据。

## 待办：退出后 Tab 丢失与持久布局恢复

- 优先级：低；本轮只记录，不实现。
- 用户报告：主动退出 Warp 后，有时此前保存的 Tab 全部消失。具体退出方式、触发频率和根因未确认。
- 期望：重要 Tab、分屏、名称和工作目录可以持久保存，在自动恢复失败后仍有找回途径。
- 尚未确认“保存 Tab”指 pinned tab、自动会话恢复、Tab Config 或其他保存方式；后续先复现，再决定修复现有路径还是增加独立布局记录与恢复入口。
- 恢复布局与恢复退出前运行中的 shell/SSH/CLI 进程是不同能力；进程续接不在当前要求内。
- 官方说明：[Session Restoration](https://docs.warp.dev/terminal/sessions/session-restoration)。
- 可作对照的社区报告：[退出后 saved/pinned tabs 消失 #16729](https://github.com/warpdotdev/warp/issues/16729)，尚不能认定为同一根因。

## 本轮构建记录

2026-11-09 完成第一步：原版 OSS release 编译、开发包组装、启动及命令执行确认。

- `cargo build --locked --release --bin warp-oss --features gui -j 2` 成功，首次全量构建用时 28 分 15 秒；应用源码、Cargo.lock 和上游默认功能保持原样。
- 应用位置：`/Users/stonefish/Desktop/Warp/target/release/bundle/osx/WarpOss.app`。
- Bundle ID `dev.warp.WarpOss`、可执行文件 `warp-oss`、URL scheme `warposs` 均符合预期；实际签名不含 App Group entitlement。
- `codesign --verify --deep --strict` 通过；bundled 资源与源码一致；生成的 settings schema 通过结构检查；动态依赖均指向系统库/框架。
- 实际 OSS 进程、可见窗口及其 zsh 子进程已确认；用户随后手动确认可以正常输入并执行命令。自动输入尝试没有取得执行证据，不计作通过证据。
- OSS 已创建自己的 `~/.warp-oss`、偏好 domain 和 `~/Library/Application Support/dev.warp.WarpOss/warp.sqlite`，没有向其导入 Preview 数据。
- [构建命令和本机环境适配](build-macos.md)。官方打包工具还自动产生了 target 下的 DMG；本轮使用并验证的是最终 `.app`，未验收 DMG，不将它作为发布包。

### Preview 保护检查的实际结果

本轮未执行 Preview 安装、退出、数据迁移或清理命令。初始版本为 `0.2026.10.21.08.26.02`，构建期间检测到其已变为 `0.2026.11.04.08.17.01`；当前 Preview 进程启动于 15:25:36，早于 OSS 的 15:27:50。这里只记录观察到的更新/重启，不推断由谁触发。

由于该期间变化，Preview 应用二进制、Info.plist、设置和偏好的前后校验并非全部一致；不能声称旧应用与数据逐字节未变。键位文件 SHA-256 一致，原会话 SQLite/WAL/SHM 仍在原路径，主数据库 inode 保持一致；没有读取或逐条核对会话内容。OSS 实际使用另一个独立数据库，未覆盖旧库。

构建日志、打包日志及启动前后元信息保存在本仓库被忽略的 `.note/oss-baseline-2026-11-09/`，不包含数据库内容或凭证。该基线阶段未暂存、提交或推送本地文档，没有实施汉化/裁剪，也没有运行完整测试套件、长时间稳定性测试或性能基准。

## TinyWarp H1：中文基础与常用界面

用户已确认方案及分阶段提交，并将项目命名为 TinyWarp。源码目录和 GitHub 仓库名保持不变，以复用现有构建缓存。

- 应用显示名和独立候选包名改为 TinyWarp，内部二进制仍为 `warp-oss`。
- 翻译使用小型内部 `warp_i18n` crate，固定简体中文、内嵌字典、一次解析和静态英文回退；复用既有 serde/serde_json，未升级第三方依赖。
- 覆盖 macOS 菜单、常用动作、设置导航及完整外观页，包括工具栏编辑和目录颜色控件。
- 中文显示与稳定身份分开；设置 slug、保存位置 ID、action 名称、快捷键和配置枚举值保留。命令搜索同时匹配中文标签和英文别名，不把英文下标用于高亮中文。
- 内部数据身份仍为 `dev.warp.WarpOss` / `warposs` / `~/.warp-oss`；此次改名不迁移数据、不改变 Preview。
- H1 未包含登录、账号、同步、AI 停机和 Tab 布局增强；其中账号／同步与 AI 运行停用现已由 L1／L2 处理，Tab 布局增强仍在待办。不承诺内存降低。
- 验证、候选包及提交状态见 [H1 交付记录](h1-delivery.md)。

## TinyWarp H2：补齐本地终端汉化

- 新增 752 项译文，内嵌目录共 1091 项，覆盖标签/分屏、本地终端菜单、历史/补全、搜索、本地设置、布局配置与主题弹窗。
- 保留命令、程序输出、路径、用户名称、配置值、action、查询语法和数据身份；固定菜单使用独立显示文本，并兼容英文检索。
- 8 项翻译模块和 266 项应用回归通过；两者严格 Clippy 无 warning，正式 release、签名、包检查及基础运行通过。
- 构建在 Warp Preview 新标签执行；一次误关闭后复用缓存恢复。候选位于 `.note/candidates/H2/release/bundle/osx/TinyWarp.app`，保留 H1 包，不覆盖 Preview，不迁移/清理已有数据。
- 全部证据和人工验收边界见 [H2 交付记录](h2-delivery.md)。H2 未包含免登录或同步／AI 停机，现已由 L1／L2 处理；内存优化与 Tab 恢复仍在待办，SSH 保留。

## TinyWarp L1：本地启动、账号与云同步精简

- 普通 OSS 应用启动直接进入本地工作区；新用户无需账号，旧登录记录不加载、不清除。CLI/SDK、TUI、daemon 和其他通道保留原路径。
- 隐藏账号、订阅、团队、邀请、云盘和共享入口，不构造八类账号/云设置页；保留 action ID、设置 slug 和旧快照格式。
- 停止认证刷新、设置同步、队列消费、云对象/团队轮询和缓存触发的 websocket；保留缓存、队列、数据库表与 migrations。
- 8 项翻译测试、168 项应用回归、2 项真实隔离启动测试及严格 Clippy 通过。旧数据 fixture 验证三个标签、原活动标签、历史与目录关联、真实搜索/接受和旧凭据零读写删。
- 在 Warp Preview 正式 release 构建成功，用时 14 分 10 秒；签名和包检查通过，L1 已启动并执行真实 shell 命令，原生账号与云盘菜单已确认隐藏。
- 保留 H2 包，不替换 Preview，不迁移或清理真实旧数据。实际旧标签/历史未逐条核对，视觉布局、长期稳定性和性能基准未穷尽。
- 完整证据、第三方 AI 旧密钥不加载的影响和后续范围见 [L1 交付记录](l1-delivery.md)。

## TinyWarp L2：内置 AI、Agent 与外围停用

- 内置 AI／Agent、Warp 对 Agent CLI 的增强界面、独立代码／Markdown 编辑器、文件树及相关设置／菜单／URI／IPC 入口停用；外部 CLI 使用普通 PTY。
- 停止模型／配额刷新、编排／任务轮询、MCP／skills 发现与自动启动、代码／规则索引和 LSP；保留必要闲置模型、本地控制与 shell 通信。
- 旧不可用 pane 显示只读占位并保留原快照；旧终端 AI 字段和命令块元数据保留回写。无 SQLite schema／migration／Cargo.lock 改动，不清理旧记录或凭据。
- 8 项翻译、8 项 Unix PTY、294 项应用、3 项隔离真实 GUI 测试，共 313 项通过；严格 Clippy 无 warning。正式 release 在 Warp Preview 构建完成，用时 12 分 39 秒，签名与包检查通过。
- L1 已正常退出，OSS 备份通过完整性检查；用户允许 macOS 桌面访问后，L2 真实 shell 命令与中文输出通过，原生 AI／账号／云盘菜单隐藏，⌘N／⌘Q 保留。Preview 原进程继续运行，主程序与 Info.plist 校验一致。真实旧数据逐条核对和视觉布局未穷尽。
- SSH 保留；未拆编译依赖、未做性能基准，低优先级 Tab 保存问题仍在待办。详情见 [L2 交付记录](l2-delivery.md)。

### L2 设置入口修复与日常开发

直接打开设置时，旧默认 Account 快照被当作已停用功能拦截。已让新建设置页复用本地 Features 默认规则；67 项相关回归、1 项真实 GUI 绘制／标签复用／返回终端用例和严格 Clippy 通过，用户确认设置正常显示并复用。

本轮独立开发候选应用编译 2 分 43 秒，签名／资源／真实 shell 检查通过；未运行新的正式 release。该修复阶段尚未导入 Preview 设置，随后已按用户授权单独完成本地偏好迁移。证据见 [设置入口修复](l2-settings-fix.md)，开发方式见 [macOS 构建记录](build-macos.md#日常开发与快速验证)。

### Preview 本地设置导入

2026-11-10，按用户开始日常使用 TinyWarp 的要求，导入 Preview 的 23 项显式本地偏好、2 个自定义快捷键和 2 个 Tab 配置，并让 5 项 TinyWarp 显式偏好恢复为 Preview 的缺省语义。原设置及 TinyWarp 自身 SQLite 已独立备份，未导入账号、AI／同步状态、凭据、历史或会话数据库。

无需重新编译，正常重启现有开发包后配置与快捷键加载错误为零，原生本地菜单和独立身份检查通过。用户确认正在界面里自行调整设置，这些导入后的新值全部保留；Preview 配置和安装文件校验一致，原进程继续运行。范围与验证边界见 [设置导入记录](preview-settings-import.md)。


## TinyWarp L3A：隔离后台内存基线

2026-11-10，按用户调整后的范围完成本地控制接口验证和同进程三轮后台内存采样；前台输入／滚动另测。优化构建及控制入口链接在 Warp Preview 执行，没有替换日常 TinyWarp。

应用 footprint 存在逐轮增长，额外 shell 能在默认约 60 秒撤销期后退出。结果包含控制服务、窗口缓存及现场调度影响，部分采样稀疏，尚不能认定泄漏或报告优化收益。原配置哈希、数据库文件元信息与 Preview 存活复核通过。

详见 [L3A 基线与限制](l3a-performance-baseline.md)、[数字结果](l3a-results.json) 和 [测量工具](../../script/tinywarp-perf/README.md)。

## TinyWarp L3C1：专用构建与语音依赖裁剪

2026-11-10，用户调整为以功能精简推进，不再把专项性能定位作为前置任务。新增 `tinywarp_gui` 构建标记，OSS 开发与 macOS 打包入口默认使用；内置语音及独有音频链的 16 个 package 退出当前 macOS 构建图，原 `gui` 构建保留。

开发候选已在 Warp Preview 编译、签名并做隔离 shell／CLI／设置检查，日常包未替换。不报告内存或速度收益。完整范围、失败自动化与验证边界见 [L3C1 交付记录](l3c1-delivery.md)，当前构建方法见 [macOS 构建说明](build-macos.md#l3c1-之后的-tinywarp-构建选择)。


## TinyWarp L3C2：AWS 模型凭据 SDK 裁剪

继续以独立模块裁剪为主线，本批将 AWS／Bedrock 凭据 SDK 从 TinyWarp 产品依赖图移出，959 → 921 项，保留共用 TLS 和数据类型。开发及组包入口同步无默认 feature 参数；其他通道保持完整默认，新增依赖边界检查防止 SDK 回流。范围与最新验证状态见 [L3C2 交付记录](l3c2-delivery.md)。

## TinyWarp L3C3：Mermaid 专用渲染依赖裁剪

继续按专用模块裁剪，TinyWarp 移除 Mermaid SVG 渲染器，保留终端共用编辑器、图表源码与原有数据类型。完整默认构建仍保留渲染能力。依赖图 921 → 915 项，无新增依赖；交付状态与验证见 [L3C3 交付记录](l3c3-delivery.md)。

## TinyWarp L3C4：内置 Agent CLI 的 JSON 过滤引擎

保留终端与本地控制共用的 CLI 参数，单独裁掉内置 `--jq` 引擎；普通 JSON 输出与外部 CLI 保留。依赖图 915 → 894 项，无新增。763 项单元／定向回归、四组严格检查及隔离候选验证已通过，详见 [L3C4 交付记录](l3c4-delivery.md)。

## TinyWarp L3C5：本地 GUI 启动读取范围

保留窗口／标签／分屏／命令块、历史及终端共用的项目／忽略建议，跳过云对象、AI 控制数据、索引与同步动作加载；旧记录仍留在磁盘，必要原始会话元数据继续保存。本批不调整依赖清单或数据库 schema，已通过 280 项定向单元回归、4 项 GUI 回归、两组严格检查及九组旧记录保留复验，详见 [L3C5 交付记录](l3c5-delivery.md)。


## TinyWarp L3C6：内置语言服务器与 Node 运行链

五类内置语言服务器的具体发现／安装实现退出 TinyWarp 构建，保留共用协议、持久化类型和外部 CLI；完整默认保留原能力。产品依赖图 894 → 891 项，移出 Node 运行时、tar 和 xattr，无新增。317 项单元回归、4 项 GUI 集成、四组严格检查及十组旧记录保留复验通过；原生设置截图和 OS 菜单标签／分屏复验已在解锁后补齐，日常应用未替换。详见 [L3C6 交付记录](l3c6-delivery.md)。


## TinyWarp L3C7：输入分类器与词库评分后端

TinyWarp 不再创建内置输入分类器，也不编入具体启发式分类／词库评分实现；共用输入类型、shell 工具与旧设置保留。词干库仍由终端共用搜索使用，包总数保持 891，无新增或版本升级，改以真实 feature 门禁防止后端回流。330 项单元／组件测试、4 项 GUI 集成、七组严格检查及隔离原生／十组旧记录保留复验通过。本阶段日常应用未替换；构建方式后来按 D1 明确为同一个 Preview 窗口内新开会话。详见 [L3C7 交付记录](l3c7-delivery.md)。


## TinyWarp D1：日常优化版已安装

已基于 `7c784cee2` 生成并验证 opt3 优化包，安装到 `/Applications/TinyWarp.app` 并恢复原 OSS 数据。原三标签、两终端 pane、十一条历史及旧记录回读通过，设置保留；配置、原生偏好和 SQLite 只读在线备份保留，用户确认输入、中文选词及命令执行正常。Preview 保留，后续构建在同一 Preview 窗口内新开会话。本轮精简收尾，偶发 Tab 丢失仍作为低优先级反馈跟进。详见 [D1 日常交付与回退](d1-delivery.md)。

## TinyWarp U1/U2：本地更新与重启流程

已接入本地更新收件箱，复用 Warp 更新状态机、可取消退出确认与会话保存，增加完整应用原子换包、旧包保留和启动失败回退。右上角齿轮恢复为不含账号功能的本地应用下拉菜单，更新准备好后可重启更新。相关回归、严格检查、隔离换包／失败回退与用户实际菜单／取消操作完成；真实界面更新后的标签、分屏、历史和设置独立回读通过。版本显示截断已在最终开发候选修正，外观待复看。U1/U2 阶段未替换日常应用；后续 U3 已补齐公开源和发布签名，并完成用户确认的引导安装，见下文。详见 [U1/U2 交付记录](u1u2-delivery.md)。

## TinyWarp U3：公开源与发布签名候选

已接入个人 GitHub Releases 匿名 HTTPS 更新源，内置专用发布公钥，先验证 SSHSIG 清单再校验下载包；普通使用者不需要账号或密钥。10 项定向回归、严格检查、opt3 构建、离线签名产物和隔离原生启动／会话／CLI／正常退出检查通过。当前公开清单为 404，无公开 Release，完整在线下载安装尚未验证。用户确认后已完成日常引导安装：原 2 标签、终端／设置页、13 条历史、4 个配置文件和数据库 inode 独立回读保留，Preview 原进程继续运行；实际输入和最终菜单外观待人工确认。详见 [U3 公开更新实现与边界](u3-public-updates.md)。

安装与回退详见 [U3 日常引导安装](u3-bootstrap-delivery.md)。


## TinyWarp 独立版本规则

产品版本采用 `0.1.0` 起始的三段数字；构建号独立递增，关于页展示，紧凑齿轮菜单不显示版本。开发引导包使用 `0.1.0-dev.1`，后续通过正式 `0.1.0` 验证公开更新；日期版首次切换需正常退出后的本地引导安装。规则、构建入口和交付边界见 [版本规则](tinywarp-versioning.md)。
