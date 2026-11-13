# TinyWarp 关于页字标

2026-11-13，用户确认采用彩色终端字符格／块字与立体厚度的 TinyWarp 字标，并从改色候选中选定下方更亮、背景干净的一版。应用接入范围为关于页。

![TinyWarp 字标](../../app/assets/bundled/png/tinywarp-logo.png)

## 资源与设计来源

- 稳定资源：`app/assets/bundled/png/tinywarp-logo.png`。
- 尺寸：2172 × 724；PNG 含 alpha，已读取像素确认四角透明。
- 文件大小：660,562 字节；SHA256：`373a650206fc6cf7da55cc27c7a7c584129d1c6c0d247db9c5cae34ff1319306`。
- 配色方向：青／蓝过渡到靛紫，再以珊瑚橙收尾；保留原字符格字形、黑色轮廓和立体挤出。
- 使用内置 imagegen 生成与改色，经用户选定；设计参考仅用于方向讨论，未直接作为应用资源。

后续调整以此已选定 PNG 为参考，先给出视觉稿再替换；不要从生成目录的随机文件名自动挑选“最后一张”。生成式编辑不保证逐像素复现，当前文件本身是可回读的资源事实源。

## 应用接入

`AboutPageWidget` 沿用现有 `Image`、`AssetSource::Bundled` 和 `CacheOption::BySize`。本地终端模式使用 TinyWarp PNG，其他模式继续使用原主题对应的 Warp SVG。用户在首轮候选中反馈字标偏小后，本地模式的最大尺寸调整为 500 × 150，保留原图比例与透明留白；其他模式继续采用 350 × 100。居中排列、版本行和原有来源署名沿用。

关于页增加“GitHub 仓库”链接，复用已有 Link 组件和持久 MouseStateHandle。链接与更新器共用 `local_terminal::REPOSITORY_URL`，更新资产地址仍由固定仓库、已验证版本和架构派生。

没有改变设置页的 Monolith 结构、搜索规则、版本复制动作、应用图标、数据身份或存储格式。图片由现有 `warp_assets` 的 bundled 资源规则编入应用，无新增依赖。

## 设置默认入口

用户同时确认普通设置入口应进入“外观”。`SettingsSection::LOCAL_TERMINAL_DEFAULT` 统一普通入口、初始化和旧不可用页面的回退；指定打开功能、快捷键、关于等可用页面仍有效。现有设置标签继续复用，普通入口会切回外观。侧栏原有顺序已是外观在功能之前，未重新排列其他项目。

## 验证与交付状态

2026-11-13，本轮源码已接入并完成以下检查：

- 69 项定向 Rust 回归全部通过，覆盖默认外观、不可用旧页面回退、设置标签复用、本地模式及更新源验证。
- `warp` 的 lib／tests／`warp-oss` 和 `integration` 的定向 Clippy 均以 `-D warnings` 通过，随后完成所有修改 Rust 文件的最终格式化。
- 真实显示 GUI 回归 `test_local_terminal_settings_open_once_and_return_to_terminal` 通过：菜单打开外观、显式进入功能／关于、普通入口复用原设置标签并回到外观、返回终端执行命令。
- 深／浅主题均实际渲染且 PNG 解码成功；已人工检查截图中字标、版本行、来源署名及 GitHub 入口无裁切。外观页截图确认导航与页面内容一致。
- GUI 使用临时 HOME 与空账号 fixture，并以 sandbox 拒绝访问日常 TinyWarp／Preview 数据；未替换、重启或迁移日常应用。

GUI 证据位于本地忽略目录 `.note/settings-polish/gui-artifacts/test_local_terminal_settings_open_once_and_return_to_terminal/2026-11-13T13-04-23/`，包含 `about-dark.png`、`about-light.png`、`settings-appearance.png` 与步骤记录；单元与严格检查日志位于 `.note/settings-polish/`。

首轮 GUI 失败来自误将“曾经渲染过”的控件位置缓存当成“当前可见”断言；已移除这条无效的消失判断，保留首次渲染断言，并以实际当前页面、同一设置标签及截图验证返回外观。

早期独立候选编译曾复用未包含新 PNG 的 `warp_assets` 缓存；仅清理该包的生成缓存后重建，已确认批准 PNG 的完整字节嵌入候选。该候选的较小字标已被本轮放大实现取代，不能作为最终尺寸证据。

本轮未发布新版本或替换日常应用。GitHub 链接使用既有组件，已核对目标地址与页面渲染，尚未实测点击后外部浏览器跳转；全工作区、其他平台、VoiceOver 与完整缩放矩阵未验收。


## 0.1.2 已发布

2026-11-13，用户授权后已随 0.1.2（构建 5）公开发布。正式包与新旧公开更新地址的回读通过，日常升级由用户自行通过检查更新完成；证据和未验收范围见 [发布记录](0.1.2-about-settings.md)。前文“本轮未发布”描述的是 UI 实现阶段。
