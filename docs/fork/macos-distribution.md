# TinyWarp macOS 支持与正式分发

本文管理 TinyWarp 的系统支持证据、首次下载安装，以及后续 Developer ID 签名和公证流程。当前分发方式继续采用 GitHub Release 的应用 ZIP 与独立签名更新清单；不引入安装器、修改系统安全设置或迁移应用数据。

## 2026-11-16 的实际状态

| 对象 | 只读核对结果 | 能说明什么 |
| --- | --- | --- |
| 开发机 | macOS 27.3，构建 26C5035r，arm64 | 这是已观察的本机环境，不是所有 macOS 版本的支持证据 |
| 日常 0.1.2／构建 5 | 主程序 arm64；Mach-O `minos 11.0`、SDK 27.2；ad hoc 签名，无 TeamIdentifier，未启用 hardened runtime | 已发布程序的实际格式；不证明首次下载可通过 Gatekeeper |
| 图标修正开发候选 | 主程序 arm64、`minos 11.0`、SDK 27.2；ad hoc + hardened runtime，无 TeamIdentifier | 可做开发验证，不能作为 Developer ID 正式分发包 |
| 候选 Dock 插件 | 含 arm64／x86_64；对应 `minos` 为 11.0／10.14 | 插件包含 Intel 切片不代表主程序支持 Intel |
| 应用 Info.plist | 日常包与候选均未设置 `LSMinimumSystemVersion` | 尚未在应用元数据中明确最低支持版本 |
| 本机有效代码签名身份 | 当前钥匙串搜索链只有本地开发身份，没有有效 `Developer ID Application` | 不能在这台机器上按当前配置执行正式 Developer ID 签名；不代表发布者没有会员或其他机器上的证书 |
| 本机工具 | `notarytool`、`stapler`、`syspolicy_check` 均可找到 | 工具存在，不等于证书、公证凭据或正式分发验收已就绪 |

当前可以说明发行物面向 macOS Apple Silicon，并列出实际测试环境；不能声明 Intel、macOS 10.14、macOS 11 及以上所有版本已经支持，也不能声明已经通过首次 Gatekeeper 安装。

### 编译下限和产品支持范围分开记录

`.cargo/config.toml` 与上游 `script/macos/bundle` 的 `MACOSX_DEPLOYMENT_TARGET` 为 `10.14`，但本轮实际 arm64 主程序的 Mach-O 下限为 `11.0`。维护者应读取产物，而不是从环境变量直接生成支持承诺。

