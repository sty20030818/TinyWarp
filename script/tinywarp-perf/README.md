# TinyWarp L3A 测量工具

适用于本仓库 macOS GUI 候选。只操作隔离测试包、临时 HOME 和 `.note/l3a`，不把日常配置或历史作测试数据。L3A 最终采用后台内存协议，前台输入／滚动另测。

## 后台协议

- `metrics.c` 用 `proc_pid_rusage` 读取 RSS、physical footprint、CPU 时间。退出进程的 ESRCH 可跳过，其他错误及全体 PID 不存在时失败。
- `control.swift` 只接受 `dev.warp.TinyWarpL3A*` 身份和完全匹配的规范化包路径，拒绝 Preview／日常 TinyWarp。后台协议仅用 hide／info／正常 quit，不发送全局键盘事件、不改剪贴板、不移动窗口。
- `control_main.rs` 复用同一优化应用库，测试入口额外开放 `WarpControlCli`；权限仍需在测试 UI 中主动启用。无需安装 `/usr/local/bin` 命令；客户端调用包内可执行文件的 `--warpctrl`。
- `launch_control.py` 复制候选资源、替换测试入口、设置唯一身份和签名，启动独立 HOME。内核文件规则拒绝实际 TinyWarp／Preview 的配置、Application Support、原生偏好文件及 Group Containers。结果和临时数据保留，不自动删除。
- `background.py` 始终指定测试 PID、窗口和标签 ID，通过原有本地控制接口操作。每次采样都检查测试包隐藏且不在前台。按测试应用后代定位 zsh，专用信号 trap 产生 10 万行 PTY 输出；不执行用户项目。
- `fixtures.py` 固定 Hack 14、120 列／40 行、关闭竖向标签、50000 行滚动容量，关闭会话恢复／登录启动／退出确认／遥测。保留默认 60 秒撤销关闭窗口。

同一进程连续三轮：单标签空闲 → 6 标签／8 pane → 关闭并等 shell 回收 → 20 次开关标签后的即时与回收状态 → 新标签内固定输出 → 关闭并等回收。每个稳定窗口目标约 15 秒，每次读取后休眠 1 秒，实际间隔包含读取与系统调度开销；独立子 shell 退出是回收前置条件。三轮不是三个冷启动。

## 构建与运行

先按 [macOS 构建说明](../../docs/fork/build-macos.md) 在 Warp Preview 构建优化候选，保留原 release profile，不加开发 opt0 覆盖。下面目录必须是新建的私有目录；避免覆盖旧记录。

```sh
mkdir -m 700 -p .note/l3a
xcrun clang -O2 -Wall -Wextra -Werror script/tinywarp-perf/metrics.c -o .note/l3a/metrics
xcrun swiftc -O -warnings-as-errors script/tinywarp-perf/control.swift -o .note/l3a/control
python3 -B script/tinywarp-perf/baseline.py --self-test
```

控制测试入口可复用该次构建的 release rlib，仅链接新入口。**warp 与 warp_core 的哈希必须来自同一次构建且依赖匹配**，不能随意取目录里第一个文件。以下为 L3A `944a4fd23` 的已验证示例；上游／工具链／feature 改变后应重新核对构建指纹。

```sh
rustc --edition=2024 -C opt-level=3 -C debuginfo=0 \
  -C debug-assertions=no -C overflow-checks=no -C link-arg=-lc++ \
  -L "native=$(xcrun clang --print-runtime-dir)" -L dependency=target/release/deps \
  --extern warp=target/release/deps/libwarp-a3a1a0cad09401ee.rlib \
  --extern warp_core=target/release/deps/libwarp_core-8718eb31a58f4c24.rlib \
  script/tinywarp-perf/control_main.rs -o .note/l3a/warp-perf-control

python3 -B script/tinywarp-perf/launch_control.py \
  --app .note/candidates/L3A/release/bundle/osx/TinyWarp.app \
  --binary .note/l3a/warp-perf-control --work .note/l3a/new-background-run
```

首次在测试包设置搜索结果里启用 `warpctrl CLI`，**无需点安装命令**。准备阶段最多等待 10 分钟；授权后自动隐藏测试包并测三轮，最后正常退出。测量阶段电脑可继续使用，但不要操作测试包；其他进程与系统内存压力仍会影响结果。此前已经启用的隔离实例也可用 `background.py --state <state.json> --output <新目录>` 接续。

## 指标与边界

application 是主进程和同包辅助进程之和；children 单列 shell／其他后代。RSS 可能重复计算共享页，physical footprint 是主要指标。峰值只是采样窗口内观察峰值；CPU 根据连续采样仍存活 PID 的时间差估计，可能漏掉短命子进程。控制服务和已访问过的设置页缓存包含在应用内存中，未测出可单独扣除的控制服务成本。

标记检测中的 0.15 秒只是循环休眠，不是有效计时精度；实际耗时包含进程检查及调度等待。输出完成标记代表 shell 已写完测试输出，不是最后像素已呈现；signal 响应也不等同键盘延迟。`vmmap` 在第三轮定量采样结束后采集。失败时保留记录，先诊断，不覆盖重试；不得混入未完成／前台混合轮次。

`baseline.py --cycle 1/2/3` 保留为前台实验工具，需要单一显示条件、连续前台及 Accessibility 权限；它的历史结果未纳入后台基线。当前不据此宣称冷启动性能、长期无泄漏或相对原版节省了多少内存。
