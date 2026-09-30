# 桌面字幕

`substream-overlay` 是 KDE Plasma Wayland 显示端。字幕位于所选屏幕底部，可以覆盖全屏窗口，鼠标和键盘操作仍交给原窗口。没有新字幕时默认五秒后隐藏。

## 构建

需要 C++20 编译器、CMake 3.22+、Ninja、Qt 6.5+ 的 Quick 和 WebSockets 模块，以及 LayerShellQt 6。运行时需要 Qt Wayland 支持。

Arch Linux 可安装以下软件包：

```bash
run0 pacman -S --needed base-devel cmake ninja qt6-declarative qt6-websockets qt6-wayland layer-shell-qt
```

在仓库根目录构建：

```bash
cmake -S apps/overlay -B build/overlay -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build/overlay --parallel 2
build/overlay/substream-overlay --preview
```

预览显示固定示例字幕，五秒后退出，不启动识别。显示程序与 Rust 服务分别构建，安装显示依赖不影响命令行的默认构建。

安装到当前用户目录可同时注册桌面应用信息：

```bash
cmake --install build/overlay --prefix "$HOME/.local"
```

## 浏览器字幕

按照 [浏览器扩展说明](../README.md#浏览器扩展) 启动本地识别服务，再运行：

```bash
build/overlay/substream-overlay --token-file /tmp/substream.token
```

开始捕获标签页音频后，弹窗和桌面会同时显示字幕。悬浮层可以随时打开或关闭，不会停止浏览器识别。服务断开时字幕立即清空，显示程序会自动重连；令牌或协议错误会输出原因并退出。

服务使用其他端口时，指定完整订阅地址：

```bash
build/overlay/substream-overlay --token-file /tmp/substream.token \
  --server ws://127.0.0.1:9744/v1/display
```

## 系统音频

显示程序可以直接读取 `stream` 的字幕事件，无需启动本地服务。配置好模型后，将识别输出接到 `--stdin`：

```bash
pw-cat --record --raw --rate 16000 --channels 1 --format s16 --latency 20ms \
  --properties '{ stream.capture.sink=true }' --target '<sink-node-name>' - |
  target/release/substream stream --backend sherpa --config configs/sherpa.example.toml |
  build/overlay/substream-overlay --stdin
```

模型构建与设备选择见 [实时识别](../README.md#实时识别)。输入结束后，末尾字幕保留到显示时间结束，随后窗口退出。按 Ctrl+C 可停止前台命令。

## 显示设置

```bash
build/overlay/substream-overlay --list-screens
build/overlay/substream-overlay --preview --screen eDP-1 \
  --font-size 32 --width 1000 --bottom-margin 80 --hold-ms 8000
```

`--screen` 使用屏幕名称，默认选择主屏幕。宽度、字号和底部距离采用逻辑像素，随桌面缩放调整。显示器断开时窗口退出，可指定其他屏幕重新启动。

字幕按纯文本显示，自动换行；过长的段落显示末尾四行。`--hold-ms` 从最近一次字幕更新计时。正常结束会保留末尾字幕，取消会话会清空画面。

## 显示端接口

KDE 窗口设置集中在 `apps/overlay/src/kde_window.cpp`，消息接收和字幕状态分别位于 `event_source.cpp`、`caption_model.cpp`。

跨桌面接入使用 [只读字幕协议](protocol.md#桌面字幕订阅)。GNOME Shell 扩展可使用自身的网络和界面组件订阅 `/v1/display`，按 `session_id` 切换会话、替换字幕，并按 `caption_age_ms` 控制显示时长。该接口不要求继承 C++ 类，也不依赖 Qt；当前提供的窗口实现仅支持 KDE Wayland。

## 开发检查

```bash
clang-format --dry-run --Werror apps/overlay/src/*.cpp apps/overlay/src/*.h apps/overlay/tests/*.cpp
/usr/lib/qt6/bin/qmllint apps/overlay/qml/Overlay.qml
ctest --test-dir build/overlay --output-on-failure
```

自动测试检查字幕修正、过期隐藏、会话切换和订阅行为。窗口行为需要在目标桌面检查：全屏覆盖、鼠标穿透、原窗口焦点、屏幕选择和缩放。
