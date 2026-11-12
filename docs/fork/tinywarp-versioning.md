# TinyWarp 产品版本与构建规则

TinyWarp 从 `0.1.0` 开始使用独立产品版本。补丁、翻译和样式修正递增 patch；功能递增 minor；达到稳定交付目标时再确认 `1.0.0`。`0.x` 期间仍以发布说明明确可能影响使用的变化。

## 唯一构建配置

仓库根目录 `tinywarp-version.json` 保存稳定产品版本 `version` 和正整数 `build`。应用关于页、macOS 打包脚本与发布工具共用这份配置，构建号不从日期、Git 提交数或本地时间推导。

- 关于页：`0.1.0（构建 2）`；复制按钮保留完整更新标签 `v0.1.0`。
- 齿轮菜单：不显示版本，保留紧凑布局和检查／重启更新动作。
- Git 标签和清单：`v0.1.0`。
- macOS `CFBundleShortVersionString`：`0.1.0`；`CFBundleVersion`：`2`。
- 本地引导包：`v0.1.0-dev.1`，关于页显示 `0.1.0-dev.1（构建 1）`，macOS 产品版本仍为三段数字。

每份交付给使用者的候选都应递增 build；开发包的 `dev.N` 序号也须递增。两者职责不同：build 供系统和排错使用，更新器只比较产品版本与 dev 序号。同一个公开版本不得覆盖发布；需要重发时递增产品版本，不能只改 build。

Rust 使用锁文件已有的 `semver 1.0.20` 做数字比较。TinyWarp 仅接受 `vX.Y.Z` 和本地 `vX.Y.Z-dev.N`，不接受前导零、其他预发布通道或 `+metadata`。正式版本高于同一产品版本的开发包；`0.10.0` 高于 `0.9.0`。公开发布工具拒绝 dev 包。其他 Warp 通道保留原日期规则。

## 构建与离线发布

直接 Cargo 构建前设置标签，并在同一环境完成打包：

```sh
export GIT_RELEASE_TAG="$(./script/tinywarp-version --development 1)"
# 使用 build-macos.md 中的 tinywarp_gui 构建和独立候选打包命令。
```

公开候选先更新配置的 build，再去掉 development 参数：

```sh
export GIT_RELEASE_TAG="$(./script/tinywarp-version)"
# 编译、打包及独立验收完成后：
script/tinywarp-release /绝对路径/TinyWarp.app --output /不存在的发布产物目录
```

打包会核对标签与配置，并写入数字格式的系统版本字段。投递和发布工具会同时校验 Info.plist、资源元数据与版本格式。发布源、签名、三个分发文件和公开操作边界继续遵循 [U3 方案](u3-public-updates.md)。私钥、用户配置和数据库不进入发布文件。

## 从日期版引导

现有日期版没有 SemVer 解析能力，不能靠它自动安装 `v0.1.0-dev.1`。首次切换需要在候选验证完成后，正常退出日常 TinyWarp、备份完整应用和当前数据，再使用已有完整应用换包 helper 安装本地开发引导包。不得强退正在运行的 CLI，不覆盖设置、历史或数据库；应用回退不恢复旧数据库。切换前仍须确认用户当前任务已结束。

新客户端保留旧日期版的本地 inbox 文件，但跳过其清单，使它不会阻挡 GitHub 更新。无须清空收件箱或迁移 SQLite。切换后通过正式 `v0.1.0` 验证真实公开更新；不要提前将正式包投递到日常本地 inbox，否则会遮蔽 GitHub 来源。

2026-11-12 经用户确认任务已结束，已将 `v0.1.0-dev.1（构建 1）` 安装到 `/Applications/TinyWarp.app`。用户随后明确批准，已推送该 TinyWarp 源码分支及 `v0.1.0` 标签，并公开发布正式 `0.1.0（构建 2）`。

## 定向验证

