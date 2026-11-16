# TinyWarp 支持与反馈

TinyWarp 首阶段聚焦 macOS Apple Silicon。Intel Mac、Windows 和 Linux 尚不列入产品支持范围；Warp 官方的支持承诺不适用于本 fork。

## 系统与安装

当前发布包为 arm64。2026-11-16 仅核实本机 macOS 27.3 的运行情况；包内主程序的 Mach-O 编译下限为 11.0，这不证明所有 macOS 11 及以上系统都已兼容。正式最低支持版本需稳定系统上的安装、终端、恢复与更新验收后再确定。

首次安装从 [TinyWarp Releases](https://github.com/sty20030818/TinyWarp/releases/latest) 获取 `TinyWarp-aarch64.zip`，解压后将应用放入 Applications。当前公开包仍为 ad hoc 签名，没有 Developer ID／Apple 公证；首次下载的 Gatekeeper 验收尚未通过。遇到系统阻止启动时，请记录完整提示并反馈，不把关闭 Gatekeeper 或清除隔离属性作为安装步骤。正式分发要求及尚缺条件见 [macOS 分发说明](docs/fork/macos-distribution.md)。

已有安装使用应用内“检查更新”。更新包沿用应用身份、设置与本地数据路径，不需要 Warp 账号；运行中的命令需在重启前自行处理。Tab／分屏布局恢复不代表已退出的 shell、SSH 或 CLI 进程继续运行。

## 使用问题与需求

在 [TinyWarp Issues](https://github.com/sty20030818/TinyWarp/issues) 提交前先搜索已有问题。请提供：

- TinyWarp 版本与构建号、macOS 版本、Apple 芯片型号。
- 最小操作步骤、预期与实际结果、问题出现频率。
- 必要的脱敏截图或日志；会话问题请区分直接退出应用、先关闭窗口／标签，以及异常退出。

请勿上传私人命令历史、真实数据库、访问令牌或密钥。安全问题使用 [私密漏洞渠道](SECURITY.md)，不要放入公开 Issue。

维护者按影响与可复现性处理反馈，目前没有固定响应时限、商业支持或历史版本回补承诺。产品方向与未完成事项见 [路线](ROADMAP.md)；当前没有全应用键盘或 VoiceOver 可访问性通过声明。
