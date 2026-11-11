# TinyWarp macOS 构建与原版 OSS 基线记录

日期：2026-11-09。源码基线：`abb3fe33e004f7b1de540fecd6e3eec20dbc1883`。

## 环境准备

本机已有 Xcode 27.2、Apple Silicon 和基础 Rust/Cargo。经用户单独确认，本轮补充 Rust 1.92.0（不改变默认工具链）、Git LFS 3.8.1、protobuf 36.2、pkgconf 3.0.10、cmake 4.4.5、Apple MetalToolchain 27B47，以及仓库固定的 cargo-binstall 1.18.1 / cargo-bundle 0.11.0。Homebrew 同时安装 protobuf 的 abseil 依赖。没有运行全量 bootstrap，也没有安装 Docker、gcloud、PowerShell 或 common agent skills。

Git LFS 只在本仓库初始化，已下载上游资源。`origin` 指向个人 fork，`upstream` 指向 `warpdotdev/warp`；没有更改全局 Git 配置。

## 本机 Metal 查找问题

`xcodebuild -downloadComponent MetalToolchain` 安装成功，实际 metal/metallib 可执行，但本机 XcodeDefault 的 `xcrun metal` 启动器仍报组件缺失。仅本次构建使用局部 xcrun 转发；其他 xcrun 调用保持系统实现。已实际编译上游原版 shader 并生成非空 metallib，不使用空 shader 或跳过 Metal 的检查模式。

辅助文件保存在被 Git 忽略的 `target/local-tools/xcrun`，清理 target 后需重建；没有修改项目 build.rs 或全局 Xcode 配置。其内容为：

```bash
#!/bin/bash
set -euo pipefail
if [[ $# -ge 3 && "$1" == "-sdk" && "$2" == "macosx" && ( "$3" == "metal" || "$3" == "metallib" ) ]]; then
    tool="$3"
    shift 3
    metal_bin="$(/usr/bin/xcrun --find metallib)"
    exec "${metal_bin%/*}/$tool" "$@"
fi
exec /usr/bin/xcrun "$@"
```

## 原版 OSS 基线的构建与打包命令（历史）

在仓库根目录运行。首次实际构建使用了内容相同的 `/private/tmp/warp-oss-build-tools-2026-11-09/xcrun`；以下路径改为本仓库保留的 helper，方便复用。

```bash
PATH="$PWD/target/local-tools:$PATH" \
SDKROOT="$(/usr/bin/xcrun --sdk macosx --show-sdk-path)" \
LIBCLANG_PATH=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib \
FRAMEWORK_OVERRIDE=dev \
CARGO_TARGET_DIR="$PWD/target" \
cargo build --locked --release --bin warp-oss --features gui -j 2

WARP_BIN_NAME=warp-oss WARP_CHANNEL=oss FEATURES=gui \
CARGO_TARGET_DIR="$PWD/target" CARGO_BUNDLE_SKIP_BUILD=1 \
./script/macos/run --release --dont-open --generate-schema

open "$PWD/target/release/bundle/osx/WarpOss.app"
```

以上是改名前基线阶段执行的命令，`WarpOss.app` 是该阶段产物；当前 TinyWarp 应使用下一节的独立候选路径。保留上游默认 feature，不提前实施精简。先 Cargo `--locked` 编译，再由官方脚本跳过重复编译组装并签名；cargo-bundle 本身不接受 `--locked`。构建并发设为 2，避免在 16 GB 机器上同时启动太多编译任务。

这是本机开发包。上游开发脚本设置 `NO_LICENSES=1`，不会生成正式分发用的第三方许可证汇总；当前不发布、不公证或上传该应用。

## 验证边界

- 启动前核对 bundle ID `dev.warp.WarpOss`、URL scheme `warposs`、Debug entitlements、签名和资源。
- Preview 的应用身份是 `dev.warp.Warp-Preview`，配置在 `~/.warp-preview`；OSS 配置在 `~/.warp-oss`。Keychain 使用独立 service，未读取已有凭证。
- 记录 Preview 应用、设置与键位的 SHA-256；会话 SQLite/WAL 只取文件元信息，不读取内容，也不迁移到 OSS。
- OSS 首次运行后检查进程、窗口及自身新数据目录。静态文件一致不等于所有历史记录逐条验收；进程和窗口存在也不等于完成中文输入、长时间稳定性或性能验收。
- 不主动退出 Preview，不通过关闭用户现有会话来复现 Tab 丢失问题。

