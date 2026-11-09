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
