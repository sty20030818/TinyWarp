# TinyWarp L3C3：Mermaid 专用渲染依赖裁剪

2026-11-10 开始，2026-11-11 完成验证，基于 C2 `6cb85156c`。本批继续专用依赖裁剪，独立开发候选已完成编译和隔离原生检查。

## 范围与兼容

Mermaid SVG 渲染仅由 AI 图表和 Markdown 编辑器使用。TinyWarp 已关闭这些外围入口，本批将 `mermaid_to_svg` 改为可选依赖，由原版完整应用的 `markdown_mermaid` 启用；终端共用编辑器继续保留。

- `warp_editor` 独立默认仍提供图表渲染；workspace 和 `languages` 的编辑器引用关闭默认渲染，由应用 feature 显式选择。原版 GUI／TUI 继续通过完整应用默认启用。
- 语言标记识别移至共用编辑模块，与固定版本渲染器的原判定完全一致，不再为了识别 `mermaid` 标记引入整套渲染器。
- 保留 `CodeBlockType::Mermaid`、Agent 图表数据、Markdown 源码及序列化结构。旧类型在精简构建请求渲染时明确失败，沿用已有源码代码块回退，不生成假图；正常 TinyWarp 的渲染 flag 关闭。
- 复制／剪切继续保留 fenced Markdown 和代码 HTML，仅关闭精简构建中的 SVG 生成／追加。完整构建继续生成原 SVG HTML。无数据库迁移或历史内容重写。
- `script/check_tinywarp_features` 增加 Mermaid 及图布局专用库边界。

macOS arm64 产品 normal／build 图从 921 项降至 915 项：移除 `mermaid_to_svg`、`dagre_rust`、`graphlib_rust`、`ordered_hashmap`、`serde_yaml 0.9`、`unsafe-libyaml`，无新增。共用 `petgraph` 与其他版本 YAML 支持保留。完整默认 GUI 构建的 975 项依赖集合与本批开始前一致。

这是编译依赖裁剪，不代表整个 AI／Agent／云对象模块已经退出二进制，也不据此推算内存或速度收益。

## 验证记录

全部编译、定向单元回归、严格 Clippy 和组包由 Warp Preview 独立构建窗口执行，父进程链留证。复用 Rust 1.92 缓存与既有 opt0 应用覆盖，不做正式优化构建或性能对比。

- 精简／完整编辑器：19／20 项定向回归通过，覆盖标记边界、Markdown 往返、选择／布局、失败源码回退和完整渲染；精简／完整应用：73／74 项通过，包含复制／剪切、旧数据往返、本地设置、标签／历史及外部 CLI 路径。
- 编辑器和应用分别对精简／完整组合运行 `cargo clippy --lib --tests ... -- -D warnings`，四组均通过。精简应用测试首次成功编译仍报告一项仅测试导入警告，随后仅为该导入补齐 cfg；当前源码由严格检查重新编译验证，未改变测试行为。
- 真实产品依赖边界检查通过；本阶段实际应用编译指纹不含 `default`、`voice_input`、`aws_llm_credentials`、`markdown_mermaid`，含 `app_defaults`／`tinywarp_gui`。
- 开发候选编译 2 分 13 秒，组包成功。签名、身份、图标、无 App Group、140/140 资源、19 个 schema 分区及 23 项系统动态依赖检查通过；设置 schema 与 C2 的 JSON 语义一致。
- 新候选的隔离副本：真实 zsh 就绪、外部 `/bin/cat` 输入输出字节检查、新标签／分屏 shell 数 1 → 2 → 3 均通过。设置截图人工查看确认绘制正常；测试实例均正常退出。这里是原生候选冒烟检查，本批未重新构建独立 integration runner。
- 候选编译前冻结的 5,795 项源码输入在验证结束时哈希一致；最后仅定向格式化改动 Rust 文件，按仓库规则不因纯格式化重跑测试或构建。

初轮将公共测试函数提取后出现测试生命周期编译错误，已修正。应用初轮发现一项要求 SVG 加载成功的测试，在无渲染器构建失败；该测试现明确限定到完整渲染能力，原成功断言保留。精简版新增有效源码在无渲染器时返回明确错误／回退代码块的检查，复制和剪切测试仍检查源码及代码 HTML，另断言不会附加 SVG。初轮失败日志独立留存，未混入最终通过结果。

## 数据保护与交付

测试使用临时 HOME、专用应用身份和文件访问限制，拒绝真实 TinyWarp／Preview 的配置、Application Support、原生偏好、Group Containers 和 `~/.aws`。测试子进程移除 AWS 环境变量，未读取真实凭据或调用真实登录接口。最终复核：原 8 个配置文件哈希一致，原数据库 inode 保留，原 Preview 进程存活，测试残留进程为 0，Cargo.lock 未变。真实数据库仅检查元信息，没有迁移、重写或删除。

候选位于 `.note/candidates/L3C3-dev/release/bundle/osx/TinyWarp.app`，日常 TinyWarp 未自动替换或切换。阶段结束本地提交，不推送。原始证据在 Git 忽略目录 `.note/l3c3/`；不提交构建产物、测试数据库或用户配置。

未做长期内存基准、真实中文 IME／SSH 或已登录外部 Agent CLI 验收；不把依赖减少换算成运行内存或速度收益。
