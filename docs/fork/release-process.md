# TinyWarp CI 与发布维护流程

2026-11-16。适用于当前 macOS Apple Silicon 产品。CI 检查与公开发布分开：CI 不持有发布私钥、不上传应用、不创建或公开 Release；维护者在本机验收候选后，按明确授权执行最后的远端步骤。

## 轻量 CI 的范围

`.github/workflows/tinywarp-ci.yml` 面向本仓库 `main` 的 push、PR 与手动触发，使用一个固定的 `macos-15` 标准 ARM64 runner。权限只有 `contents: read`，checkout 不保留凭据，旧运行可取消，总超时 20 分钟；不启用继承的 Warp runner 矩阵或内部服务。

检查内容：

- `python3 -B script/test_tinywarp_version.py`：版本规则、macOS 字段和开发包拒绝发布。
- `python3 -B script/macos/test_bundle_dock_tile_plugin.py`：编译一个最小 Mach-O 程序及真实 Objective-C Dock 插件，检查插件安装、入口类与完整 PNG 资源，并验证重复安装及损坏包会失败。
- `python3 script/check_tinywarp_features`：当前 macOS host 上的 TinyWarp 产品依赖边界，使用锁文件。
- `cargo test --locked -p warp_i18n` 与该 crate 的严格 Clippy：中文目录的最小 Rust 回归。

无参插件检查是最小原生包 fixture，不构建或运行 TinyWarp 产品，不启动 GUI，不访问用户配置或数据库。保留传入真实 `.app` 的模式，供正式候选验证使用。轻量 CI 不等于整应用已编译、应用级 Rust／GUI 回归通过或 Dock 行为已实测；这些仍属于相应改动与发布候选的本地门禁。

环境使用仓库固定的 Rust 1.92.0 和 runner 自带的 Python、Xcode／clang／make／lipo；不运行全量 bootstrap，不拉取无需使用的 LFS 模型，不安装上游云工具。初版不保存编译缓存或上传 Actions artifact，避免把大型本地 `target` 搬进 CI 存储。GitHub 的日志仍是核对实际 runner、工具版本和检查结果的依据。

GitHub 官方当前将公共仓库的标准 hosted runner 计算列为免费，`macos-15` 为 ARM64、3 核、7 GB RAM、14 GB 磁盘；larger runner 即使在公共仓库也收费，artifact 与 cache 另有存储规则。因此这里不使用 `-large`／`-xlarge`，也不把小 runner 容量当作全应用冷构建已验证的证据。首次云运行尚须按实际日志验收，不能以工作流文件存在宣称 CI 已通过。

