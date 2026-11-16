# macOS 应用图标切换修复

2026-11-16，完成源码修复及独立候选验证，已随 [0.1.3（构建 6）](0.1.3-settings-avatar.md) 公开发布。日常安装本轮未替换，升级验收仍由用户完成；没有覆盖、退出日常应用或迁移数据。

## 根因与实现

0.1.2 的公开包及日常安装都缺少 `Contents/PlugIns/WarpDockTilePlugin.docktileplugin`、主应用的 `NSDockTilePlugIn` 注册和替代图标 PNG。设置事件已经到达图标更新逻辑，但找不到资源后提前返回。插件声明的 `NSPrincipalClass` 另与实际 Objective-C 类名大小写不一致。

`script/macos/run` 和 `script/macos/bundle` 统一调用 `script/macos/bundle_dock_tile_plugin`：前者按既有 Makefile 构建，后者安装预构建插件，两者最后执行相同的身份、入口类、架构与 20 份 PNG 资源校验。拒绝覆盖已有插件，组装失败即停止后续签名。`script/tinywarp-release` 在读取发布私钥之前执行只读校验，避免给缺插件的应用生成分发文件。

公开设置的权威来源继续是现有 TOML。`AppearanceManager::set_app_icon` 将当前选择单向镜像到应用自己的 native `AppIcon` 偏好，供 Dock 插件读取；包括启动、设置变化和更新后的既有调用路径。不从旧 native 值反写 TOML，镜像写入失败保留诊断，并继续尝试更新运行图标。检查 `NSWorkspace` 图标设置的返回值，避免忽略失败。

## 已运行验证

- 73 项定向 Rust 回归全部通过，含 4 项新增测试：旧 native 镜像被当前选择替换、默认值清除旧选择、偏好写入失败保留错误、全部非默认图标映射到真实 PNG。
- 实际候选的 5 项打包检查通过：完整包、带空格路径的预构建安装、拒绝重复安装、拒绝错误入口类、拒绝缺失图标。插件包含 arm64 与 x86_64，覆盖本候选主二进制的 arm64。
- 日常旧 0.1.2 包的只读检查如预期失败；发布工具在读取私钥前拒绝该包，没有生成发布目录。
- 隔离原生实例使用独立 bundle identity、临时 HOME 和沙箱，禁止访问日常／Preview 数据、AWS 与日常应用包，并禁止 IP 出站。Neon 和 Cow 的文件图标、运行图标及实际加载的插件图像，在统一 RGBA 渲染后与对应 PNG 完全一致。
- 正常退出并重启后 Cow 保留；选择默认后 native 值为 Default、插件清除自定义视图。再次正常启动后，文件图标与初次默认状态完全一致，三个测试进程均正常退出。
- `cargo clippy --locked -p warp --lib --tests --bin warp-oss --no-default-features --features tinywarp_gui -j2 -- -D warnings` 通过；相关 shell、Python 语法与插件 plist 检查通过，最后完成两个 Rust 文件的格式化。没有运行完整 presubmit。

本轮通过设置文件监听触发真实设置变化路径，没有自动点击设置页的图标下拉项。插件通过原生 AppKit 加载，并核对入口类与实际图像；没有重启系统 Dock，也不将此表述为系统 Dock 进程自身的插件加载验收。默认图标的系统样式过滤沿用既有行为：从自定义图标启动后选择默认，完整系统样式在下一次启动恢复。

候选位于 `.note/candidates/icon-fix-dev2/release/bundle/osx/TinyWarp.app`，开发标签 `v0.1.2-dev.2`。这是应用 opt0 的本机功能验证包，不用于性能比较或公开更新。原始日志、图像与脱敏数据回读保存在忽略目录 `.note/icon-fix-2026-11-16/`。

## 后续交付

0.1.3 已从确认的干净源码构建正式优化包，完成插件校验、签名与更新资产核验；用户通过日常 TinyWarp 的“检查更新”升级验收。现有应用身份、配置与数据路径沿用，已发布 0.1.2 资产未覆盖。全工作区、跨平台、VoiceOver 与正式分发签名／公证不属于本次功能验证。