实际构建和启动结果见 [本轮构建记录](README.md#本轮构建记录)。

## TinyWarp H1 独立候选

当前显示品牌为 TinyWarp，保留 OSS 身份。不要用默认目标目录覆盖当前运行的基线包；编译复用已有缓存，组装另一个候选目录。以下命令在仓库根目录运行：

```bash
PATH="$PWD/target/local-tools:$PATH" \
SDKROOT="$(/usr/bin/xcrun --sdk macosx --show-sdk-path)" \
LIBCLANG_PATH=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib \
FRAMEWORK_OVERRIDE=dev \
CARGO_TARGET_DIR="$PWD/target" \
cargo build --locked --release --bin warp-oss --features gui -j 2

mkdir -p .note/candidates/H1/release
cp target/release/warp-oss .note/candidates/H1/release/warp-oss

WARP_BIN_NAME=warp-oss WARP_CHANNEL=oss FEATURES=gui \
CARGO_TARGET_DIR="$PWD/.note/candidates/H1" CARGO_BUNDLE_SKIP_BUILD=1 \
./script/macos/run --release --dont-open --generate-schema --format osx
```

产物为 `.note/candidates/H1/release/bundle/osx/TinyWarp.app`。如果该候选已经在运行，后续构建应改用新的候选目录，或先妥善退出它；不要覆盖正在运行的 `.app`。不要在旧 OSS 尚有运行中任务时直接打开同一身份的新候选；它们使用同一个 OSS 数据目录，需先妥善结束旧 OSS。Preview 使用另一身份，继续并存。

本阶段的实际验证和人工验收边界见 [H1 交付记录](h1-delivery.md)。

## 日常开发与快速验证

用户已明确要求固定复用同一个 Warp Preview 构建窗口／标签，不再每轮新开。先核对 `.note/tinywarp-build-session.json` 记录的应用、shell PID 和 TTY 仍属于构建会话，确认上一条任务已经结束后，在该会话执行下一条命令；当前任务运行中先等待。激活 Preview 后先等待窗口恢复，不能把暂时读不到 AX 窗口当作会话不存在；仅在确认原会话已关闭时新建。目标不明确时先核对窗口，不向其他项目缓冲输入。该记录只用于本机会话定位，不提交，也不更改 Preview 设置。

改主题、字体、设置或快捷键时，已有文件监听可重载相应配置；修改 Rust 界面或程序逻辑仍需编译并重启，没有现成的源码热替换流程。不要为普通逻辑修复默认执行完整正式 release。

仓库 `./script/run` 默认使用 Cargo dev profile，macOS 开发配置使用 `split-debuginfo="unpacked"` 避免生成 packed dSYM。Cargo dev 默认支持增量编译，见 [Cargo profiles 官方说明](https://doc.rust-lang.org/stable/cargo/reference/profiles.html)。首次切换 profile 仍可能补建依赖，不能承诺秒级构建。

本机已有完整 release 依赖缓存时，可沿用本文的 SDK／Metal 环境适配，仅覆盖应用包的编译参数：

```bash
cargo build --locked --release -p warp --bin warp-oss --features gui -j 2 \
  --config profile.release.package.warp.opt-level=0 \
  --config profile.release.package.warp.debug=0 \
  --config 'profile.release.package.warp.split-debuginfo="unpacked"' \
  --config profile.release.package.warp.debug-assertions=false \
  --config profile.release.package.warp.overflow-checks=false
```

虽然沿用 release 依赖缓存，上述应用本身为无优化开发产物，不能用于性能对比或当作最终正式 release。它仍需链接，并非热更新。构建后复制到未使用的独立候选目录，用前述 `CARGO_BUNDLE_SKIP_BUILD=1` 与 `--dont-open --format osx` 组包；不覆盖运行中的 `.app`，也不通过强退项目任务实现自动重启。

验证使用本次改动相关的测试与检查；普通修复先使用开发候选，正式 release 留到明确的阶段验收／发布需要。没有启用常驻自动构建或 watcher。设置入口的具体复现、修复和开发候选证据见 [L2 设置入口修复](l2-settings-fix.md)。本轮快速开发应用编译实测 2 分 43 秒（不含测试构建与验收）；不是后续每次的耗时保证。

## L3C1 之后的 TinyWarp 构建选择

当前 TinyWarp 使用 `--features tinywarp_gui`。上面的原版／H1／设置修复命令是历史记录，显式使用 `gui` 仍会引入语音。`script/run` 的 OSS 默认值及 macOS 分发脚本的 OSS app 分支已更新；其他通道沿用 `gui`。

继续沿用本文 SDK／Metal 环境；构建正式优化候选时：

```bash
cargo build --locked --release -p warp --bin warp-oss --no-default-features --features tinywarp_gui -j 2
```

普通开发可继续附加上一节列出的应用 opt0 覆盖，避免默认重做正式优化构建。组包时也必须传 `FEATURES=tinywarp_gui`，并选择从未使用的候选目录，复制对应二进制后用 `CARGO_BUNDLE_SKIP_BUILD=1`、`--dont-open --generate-schema --format osx` 组包。不要覆盖运行中的应用或直接打开同一 OSS 身份的第二实例。

依赖裁剪、开发包与验收边界见 [L3C1 交付记录](l3c1-delivery.md)。


## L3C2 的默认能力拆分

从 C2 起，直接调用 Cargo 必须同时使用 `--no-default-features --features tinywarp_gui`；只加 `tinywarp_gui` 会与完整默认叠加，重新引入 AWS SDK。`script/run` 的 OSS 分支、`script/macos/run` 的 TinyWarp 选择和 macOS 分发脚本的 OSS app 分支已自动传递该参数；其他通道及独立 CLI／TUI 仍使用完整默认。

构建前可快速检查依赖边界：

```bash
python3 script/check_tinywarp_features
```

这里检查当前主机的 normal／build 依赖图，不运行应用或读取终端数据。SDK 缺席时，TinyWarp 内置 Bedrock 凭据请求明确返回不可用；正常外部 CLI 不变。完整范围与验证见 [L3C2 交付记录](l3c2-delivery.md)。

## L3C3 的图表渲染边界

TinyWarp 的 Cargo 参数继续使用 `--no-default-features --features tinywarp_gui`。`app_defaults` 只保留共享默认能力；完整默认另行启用 `markdown_mermaid`，该 feature 同时启用应用剪贴板 SVG 导出和 `warp_editor/mermaid`。TinyWarp 不生成 Mermaid SVG，源码和代码块格式继续保留。

`warp_editor` 独立构建默认仍启用 `mermaid`；workspace 引用关闭其默认 feature，由实际应用的渲染能力显式选择。`languages` 与 `syntax_tree` 使用编辑数据／语法类型，不需要渲染器；原版 GUI、TUI 通过完整应用默认继续启用渲染。依赖边界脚本同时防止 Mermaid、图布局专用库回流。详见 [L3C3 交付记录](l3c3-delivery.md)。

## L3C4 的内置 JSON 过滤边界

构建参数不变：TinyWarp 使用 `--no-default-features --features tinywarp_gui`。完整应用默认额外启用 `agent_cli_jq`，将应用过滤输出与 `warp_cli/jq` 一起开启。`warp_cli` 的独立默认仍含 jq；workspace 消费者由应用 feature 选择。精简版的内置 `--jq` 解析会明确失败，普通 JSON、终端和外部 CLI 沿用原路径。详见 [L3C4 交付记录](l3c4-delivery.md)。

## L3C5 的本地 GUI 数据范围

构建参数不变。LocalGui 是按现有本地终端策略选择的启动读取范围，保留 GUI 恢复、历史、本地项目路径与忽略的 shell 建议，不能替换成会跳过 GUI 恢复的 TuiFrontend。原版非本地 GUI／CLI、TUI 和 daemon 的选择不变；完整构建中的 OSS 本地 GUI 也采用 LocalGui。无 schema 或依赖清单调整，详见 [L3C5 交付记录](l3c5-delivery.md)。
