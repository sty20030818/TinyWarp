# 参与 TinyWarp

TinyWarp 是面向公众、独立维护的 Warp 终端 fork。首阶段聚焦 macOS Apple Silicon、本地终端与中文界面，产品边界和优先事项见 [README](README.md) 与 [路线](ROADMAP.md)。

## 报告问题与讨论需求

先搜索 [TinyWarp Issues](https://github.com/sty20030818/TinyWarp/issues)。Bug 报告请提供应用版本、macOS 版本、芯片架构、复现步骤、预期与实际结果，以及必要的脱敏截图或日志。

需求先说明具体使用问题，再讨论方案。涉及会话、数据、更新协议、品牌、支持平台或大范围重构时，先与维护者确认范围，避免提交无法接纳的大改动。

不要在公开 Issue／PR 中披露尚未修复的漏洞、凭据或私人数据。TinyWarp 自己的私密漏洞报告渠道尚待建立；现有 [SECURITY.md](SECURITY.md) 描述的是 Warp 上游渠道，不代表 Warp 团队负责接收或修复 TinyWarp 特有问题。

## 开发与 Pull Request

1. 从 `main` 创建工作分支，PR 目标同样为 `main`。旧 `master` 和 `sty20030818/local-terminal` 保留作历史参考。
2. 按 [macOS 构建说明](docs/fork/build-macos.md) 准备环境，沿用现有工具链和锁文件；GUI 使用 TinyWarp 专用 feature。
3. 在变更涉及的行为边界补最小回归检查，运行相关测试、严格检查，最后格式化。具体顺序见 [AGENTS.md](AGENTS.md#implementation-validation-order)。文档变更核对 diff 和链接，不默认运行完整 presubmit。
4. 使用 [PR 模板](.github/pull_request_template.md)，说明问题、修改后的行为、实际验证和未验证范围。UI 变更附实际截图；涉及本地数据时说明保留与恢复边界。
5. 提交信息使用约定式格式，例如 `fix(menu): 修正菜单状态刷新`。保持一次 PR 聚焦一个完整目标。

TinyWarp 尚未建立自己的完整 CI，也未接入上游 Oz 自动分派、评审或发布流程。不要依赖上游机器人、内部 runner、Slack 或 `@oss-maintainers` 处理本项目的贡献。

## 同步上游

`upstream` 指向 [warpdotdev/warp](https://github.com/warpdotdev/warp)，产品开发在 TinyWarp `main`。按需挑选终端、安全与兼容性修复；引入前核对账号、云、AI、数据与依赖边界，合入后运行对应回归，不自动整体同步上游。

保留上游提交来源及现有许可。需要调整共享模块时核对全部消费者，不为精简破坏终端依赖的共用类型、存储契约或外部 CLI 行为。