Mach-O 下限是装载约束，不能证明运行时 API、Metal、字体、PTY、Dock 插件和会话恢复在该系统上可用。后续先选定一台正式稳定 macOS 的 Apple Silicon 机器或 VM，完成下面的安装与升级基线；再在拟定最低版本上重复验证。只有形成对应结果后，才确定对外最低支持版本，并同步构建参数、`LSMinimumSystemVersion` 和支持说明。[Apple 的字段定义](https://developer.apple.com/documentation/bundleresources/information-property-list/lsminimumsystemversion)说明该字段用于声明应用支持的最低 macOS。

核对应用产物可使用：

```sh
plutil -p "$TINYWARP_APP/Contents/Info.plist"
lipo -archs "$TINYWARP_APP/Contents/MacOS/warp-oss"
vtool -show-build "$TINYWARP_APP/Contents/MacOS/warp-oss"
codesign -dv --verbose=4 "$TINYWARP_APP"
codesign -d --entitlements :- "$TINYWARP_APP"
```

其中 `TINYWARP_APP` 指向本轮独立候选，不指向日常运行包。插件及其他新增 Mach-O 也须逐个读取架构和下限；主程序结论不能代替整个包。

## 正式签名前需要补齐的条件

当前 `script/macos/run` 选择 Apple Development 证书，找不到则使用 ad hoc，并套用 `script/Debug-Entitlements.plist`。这条路径用于开发。日常 0.1.2 与图标修正候选的实际签名权限均含 `com.apple.security.get-task-allow=true`，不能直接作为正式公证输入。

上游 `script/macos/bundle` 的生产路径依赖 Warp 的证书、团队及 GCP 配置；`script/Entitlements.plist` 还包含上游 App Group。TinyWarp 不能通过开启该路径直接继承这些身份和权限。

后续接入前完成两件事：

1. 发布者明确所用 Apple Developer 团队，并在发布机上提供可用的 `Developer ID Application` 身份及公证认证方式。由用户处理账号、证书和凭据，不自动注册、采购或上传私钥。当前只有应用 ZIP，不需要为此新增 `Developer ID Installer`；将来采用 `.pkg` 时再评估。[Apple 证书说明](https://developer.apple.com/help/account/certificates/create-developer-id-certificates)区分了应用与安装包证书及创建角色。
2. 根据 TinyWarp 当前消费者审阅生产 entitlements，形成独立于 Debug 的最小权限文件。不得保留 `get-task-allow=true`，不得原样使用 Warp App Group。摄像头、麦克风、联系人、Apple Events、library validation 等既有权限逐项按实际路径判断；验证 PTY、外部 CLI、设置、图标插件和更新后再确认，不在本轮文档工作中直接删改权限。

[Apple 公证要求](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)包括有效 Developer ID 签名、hardened runtime、secure timestamp，以及禁止开启 `get-task-allow`。本地开发证书、ad hoc 签名或单独启用 runtime 均不能替代这些要求。

当前没有可用 Developer ID 身份，也尚未形成 TinyWarp 生产权限配置，因此下面是待接入的维护步骤，不是已经执行成功的流水线。本文不新增签名脚本或权限文件。

## 后续正式候选的处理顺序

沿用 [构建说明](build-macos.md)、[版本规则](tinywarp-versioning.md) 和 [公开更新方案](u3-public-updates.md)。每次从干净的最终源码提交构建，使用未占用的独立候选目录，不覆盖正在运行的应用。

### 1. 完成组包，再签名

先把版本、项目及第三方许可、源码定位、资源和 Dock 插件完整放入 `.app`。确认主包和插件的注册、架构、资源以及主应用身份一致。保持 `dev.warp.WarpOss`、现有配置／数据库路径和更新公钥；Apple 签名接入不同时承担数据迁移。

枚举所有嵌套代码，先签最内层代码，再签外层应用。当前图标修正候选包含主程序与 `WarpDockTilePlugin.docktileplugin`；若后续出现 helper、framework 或其他可执行文件，先更新实际代码清单和签名次序。不要依赖 `codesign --deep` 代替明确的逐项签名。

签名阶段使用已核对的 Developer ID Application 身份，主程序启用 hardened runtime，使用已审阅的 TinyWarp 生产 entitlements，并加入 secure timestamp。插件使用相同团队身份；其权限与宿主关系也须单独验证。签名后回读每个代码对象的身份、TeamIdentifier、timestamp、runtime 和 entitlements，再运行：

```sh
codesign --verify --deep --strict "$TINYWARP_APP"
```

核对失败就停止本次候选，不回退成 ad hoc 后继续公开。资源或权限再次改变时，重新签名和验证。[Apple 分发说明](https://developer.apple.com/documentation/xcode/packaging-mac-software-for-distribution)要求先签内部内容，再处理外层容器。

### 2. 提交公证并回读结果

把完成签名的 `.app` 用 `ditto -c -k --keepParent` 生成临时 ZIP，使用 `notarytool submit` 和已配置的 Keychain profile 提交，等待状态并保留 submission ID。以下变量均由维护者在本次发布环境中提供，不是仓库内置配置：

```sh
ditto -c -k --keepParent "$TINYWARP_APP" "$TINYWARP_NOTARY_ZIP"
xcrun notarytool submit "$TINYWARP_NOTARY_ZIP" \
  --keychain-profile "$TINYWARP_NOTARY_PROFILE" --wait
xcrun notarytool log "$TINYWARP_SUBMISSION_ID" \
  --keychain-profile "$TINYWARP_NOTARY_PROFILE" "$TINYWARP_NOTARY_LOG"
```

必须确认 `Accepted` 并检查日志中的错误及警告，上传成功不等于公证通过。认证信息不进入脚本、仓库、构建日志或发布说明。若网络超时导致结果未知，先按 submission ID 查询状态，不盲目重复提交。[Apple 自定义公证流程](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)说明了认证、日志和 ticket 的处理。

### 3. 给应用附票，再生成最终更新资产

公证接受后，给 `.app` 附加并验证 ticket：

```sh
xcrun stapler staple "$TINYWARP_APP"
xcrun stapler validate "$TINYWARP_APP"
codesign --verify --deep --strict "$TINYWARP_APP"
syspolicy_check distribution "$TINYWARP_APP"
```

`syspolicy_check` 适用于 macOS 14 及以上；旧测试系统可补充 `spctl --assess --type execute -vv`，但均不能替代首次启动路径。ZIP 本身不能 staple；给其中的应用附票后，必须重新生成最终 ZIP。

最后才对这份已附票应用运行现有 `script/tinywarp-release`，输出目录必须不存在。工具生成：

- `TinyWarp-aarch64.zip`
- `tinywarp-update-aarch64.json`
- `tinywarp-update-aarch64.json.sig`

保持“最终应用 → 最终 ZIP → SHA256 → 清单 → SSHSIG”的顺序，不使用临时公证 ZIP 的哈希，不在生成清单后修改应用或压缩包。当前离线发布工具负责更新协议和签名检查，并未自动实现本文的 Developer ID／公证门禁；不能把它成功运行当成 Apple 分发验证通过。

SSHSIG 认证 TinyWarp 的更新来源及包哈希，Apple 签名／公证服务于 macOS 的分发信任，两者都保留。正式公开前继续核对包内容、三份资产、本地与服务端哈希、匿名 latest 清单及签名；不覆盖已经发布的同版本。

## 首次安装和旧版升级验收

在未使用过 TinyWarp 的正式稳定系统 Apple Silicon 机器或可恢复快照的 VM 上，从待分发的真实地址验证以下路径。开发机运行、命令行下载和隔离 HOME 测试不能代替此项。

| 场景 | 操作与要记录的结果 |
| --- | --- |
| 首次下载 | 用浏览器下载最终 ZIP，通过 Finder／Archive Utility 解压，保留下载隔离属性；不移除 quarantine，不关闭 Gatekeeper |
| 首次与第二次启动 | 分别检查解压后直接启动、移入 Applications 后启动；记录系统提示、启动结果及首次路径随机化情况下的资源加载 |
| 基础终端 | 无账号启动、PTY 输入输出、外部 CLI、中文、历史／补全、设置默认外观与保存、关于页、图标切换及退出后重开 |
| 联网与离线 | 检查附票后的启动体验；离线启动不应依赖本次临时联网取得 ticket，失败时保留系统诊断 |
| 旧版升级 | 在专用测试数据中准备旧版历史、配置、标签／分屏与工作目录；走真实公开检查更新、正常退出和重启安装，独立回读数据与设置，不还原旧数据库冒充保留 |
| 已安装副本 | 检查同一版本位于不同位置、旧版和新版共存时的启动与身份选择，避免把另一个副本当成验收对象 |

日常 TinyWarp 的升级仍由用户通过内置检查更新完成。验收期间不替换或退出用户正在使用的应用，不把独立测试数据库与日常数据互换。

[Apple DTS 的可信执行说明](https://developer.apple.com/forums/thread/706442)指出，浏览器下载与系统解压会传播隔离标记，开发机和 `curl` 路径可能看不到相同的问题；系统策略命令通过也不覆盖全部启动故障。[Apple 分发测试建议](https://developer.apple.com/documentation/xcode/packaging-mac-software-for-distribution)列出了首次、升级、重复安装及路径随机化场景。

每轮记录具体 macOS 版本／构建、芯片、应用版本／构建／源码提交、资产 SHA256、签名团队、submission ID、附票结果、实际安装路径与数据回读结果；凭据、私人历史和数据库不进入公开记录。某一系统通过只支持该系统的结论，拟定最低版与其他版本仍需各自验证。

## 尚未完成

截至 2026-11-16，以上 Developer ID、公证、附票、干净稳定系统首次安装与旧系统兼容性均未完成；当前已发布 0.1.2 仍是 ad hoc 包。最低支持版本尚待实际验证，不以编译参数、签名自洽或更新成功替代产品支持承诺。
