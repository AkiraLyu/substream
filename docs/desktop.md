# 桌面字幕

`substream-overlay` 提供参数窗口、音频采集控制和 KDE Plasma Wayland 悬浮字幕。它使用本地模型识别语音，窗口中显示当前音频源、模型、输入电平、音频时长和最新字幕。

## 构建与安装

需要 C++20 编译器、CMake 3.22+、Ninja、Qt 6.5+ 的 Widgets、Quick、WebSockets 模块，以及 LayerShellQt 6。KDE 悬浮字幕还需要 Qt Wayland；系统音频采集使用 PipeWire 的 `pw-dump` 和 `pw-cat`。

Arch Linux 可安装以下软件包：

```bash
run0 pacman -S --needed base-devel cmake ninja qt6-base qt6-declarative qt6-websockets qt6-wayland layer-shell-qt pipewire-audio
```

在仓库根目录构建并运行：

```bash
cargo build -p substream --release --features sherpa
cmake -S apps/overlay -B build/overlay -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build/overlay --parallel 2
build/overlay/substream-overlay
```

安装到当前用户目录：

```bash
cmake --install build/overlay --prefix "$HOME/.local"
install -Dm755 target/release/substream "$HOME/.local/bin/substream"
```

安装后可从应用菜单打开 Substream。GUI 与识别程序分别构建，命令行默认构建不需要 Qt。模型需单独准备，详见 [实时识别](../README.md#实时识别)。

## 系统音频

1. 在“音频源”中选择“系统音频”，选择要捕获的设备。“系统输出”捕获该设备正在播放的声音，“音频输入”捕获麦克风或其他输入设备。
2. 在“模型”中选择 TOML 配置文件和 `substream` 程序。CPU 线程数默认使用模型配置，也可以单独指定。
3. 在“字幕显示”中设置悬浮层和隐藏时间，点击“开始字幕”。

程序先加载模型，再启动采集。右侧状态来自实际进程和字幕事件，模型名称、语言和线程数在加载成功后显示，悬停在模型名称上可查看完整路径。输入电平反映声音强弱，静音时为零；音频时长包含静音。

点击“停止”会先结束采集，再等待识别器输出末尾字幕。关闭窗口也会停止其启动的进程。加载失败、设备断开或音频积压时显示原因，修正参数后可以重新开始。运行期间参数不可修改，停止后设置会在下次启动时生效。

设备列表可手动刷新；运行时定期检查设备是否仍存在。已选设备断开时不会自动切换到其他麦克风或输出设备。当前不支持从 PipeWire 列表中单独选择某个应用的播放流；浏览器标签页可用下述方式单独捕获。

## 浏览器字幕

先构建并加载 [Chromium 扩展](../README.md#浏览器扩展)。在桌面程序中选择“浏览器标签页”，填写扩展管理页中的 ID，选择模型配置后开始。服务固定监听 `127.0.0.1:9743`；如果已有服务占用该端口，需要先停止它。

首次启动会创建私有令牌文件，已有文件不会被覆盖。点击“复制配对令牌”，粘贴到扩展弹窗中，再开始捕获。界面显示当前标签页名称、模型加载状态和音频时长。模型在浏览器建立识别会话时加载，等待浏览器连接时显示“未加载”。标签页名称取自开始捕获时的标题。

在扩展中停止捕获会保留末尾字幕，本地服务继续等待下一次会话。桌面程序的“停止服务”会取消当前浏览器识别并结束服务。浏览器模式不显示输入电平，因为显示订阅只传输字幕和会话状态。

设置由 Qt 保存在用户配置目录，通常为 `~/.config/Substream/substream-overlay.conf`；令牌位于单独的私有文件中，不写入设置或日志。

## 悬浮字幕

字幕位于所选屏幕底部，支持鼠标穿透、自动换行和超时隐藏，不抢占键盘焦点。尺寸采用逻辑像素，随桌面缩放调整。长段落显示末尾四行，没有新字幕时按设置的时间隐藏。显示器断开后该悬浮窗口关闭，主窗口仍可查看字幕。

参数窗口可在其他桌面打开，但悬浮层目前只支持 KDE Plasma Wayland。GNOME 悬浮显示端尚未实现。

也可以单独运行悬浮层，连接已有服务：

```bash
build/overlay/substream-overlay --token-file /path/to/substream.token \
  --server ws://127.0.0.1:9743/v1/display
```

或接收命令行识别结果：

```bash
pw-cat --record --raw --rate 16000 --channels 1 --format s16 --latency 20ms \
  --properties '{ stream.capture.sink=true }' --target '<sink-node-name>' - |
  target/release/substream stream --config configs/sherpa.example.toml |
  build/overlay/substream-overlay --stdin
```

独立悬浮层支持 `--screen`、`--font-size`、`--width`、`--bottom-margin` 和 `--hold-ms`。使用 `--list-screens` 查看屏幕名称。标准输入结束后，末尾字幕保留至显示时间结束，窗口随后退出。

## 扩展与检查

`MainWindow` 负责参数界面，`SessionController` 管理进程，`AudioDevices` 读取设备列表。`CaptionModel` 处理字幕状态，`EventSource` 接收 JSON 消息，`KdeWindow` 负责 KDE 窗口设置。

GNOME Shell 扩展可使用自身的网络和界面组件接入 [只读字幕协议](protocol.md#桌面字幕订阅)，按会话编号替换字幕，并按字幕年龄控制显示时长。协议不要求继承 C++ 类，也不依赖 Qt。

```bash
clang-format --dry-run --Werror apps/overlay/src/*.cpp apps/overlay/src/*.h apps/overlay/tests/*.cpp
/usr/lib/qt6/bin/qmllint apps/overlay/qml/Overlay.qml
ctest --test-dir build/overlay --output-on-failure
```

自动测试检查字幕修正、过期隐藏、会话切换、停止后的末尾字幕及失败后重试。设备选择、实际识别、全屏覆盖、鼠标穿透和屏幕缩放需在目标桌面检查。
