# TinyWarp

基于 [Warp](https://github.com/warpdotdev/warp) 的独立终端 fork：保留命令分块、历史、补全、标签与分屏，提供中文界面，专注本地终端。

无需 Warp 账号，停用内置 AI／Agent、云同步与团队协作；Codex、Claude Code、SSH 等外部 CLI 仍作为普通终端程序使用。TinyWarp 由独立维护者开发，与 Warp 官方没有隶属关系。

[下载与版本记录](https://github.com/sty20030818/TinyWarp/releases) · [待办与路线](ROADMAP.md) · [参与贡献](CONTRIBUTING.md) · [构建说明](docs/fork/build-macos.md)

## 当前状态

TinyWarp 正朝面向公众的正式产品推进。当前已发布 **0.1.1（构建 4）**，首阶段支持 **macOS Apple Silicon**；Intel Mac、Windows 和 Linux 尚不列入已验证支持范围。

当前公开包采用 ad hoc 签名，应用内更新使用专用发布公钥验证清单与包哈希。Apple Developer ID、公证和首次浏览器下载的 Gatekeeper 体验尚待完成，当前发布不代表这些分发环节已通过验收。

## 保留与精简

| 保留 | 已停用／裁剪 |
| --- | --- |
| 命令分块、历史搜索与本地补全 | 内置 AI、Agent 及其增强界面 |
| 多标签、分屏、快捷键与会话恢复 | 登录、账号、订阅、云同步与团队协作入口 |
| 主题、字体与本地设置 | Warp Drive、内置代码编辑器与文件树 |
| SSH、运行项目和任意外部 CLI | 内置语音、部分语言服务器及相关专用运行依赖 |

这是产品运行与构建范围的区别，部分底层共享代码和历史数据类型仍保留。现阶段没有可证明“比 Warp 更省内存／更快”的完整对照结果，不承诺未经测量的性能收益。

## 下载与更新

首次安装可从 [Latest Release](https://github.com/sty20030818/TinyWarp/releases/latest) 获取 `TinyWarp-aarch64.zip`。同一 Release 还提供更新清单和独立签名；这里只分发 macOS Apple Silicon 应用。

已安装 TinyWarp 的用户在右上角应用菜单选择“检查更新”，准备完成后选择“重启并更新”。更新沿用现有设置与本地数据；重启前请妥善处理运行中的命令。旧 0.1.0 菜单若在出现更新红点后仍停留在检查状态，可收起再展开查看重启入口。

更新不需要 Warp 或 GitHub 账号。仓库改名前的已安装版本仍使用旧地址，由 GitHub 重定向继续获取更新；新源码使用 TinyWarp 仓库地址。发布清单认证与安装／回退边界见 [公开更新说明](docs/fork/u3-public-updates.md)。

## 数据与兼容

TinyWarp 目前沿用 OSS 身份 `dev.warp.WarpOss`、配置目录 `~/.warp-oss` 和对应本地数据库，与 Warp Preview 分开保存。仓库改名不改变这些身份，也不触发数据迁移。它会与其他使用同一 OSS 身份的安装共享数据，不应把两者当作隔离环境同时运行。

Tab／分屏布局恢复不等于恢复已退出的进程。已有偶发 Tab 丢失反馈，复现和修复列在 [路线](ROADMAP.md) 中；重要工作请保留独立记录。

## 开发与维护

主线为 `main`。TinyWarp 独立演进，按需挑选上游的终端、安全和兼容性修复，不自动合并上游全部功能。

项目沿用 Rust、WarpUI 和现有锁文件。TinyWarp GUI 使用 `--no-default-features --features tinywarp_gui`，完整环境、当前构建命令与独立候选验证方式见 [macOS 构建说明](docs/fork/build-macos.md)。部分早期文档含历史构建方法，实施时以当前章节为准。

贡献前请阅读 [贡献指南](CONTRIBUTING.md) 与 [工程约定](AGENTS.md)。TinyWarp 自己的 CI 和可复现发布流程仍在建设，不把继承的 Warp 自动化视为已经可用的验证保障。

## 反馈与路线

使用 [TinyWarp Issues](https://github.com/sty20030818/TinyWarp/issues) 报告可复现问题或提出需求；请附版本、系统、操作步骤和预期行为，提交日志前移除凭据、命令内容和其他私人信息。

当前待办、已确认的产品边界以及暂停的研究统一放在 [ROADMAP.md](ROADMAP.md)。历史实现和验收证据保留在 [项目记录](docs/fork/README.md)。

## 上游与许可

感谢 [Warp 及其贡献者](https://github.com/warpdotdev/warp) 和所依赖的开源项目。TinyWarp 从公开 `master` 的 `abb3fe33e004f7b1de540fecd6e3eec20dbc1883` 起步；保留上游来源、提交历史和许可文件。

- WarpUI（`warpui_core`、`warpui`）使用 [MIT](LICENSE-MIT) 许可。
- 其余代码使用 [AGPL v3](LICENSE-AGPL) 许可。
- 发布应用随包提供项目许可、第三方许可汇总和精确源码定位。
