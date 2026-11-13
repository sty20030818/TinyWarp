# TinyWarp 仓库身份与公众项目入口

日期：2026-11-13。用户确认将 TinyWarp 作为面向公众的独立 Warp 魔改产品，首阶段聚焦 macOS Apple Silicon，独立主线按需挑选上游终端、安全与兼容性修复。

## 仓库与分支约定

仓库从 `sty20030818/warp` 改名为 [sty20030818/TinyWarp](https://github.com/sty20030818/TinyWarp)。仓库 ID 仍为 `1451027860`，公开 fork 关系及上游 `warpdotdev/warp` 保留。About 已按本地终端定位更新，homepage 指向 TinyWarp Latest，普通 Issues 入口启用。

用户先选择现有开发分支为默认，随后明确改为新建产品主分支；最终采用 `main`。它从原 TinyWarp 成果 `4cf7427495ba2a125b62232970450c1ab41839c8` 起步，包含本轮入口与更新源修正。`master`、`sty20030818/local-terminal` 和既有标签保留，不强推或改写其历史。默认分支应指向带本轮防护的 `main`，以远端回读作为切换完成证据。

新源码的唯一公开更新源常量改用 `https://github.com/sty20030818/TinyWarp/releases`。旧 0.1.0／0.1.1 仍内置旧地址，继续依赖 GitHub 改名重定向；不得重新创建占用旧名 `warp` 的仓库。新地址只会随后续正常发布进入新客户端，本轮不重新发布或覆盖 0.1.1。

## 更新连续性与资产核对

改名前保存仓库 ID、源码、Release ID、资产 ID、大小与 SHA256 基线。改名后核对两个公开 Release 的上述属性、目标源码和发布时间均保留，Latest 仍为 `v0.1.1`。

对旧 `warp` 和新 `TinyWarp` 地址分别匿名请求：

- latest 更新清单与独立签名均 HTTP 200，字节匹配已发布 0.1.1，并以仓库内公钥独立 SSHSIG 验签通过。
- `v0.1.0` 与 `v0.1.1` 的精确 ZIP 路径均完成重定向，Range 请求 HTTP 206 且前四字节为 ZIP 签名。只取必要字节，没有重复下载完整应用包。
- curl 禁用用户配置、不附账号认证头；最终下载域为 GitHub Release 资产域。
- 改名前的 Git origin 地址仍能读取原开发分支和两个版本标签；本地 origin 已同步新 canonical 地址。

这证明已核对的公开地址连续性，不冒充旧客户端再次完成安装。应用身份 `dev.warp.WarpOss`、配置／数据库路径、发布公钥和换包协议均未改动。

## 项目入口与自动化

根 README 改为 TinyWarp 产品入口，明确能力、支持范围、安装更新、数据身份、当前分发限制、上游及 MIT／AGPL 许可。贡献指南和普通 Issue 表单改为本项目流程；两份上游 SSH 专用表单完整移至 [归档](upstream-issue-templates/README.md)，SSH 产品能力保持原状。当前待办统一放在 [ROADMAP](../../ROADMAP.md)，历史记录不批量重写旧地址或阶段结论。

继承的 `publish-agent-dev-image.yml` 原本缺少 owner 限制，按月定时、使用上游 runner／Docker OIDC 并尝试推上游镜像。本轮为 publish job 增加与其他上游自动化一致的 `warpdotdev` owner 条件。该原有任务在 TinyWarp 仓库不能通过入口条件；没有借改名启用上游构建矩阵或创建自动发布流程。TinyWarp 自己的有效 CI 仍列为待办。

## 验证与边界

- 11 项 TinyWarp 更新／签名／传输／可逆换包相关回归通过；真实数据路径隔离。
- `cargo clippy --locked -p warp --lib --tests --bin warp-oss --no-default-features --features tinywarp_gui -j2 -- -D warnings` 通过；随后只格式化本轮 Rust 文件。
- 活动 Issue 表单／配置与工作流 YAML 解析通过，Markdown 相对链接与 diff 检查通过。
- 首轮测试启动脚本漏带隔离配置，测试未运行；补齐后复用已编译程序，11 项全部通过，没有记为产品缺陷或首轮通过。
- 日常 TinyWarp 初始未运行，为遵循既有构建约定，打开已安装包后在恢复的窗口中新建专用验证标签。没有安装新候选、退出用户 CLI 或迁移数据。
- 2026-11-13 只读核对已安装应用为公开 0.1.1 构建 4，二进制匹配；原 36 条历史稳定字段、pane 标识、既有数据组、4 个配置文件和数据库 inode 保留。最终菜单体验仍需用户确认。

没有运行全仓库／跨平台测试、完整新应用打包、公证或真实旧客户端升级；没有提交本机数据、备份、私钥或临时验证材料。原始结果保存在忽略目录 `.note/repository-rename/`。

关于页新 logo 的方向为彩色终端字符格／块字与立体厚度；当前只有待审阅的视觉候选，尚未替换应用资源。

参考：[GitHub 仓库改名](https://docs.github.com/en/repositories/creating-and-managing-repositories/renaming-a-repository)、[Actions schedule 事件](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#schedule)。