33 项 Rust 回归与 5 项 Python 打包测试通过。覆盖数字版本优先级、dev 到正式版、异常／旧日期版本拒绝、旧本地收件箱跳过、更新状态机下载重试与已下载包复用、签名及传输边界、打包字段一致性和菜单／设置入口。共享状态机测试按通道选择版本夹具；原 Warp 通道的回滚预期保留，TinyWarp 拒绝降级。严格 Clippy 通过；未运行全仓库测试或完整 presubmit。

优化构建用时 9 分 45 秒，在日常 TinyWarp 现有窗口的新标签中完成。候选位于 `.note/candidates/V1-semver-dev-opt3/release/bundle/osx/TinyWarp.app`，版本 `v0.1.0-dev.1`、构建号 `1`；签名、arm64 架构、二进制内版本常量、Info.plist 和资源元数据独立回读通过。

隔离临时 HOME 和独立应用身份下，2 标签／3 分屏、2 条测试历史、设置与三份 shell 的外部 `/bin/cat` 输入输出回读通过，随后正常退出，退出码 0。旧日期版 inbox 原文保留，应用跳过它并完成公开源检查，结果为暂无更新。系统沙箱禁止访问真实 TinyWarp／Preview 数据；打开文件回读也未发现访问这些数据库。此验证没有替换日常包或重写现有配置与数据库。

候选来自本阶段工作树，最终仅格式化后提交，不冒充从新发布标签编译的正式包。证据保存在忽略目录 `.note/versioning/`。关于页版本文本已做单元验证；用户实际查看后反馈品牌区域应保留原版样式，本轮恢复原 SVG、尺寸和排版，仅保留 TinyWarp 版本／构建号。修正版在正式候选的隔离实例中已通过原生菜单和截图验收；日期版引导切换已完成，真实 latest 清单及签名匿名下载、独立验签已通过，日常客户端也已通过真实公网下载并重启到 `0.1.0（构建 2）`，未验收 Developer ID、公证和首次下载 Gatekeeper。

## 日常引导安装记录

从日期版 `_02` 正常退出后，复用完整应用原子换包 helper 安装引导版，没有重新编译。安装后二进制 SHA256 为 `acf1eb987c56fffc175b673b94ba58f80d74313d4fefba9177d21ba2b9856a55`，与已验收候选一致；代码签名、应用身份 `dev.warp.WarpOss`、系统产品版本 `0.1.0`、构建号 `1` 独立核对通过。新进程从固定安装路径启动，实际打开原 OSS 数据库，没有打开 Preview 数据库。

安装前与正常退出后分别备份当前配置、偏好和 SQLite 一致性快照；SQLite `quick_check` 均通过。完整旧应用另有安装前备份，换包 helper 保留的旧包为 `/Applications/.tinywarp-previous-X6esHo/TinyWarp.app`。私有证据和数据备份保存在 `.note/versioning-install/`，不进入 Git。

安装后独立回读确认：原 5 个标签、4 个终端 pane、1 个设置 pane、20 条历史正文与稳定字段、10 组已有记录指纹、4 个配置文件和数据库 inode 保留。没有还原旧数据；应用回退仍应继续使用当前数据库。

正常退出的 AppleScript 返回 `-128`，随后核对原主进程和 helper 均已退出，没有强制终止。旧进程的日志文件在退出请求前已经从磁盘移除，未将它记为 SQLite writer 关闭日志证据；以进程退出和退出后一致性快照作为本次检查依据。开始安装时没有运行中的 Preview 进程，本轮没有启动、退出或改写 Preview。

用户已实际查看关于页并反馈恢复原版品牌样式；命令输入尚未收到明确确认，自动回读不替代人工验收。正式 `v0.1.0（构建 2）` 已完成独立构建、签名和用户确认的公开发布，客户端公网升级和数据保留回读已通过；已发现展开菜单的状态刷新遗留，建议下一补丁修正。发布详情见 [U3 首次公开发布记录](u3-public-updates.md)。

规则参考：[Semantic Versioning](https://semver.org/lang/zh-CN/)、[Apple 产品版本字段](https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleshortversionstring)、[Apple 构建版本字段](https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleversion)。
