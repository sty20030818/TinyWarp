# TinyWarp U3：公开更新源与发布签名

沿用 U1/U2 的菜单、状态机、会话保存和安装 helper，仅增加公开来源与清单认证。普通安装者不需要 Warp／GitHub 账号，不需要创建或保管签名密钥。本阶段不推送、不上传 Release，不更换日常应用或数据。

## 来源与信任

公开源固定为个人仓库 `https://github.com/sty20030818/warp/releases`，当前本机架构为 `aarch64`：

- 清单：`/latest/download/tinywarp-update-aarch64.json`
- 独立签名：`/latest/download/tinywarp-update-aarch64.json.sig`
- 压缩包：`/download/<已验证版本>/TinyWarp-aarch64.zip`

清单只有 `version`、`sha256` 和 `architecture` 三个字段，拒绝未知字段。清单不提供任意下载 URL；包地址由固定仓库、已验证版本和本机架构推导。

使用 `http_client::Client::new_external()` 匿名 HTTPS 请求，不复用 Warp、IAP 或浏览器凭据。清单及签名分别限制为 4 KiB、请求超时 20 秒；包流式写入私有文件，限制 1 GiB、总请求超时 600 秒。清单 404 表示暂无公开发布；签名缺失、非成功响应、超限、字段错误或验签失败都终止此次检查／准备，不能回退到未签名远程包。

应用内置 [发布公钥](../../resources/tinywarp-updates.pub)，通过系统 `/usr/bin/ssh-keygen -Y verify` 校验原始清单字节，固定签名身份 `tinywarp` 和 namespace `tinywarp-updates`。验证通过后才解析版本、架构与 SHA256；清单与签名在同一个缓存文件中原子保存，下载前重新验签。下载后继续执行 U1/U2 的压缩包哈希、应用身份、资源版本、代码签名及架构检查。

本机手工投递入口仍保留：只有比当前版本更新的本地收件箱优先于公开源。此入口信任当前用户的私有本地文件；它不是未签名的远程回退通道。没有更新的本地收件箱不会永久阻止公开检查。

## 发布者配置与离线产物

专用发布私钥只在本机：

```text
~/Library/Application Support/TinyWarpRelease/update-signing
```

目录权限 0700，私钥 0600；没有复用 `~/.ssh` 的登录密钥。仓库只提交公钥，应用从编译时常量读取它，不读取发布私钥。私钥决定哪些更新可被已有安装信任，需由发布者妥善备份；不要提交到 Git 或直接作为公开 Release 文件。丢失后不能签发当前安装信任的新版本，需重新引导安装新的信任版本。当前没有自动密钥轮换或 CI 私钥上传配置。

为已构建、含当前公钥的候选生成签名发布文件：

```sh
script/tinywarp-release /绝对路径/TinyWarp.app --output /不存在的发布产物目录
```

工具先核对私钥所有权／权限、公私钥匹配、候选二进制包含当前公钥，再复用本地打包校验；对原始清单签名后立即独立验签。输出目录已存在则拒绝覆盖。产物只有以下三类，不包含私钥、用户配置或数据库：

```text
TinyWarp-aarch64.zip
tinywarp-update-aarch64.json
tinywarp-update-aarch64.json.sig
```

发布时需要将这三份文件放到清单 `version` 对应的同一个 GitHub Release，并使它成为 latest 正式版本。这里只准备文件，没有运行 `gh release create/upload`、创建远程 tag、推送或自动发布工作流。发布前仍需核对具体版本、文件、仓库与分发方式。

## 验证状态

10 项更新／签名／传输边界回归通过，覆盖有效清单与缓存字节保留、正文篡改、错误公钥、错误 namespace、签名有效但字段错误、匿名请求无账号头、清单 404、签名缺失、HTTP 失败和超限响应，以及原有校验和、私有权限、可逆换包。有限生命周期的本地 HTTP 夹具在测试完成后退出，不启动常驻服务。

严格 Clippy 和 Python 语法检查通过，无新增依赖或锁文件变更。2026-11-12 本机匿名请求当前公开清单返回 HTTP 404；没有把不存在的公开发布记成在线安装通过。优化构建在 Preview 同一窗口的新标签完成，用时 9 分 35 秒，候选为 `.note/candidates/U3-opt3/release/bundle/osx/TinyWarp.app`，版本 `v0.2026.11.12.00.30.tinywarp_01`。候选来自 `68240bd6a` 加本阶段工作树变更；最终仅格式化后提交，不将旧 Git 元信息冒充新的已打 tag 公开版本。

离线发布工具已实际生成三份文件并独立验签；压缩包大小 128,602,841 字节，SHA256 `b0645082ae11871b0714114e78c07f190be9d697d26f06d80188dd24c1d70e33`。独立读取 ZIP 核对了完整哈希、应用身份、Info.plist 与资源版本、仅含应用内容且没有用户数据库／设置或私钥。首轮文件名过滤将两个上游 `_settings.toml` 测试夹具误判为用户设置；随后按精确文件名检查并与仓库夹具逐字节核对，没有删除或隐藏这些资源。

优化候选在临时 HOME、独立测试身份与真实数据路径禁令下启动，通过 2 标签、3 分屏、2 条测试历史、文件设置和三个 shell 的外部 `/bin/cat` 字节输入输出回读；实际打开独立测试数据库，没有打开日常或 Preview 数据库。匿名公开检查完成并进入“暂无更新”。另以独立进程会话运行 40 秒后正常退出，退出码 0；追加检查最初因脚本相对路径找不到 sandbox 文件未启动，修正为绝对路径后通过。此结果不是长期稳定性或内存测试。

真实终端截图已取得；齿轮菜单与取消由 U1/U2 用户实际操作确认。版本行缩短／宽度调整已进入本候选，最后外观仍待复看，不将未成功的后续自动点击记为界面通过。发布产物和证据在 `.note/u3/`（`release-result.json`、`package-readback.json`、`native-result.json`、`hold-native-result.json`）；不提交这些私有临时文件。

U1/U2 已验证的真实界面更新、取消、会话保留与失败回退继续作为安装器基线。U3 没有修改 SQLite schema、会话快照协议或换包 helper。

## 尚未交付的边界

日常 `/Applications/TinyWarp.app` 仍为 D1，不包含新更新器；需要一次正常退出后的引导安装，保留原 OSS 身份、设置和数据库。以后才可使用应用内的重启更新。

本阶段不是公开首发验收：尚未上传公开资产，完整“公开下载 → 重启安装”还需在发布后验证。当前候选仍为本机 ad hoc 代码签名，没有 Apple Developer ID／公证／首次下载 Gatekeeper 的验收。SSHSIG 负责应用内更新来源认证，不替代 Apple 的分发签名。

参考：[GitHub latest Release 资产链接](https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases)、[OpenSSH ssh-keygen 签名与验证](https://man.openbsd.org/ssh-keygen)、[Apple 公证说明](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)。实际代码以本仓库固定实现和已运行测试为准。