依据：[标准 runner](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job)、[计费与存储](https://docs.github.com/en/billing/concepts/product-billing/github-actions)、[macOS 15 ARM 镜像](https://github.com/actions/runner-images/blob/main/images/macos/macos-15-arm64-Readme.md)。runner 和计费规则会变化，调整平台或扩展范围前重新核对。

## 1. 确定版本与可交付源码

1. 在 `tinywarp-version.json` 递增产品版本和 build；已经公开的版本不覆盖、不删除重发，开发包不进入公开 Release。
2. 完成本次相关测试、严格检查与最终格式化。文案修改不重跑整应用；Rust、原生或资源修改应运行对应回归。发布前另验收独立候选的真实 GUI、外部 CLI 与数据保留边界。
3. 按授权提交最终源码，确认工作树干净；候选必须来自这一精确提交。构建方式及日常窗口约定见 [macOS 构建说明](build-macos.md)。

以下命令示例按顺序在专用 Bash 维护会话中执行，先启用遇错退出；任何命令或人工核对失败都停止后续步骤。下面的变量只表示本次发布，不改变全局配置：

```bash
set -euo pipefail
TW_REPOSITORY=sty20030818/TinyWarp
TW_RELEASE_TAG="$(./script/tinywarp-version)"
TW_SOURCE_COMMIT="$(git rev-parse HEAD)"
git status --short
git tag --list "$TW_RELEASE_TAG"
git ls-remote origin "refs/tags/$TW_RELEASE_TAG"
```

工作树输出应为空，新标签在本机和远端均不存在。通过已认证的 GitHub 读取再确认同标签的 Release／草稿不存在；权限或网络错误不能当作“不存在”。保留已发布版本的资产 ID、大小和 digest，供最后检查未覆盖旧资产。

## 2. 构建与组装独立候选

使用 `--locked --no-default-features --features tinywarp_gui`；正式候选使用应用 opt3，不能把 opt0 开发包当正式性能或发布产物。沿用 [现有 SDK／Metal 适配和构建入口](build-macos.md)，不要默认执行全量 bootstrap。

设置 `GIT_RELEASE_TAG="$TW_RELEASE_TAG"`，编译完成后把二进制复制到从未使用的独立候选目录；`script/macos/run --release --dont-open --generate-schema --format osx` 负责组包。传递相同的 `WARP_BIN_NAME=warp-oss`、`WARP_CHANNEL=oss`、`FEATURES=tinywarp_gui`、独立 `CARGO_TARGET_DIR` 和 `CARGO_BUNDLE_SKIP_BUILD=1`。该入口会重建目标 `.app`，因此绝不能指向正在运行的应用或日常安装位置。

开发组包默认跳过许可汇总。正式分发还须使用仓库固定的 `cargo-about 0.8.4`：

```bash
TW_CANDIDATE=/绝对路径/独立候选目录/release/bundle/osx/TinyWarp.app
export GIT_RELEASE_TAG="$TW_RELEASE_TAG"
NO_LICENSES= SKIP_SETTINGS_SCHEMA=1 script/prepare_bundled_resources "$TW_CANDIDATE/Contents/Resources" oss
cp LICENSE-AGPL LICENSE-MIT "$TW_CANDIDATE/Contents/Resources/"
printf 'TinyWarp %s\nSource: https://github.com/%s/tree/%s\nCommit: %s\nLicense: AGPL-3.0-only; see LICENSE-AGPL and THIRD_PARTY_LICENSES.txt.\n' \
  "${TW_RELEASE_TAG#v}" "$TW_REPOSITORY" "$TW_SOURCE_COMMIT" "$TW_SOURCE_COMMIT" \
  > "$TW_CANDIDATE/Contents/Resources/SOURCE.txt"
```

核对 `THIRD_PARTY_LICENSES.txt` 非空且内容有效、两份项目许可和 `SOURCE.txt` 与当前源码一致，再完成对应分发方式的签名。

以下 ad hoc 命令仅适用于尚未接入 Developer ID 的当前流程，不代表正式 Apple 分发已通过。不要对已用 Developer ID 签名、公证或附票的候选执行它，也不要套用 Debug entitlements 把正式签名退回开发状态：

```bash
codesign --force --deep --sign - --entitlements script/Debug-Entitlements.plist "$TW_CANDIDATE"
```

接入正式 Apple 分发时，改按 [macOS 正式分发流程](macos-distribution.md) 完成嵌套代码与外层应用的 Developer ID 签名、生产 entitlements、hardened runtime、timestamp、公证和应用附票；不执行上面的 ad hoc 重签。两条路径都要对最终候选核对签名和插件包：

```bash
codesign --verify --deep --strict "$TW_CANDIDATE"
python3 -B script/macos/test_bundle_dock_tile_plugin.py "$TW_CANDIDATE"
```

另核对 Info.plist、资源版本、原应用身份、arm64、设置 schema、嵌入字标与图标插件。相关 GUI／会话验收使用临时 HOME、独立测试身份及真实数据路径保护；不通过启动同一 OSS 身份的第二实例来试验，不向日常更新 inbox 投递候选遮蔽公网来源。SSHSIG 不代替 Apple Developer ID、公证或 Gatekeeper 验收。

## 3. 本机离线准备三个公开文件

`script/tinywarp-release` 已承担版本、Dock 插件包、私钥权限／公钥匹配、应用签名、归档及 SSHSIG 检查。它没有自动实现 Developer ID／公证门禁：走正式 Apple 链时，必须先完成 [distribution 签名、公证与 staple](macos-distribution.md)，再把这份最终应用交给本节；不能在生成最终 ZIP／SHA256／清单后才附票或重签。私钥继续留在本机受保护位置，不进入 CI、日志、Git 或发布包。

```bash
TW_OUTPUT=/绝对路径/尚不存在的发布产物目录
script/tinywarp-release "$TW_CANDIDATE" --output "$TW_OUTPUT"
```

这个命令只生成文件，不上传、不推送、不替换应用。输出恰好为：

- `TinyWarp-aarch64.zip`
- `tinywarp-update-aarch64.json`
- `tinywarp-update-aarch64.json.sig`

独立回读 ZIP SHA256，确认等于清单 `sha256`；清单版本为本次正式标签、架构为 `aarch64`。检查归档只包含应用及系统归档元数据，没有用户配置、数据库或发布私钥，包中版本、许可、源码定位与候选一致。记录三个文件的大小与 SHA256，不以文件名正确代替内容核对。

用仓库公开密钥独立验签：

```bash
TW_ALLOWED_SIGNERS="$(mktemp)"
printf 'tinywarp namespaces="tinywarp-updates" %s\n' "$(cat resources/tinywarp-updates.pub)" > "$TW_ALLOWED_SIGNERS"
ssh-keygen -Y verify -f "$TW_ALLOWED_SIGNERS" -I tinywarp -n tinywarp-updates \
  -s "$TW_OUTPUT/tinywarp-update-aarch64.json.sig" < "$TW_OUTPUT/tinywarp-update-aarch64.json"
shasum -a 256 "$TW_OUTPUT/TinyWarp-aarch64.zip" \
  "$TW_OUTPUT/tinywarp-update-aarch64.json" "$TW_OUTPUT/tinywarp-update-aarch64.json.sig"
```

## 4. 明确授权后的远端步骤

至此应已有可审阅的精确提交、候选验收结果、三个文件与发布说明。只有本次发布范围已经明确授权时才执行下面的写入；不要把“准备候选”自动解释为允许推送或公开。

再次核对源码、当前分支和远端版本未变化，按约定创建注释标签并原子推送本次 `main` 与标签；不强推、不移动已发布标签。上传时只创建一份草稿，不使用 `--clobber`：

```bash
gh release create "$TW_RELEASE_TAG" --repo "$TW_REPOSITORY" --verify-tag \
  --target "$TW_SOURCE_COMMIT" --draft --title "TinyWarp ${TW_RELEASE_TAG#v}" \
  --notes-file /绝对路径/已审阅的发布说明.md \
  "$TW_OUTPUT/TinyWarp-aarch64.zip" "$TW_OUTPUT/tinywarp-update-aarch64.json" \
  "$TW_OUTPUT/tinywarp-update-aarch64.json.sig"
```

上传完成后，用已认证的 API 读取草稿，核对 `draft=true`、`prerelease=false`、标签、说明、精确源码目标及恰好三个资产。每个资产必须 `state=uploaded`，大小与服务端 `digest` 等于本地记录。草稿标签 URL 返回 404 时，通过已认证的 Releases 列表定位同标签草稿 ID，再按 ID 回读；不重复创建或覆盖资产。

```bash
gh api "repos/$TW_REPOSITORY/releases?per_page=100" \
  --jq '.[] | {id,tag_name,draft,prerelease,target_commitish,assets:[.assets[]|{name,size,state,digest}]}'
git ls-remote origin "refs/tags/$TW_RELEASE_TAG" "refs/tags/$TW_RELEASE_TAG^{}"
```

注释标签的 peeled commit 必须等于 `TW_SOURCE_COMMIT`，不能只检查标签名字。任何条件不符都保持草稿并查明原因。全部通过后，公开并设为 Latest 是独立的最后一步：

```bash
gh release edit "$TW_RELEASE_TAG" --repo "$TW_REPOSITORY" --draft=false --latest
```

## 5. 公开回读与交付记录

公开后再次回读 Release／Latest API，确认非 Draft／Prerelease、源码和三个资产仍一致、旧发布资产未改变。匿名请求禁止读取 curl 用户配置，不传认证头，不输出跳转得到的临时下载地址：

```bash
TW_ANONYMOUS="$(mktemp -d)"
for TW_ASSET in tinywarp-update-aarch64.json tinywarp-update-aarch64.json.sig; do
  curl -q --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
    --max-time 50 --max-filesize 4096 --output "$TW_ANONYMOUS/$TW_ASSET" \
    "https://github.com/$TW_REPOSITORY/releases/latest/download/$TW_ASSET"
  cmp "$TW_OUTPUT/$TW_ASSET" "$TW_ANONYMOUS/$TW_ASSET"
done
ssh-keygen -Y verify -f "$TW_ALLOWED_SIGNERS" -I tinywarp -n tinywarp-updates \
  -s "$TW_ANONYMOUS/tinywarp-update-aarch64.json.sig" \
  < "$TW_ANONYMOUS/tinywarp-update-aarch64.json"
```

旧客户端仍内置 `sty20030818/warp` 地址时，对旧地址的 latest 清单与签名重复相同匿名核对；不要重新创建占用旧名字的仓库。首次发布、更换更新源或传输路径时，另从实际公开 URL 完整下载 ZIP 并核对大小与 SHA256。普通补丁若只做服务端 digest 与匿名清单验签，应明确没有重跑完整客户端下载安装，不能将轻量回读表述为日常升级已验收。

同步发布记录、README 与路线，分别写清源码提交、实际运行检查、复用证据、三个资产、匿名回读和用户待验收项。私有日志／数据备份留在忽略目录，公开记录不包含用户数据库或凭据。用户准备好后通过日常应用检查更新；升级前备份与升级后数据回读不通过自动恢复旧数据库完成。
