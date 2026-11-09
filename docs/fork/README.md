# TinyWarp

## 已确认方向（2026-11-09）

先供自己的 Mac 使用，目标是中文、精简和降低内存占用，沿用 Warp，界面更干净紧凑。先编译并运行未修改的 OSS 基线，与现有 Warp Preview 并存，保留 Preview 及现有数据。

保留命令分块、历史搜索、本地补全、多标签、分屏、会话恢复、快捷键，以及运行项目和任意 CLI 的能力。计划移除内置 AI / Agent、Warp 为 Agent CLI 提供的增强界面、内置代码编辑器与文件树、Warp Drive、登录、云同步和团队协作入口；这些裁剪尚未实施。

SSH 暂时保留。用户认为也可以移除，但如果对性能影响不大，不必为了精简而删除；以后依据实际测量决定。

## 当前技术方案

用户已确认继续基于 master，先汉化，再逐步精简登录、账号、同步与 AI。技术方案已确认，项目命名 TinyWarp；每个阶段完成验证后提交一次，不自动推送。具体实现见 [中文本地终端技术方案](2026-11-09-technical-plan.md)，H1 已完成并验证，按阶段进行本地提交；后续批次独立推进。

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
- 登录、账号、同步、AI 停机和 Tab 布局增强留在后续阶段；H1 不承诺内存降低。
- 验证、候选包及提交状态见 [H1 交付记录](h1-delivery.md)。
