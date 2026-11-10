# TinyWarp L2 设置入口修复

2026-11-10，基于 `c55986fd5`。用户报告直接点击设置会打开“此功能不可用”占位页，并要求缩短日常构建等待。沿用本地提交、不推送、在 Warp Preview 构建、保留 Preview 与现有数据的约定。

## 根因与修复

`WorkspaceAction::ShowSettings` 没有指定 section。`open_settings_pane` 新建快照使用 `page.unwrap_or_default()`，默认值为 Account；L2 的旧功能快照保护先将它变成 UnavailableFeaturePane，因此尚未到达 SettingsView 自身的本地 Features 默认页规则。占位 pane 未注册为真实 SettingsPane，重复点击又创建一个。

两项新增测试先沿真实 action 路径复现，旧实现均在真实 SettingsPane 类型断言失败；编译用时 3 分 15 秒，测试运行 0.24 秒。日志位于忽略的 `.note/l2-settings-fix/runs/red/`。

修复只让新建 settings 快照复用已有 `initial_settings_section`：本地未指定页为 Features，其他通道仍为 Account；已有设置页的复用分支不改，保留其当前页与搜索语义。没有修改 SettingsSection enum/default、设置 slug、持久化格式或旧快照保护，不删除已存在的占位标签。

新增隔离 GUI 用例通过与齿轮相同的 ShowSettings 动作打开设置，检查真实设置 pane、可用本地 section、重复点击复用标签，再回到终端执行命令。采用临时 HOME、独立测试 AppId 和凭据 Spy，不把用户真实数据作测试夹具。

## 开发构建与验证

用户明确指出每次完整构建等待过长。本轮先采用现有 release 依赖缓存，只为 warp 包关闭优化与调试符号生成，不执行仓库原 profile 的正式 release，也不新增依赖、全仓测试或常驻文件监听服务。Rust 逻辑改动仍需编译／重启；配置热加载不能替代程序逻辑修复。

67 项设置、入口与旧快照相关应用回归已通过；真实 GUI 用例也通过，包括内容控件首次打开前未绘制、打开后已绘制、设置标签复用与返回终端执行命令。GUI 的绘制证据来自现有 `SavePosition::paint` 写入的位置缓存，不用“构造了视图”代替“实际绘制”。应用和 integration 接线的严格 Clippy 均通过，无 warning。

初轮绿色测试的额外渲染观测使用了 `debug_child_view_ids`，但 Shrinkable 容器没有转发该接口，导致已正常的 SettingsPane 被漏报。已移除该无效观测，保留所有真实 pane、当前页、快照、manager 与重复 tab 断言，并把绘制检查放在上述 GUI 接缝。GUI helper 一处错误方法调用已改为现有 `tab_count()`；这些测试接线修正没有改变产品行为。

本轮没有为测试框架补新接口或修改 UI 容器。应用回归构建实际为 4 分 53 秒，GUI 测试程序构建为 1 分 52 秒；它们不是快速开发应用包的构建耗时。

独立开发候选目标为 `.note/candidates/L2-settings-fix-dev/release/bundle/osx/TinyWarp.app`。构建仍在 Warp Preview，候选必须保持独立，不能覆盖运行中的 L2。开发候选不是正式 release，不用于内存或响应性能比较。开发应用编译实测 2 分 43 秒，组包／签名／检查退出码 0；签名、身份、无 App Group、19 个 schema 分区、140/140 资源及 25 项系统动态依赖通过。这个耗时只指开发应用编译，不包含测试构建和人工验收，也不是后续每次构建时间的保证。

切换前确认旧 L2 只有空闲 shell，并对 OSS 自身 SQLite 做只读在线备份，quick_check 通过；正常退出旧 root/helper 后启动独立开发候选，没有并发使用同一 OSS 数据库。原 L2 包和备份保留，Preview 原进程仍存活。

开发候选实际 shell 的 pwd／中文输出／架构／无 CLI 增强广告检查通过，执行标记的父进程链归属精确候选；启动后签名复查通过。用户明确确认：“设置正常显示，重复打开会复用”。已存在的旧占位 Settings 标签没有自动删除。

本轮仅对 7 个 Rust 文件执行一次最终格式化，差异空白检查通过；按仓库要求未因纯格式化重跑测试或构建。无 Cargo.lock／schema／设置数据变更，本轮本地提交、不推送。

验证封存首次因顶层临时 GUI 摘要被纳入脚本指纹而中断；该摘要已移入运行目录，逐项核对源码与验证脚本均与起始快照一致后，仅补做封存，保留原退出码及原因，没有重跑或伪报编译结果。

## Preview 设置

只读核对到 Preview 使用 `~/.warp-preview/settings.toml` 与 `keybindings.yaml`，TinyWarp 使用自己的 `~/.warp-oss` 配置目录。是否导入与设置入口回归无关；本轮没有向任何一方写入、合并或复制配置。

若后续确认导入，建议先比较并备份 TinyWarp，再选择字体、主题、标签／分屏外观及快捷键；不整目录覆盖，不带入账号、密钥、AI／同步状态、数据库或会话。自定义主题还需核对相应资源，不能仅复制主题名称。兼容性与具体覆盖项应在实际导入前单独核对。
