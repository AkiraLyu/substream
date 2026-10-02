# Substream

Substream 是面向 Linux 的本地语音识别与字幕工具，支持系统音频和浏览器标签页的实时字幕，以及本地媒体文件的完整字幕导出。

实时识别使用 sherpa-onnx，文件识别使用 whisper.cpp。提供命令行、本地 WebSocket 服务、Chromium 扩展、桌面设置窗口和 KDE Wayland 悬浮字幕。GNOME 显示端、网页视频下载、翻译和总结尚未实现。

技术选择和模块职责见 [架构设计](docs/architecture.md)，客户端接入方式见 [本地协议](docs/protocol.md)。

## 桌面程序

需要 Linux、Rust 1.88+、C++20 编译器、CMake、Qt 6 的 Widgets、Quick、WebSockets 模块和 LayerShellQt。系统音频采集需要 PipeWire 工具。

```bash
cargo build -p substream --release --features sherpa
cmake -S apps/overlay -B build/overlay -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build/overlay --parallel 2
build/overlay/substream-overlay
```

在“音频源”中选择输出设备或输入设备，在“模型”中选择准备好的模型配置文件，再点击“开始字幕”。窗口显示当前捕获源、实际加载的模型、支持的语言、线程数、音频时长和最新字幕。悬浮字幕可设置屏幕、字号、宽度、底部距离和隐藏时间。

模型需单独准备，程序不附带模型。配置方法见下文，桌面操作、浏览器服务和安装方法见 [桌面字幕](docs/desktop.md)。

## 实时识别

从 [sherpa-onnx 模型目录](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/index.html) 选择流式 Transducer 模型，将模型文件路径填入 [配置文件](configs/sherpa.example.toml)。相对路径以配置文件所在目录为准，`languages` 应填写该模型支持的语言。

```bash
cargo build -p substream --release --features sherpa
ffmpeg -nostdin -i input.mp4 -vn -ac 1 -ar 16000 -f s16le - |
  target/release/substream stream --config configs/sherpa.example.toml
```

`stream` 读取标准输入中的音频，逐行输出 JSON 字幕事件。按正常播放速度测试文件时，在 FFmpeg 的 `-i` 前添加 `-re`。

当前适配器使用 CPU。启用 `sherpa` 后，首次编译会下载 sherpa-onnx 1.13.8 对应的原生库；已有库可通过 `SHERPA_ONNX_LIB_DIR` 指定。语音模型需单独准备。详见 [原生库安装说明](https://k2-fsa.github.io/sherpa/onnx/rust-api/advanced-install.html)。

系统音频通过 PipeWire 的 `pw-cat` 采集。先用 `wpctl status -n` 查找输出设备，将下面的 `<sink-node-name>` 替换为设备节点名称：

```bash
pw-cat --record --raw --rate 16000 --channels 1 --format s16 --latency 20ms \
  --properties '{ stream.capture.sink=true }' --target '<sink-node-name>' - |
  target/release/substream stream --config configs/sherpa.example.toml
```

音频经进程管道传入，缓冲和传输时间会影响字幕延迟。设备访问方式取决于系统的 PipeWire 配置，详见 [音频节点属性说明](https://docs.pipewire.org/group__pw__keys.html)。

## 浏览器扩展

需要 Chromium 116+；构建和测试扩展使用 Node.js 24+。浏览器与本地服务应运行在同一台机器上。

```bash
cd browser/extension
npm ci --ignore-scripts
npm run build
```

在浏览器扩展管理页启用开发者模式，加载 `browser/extension/dist`，复制扩展 ID。回到仓库根目录，创建令牌并启动服务：

```bash
cargo run -p substream -- token --output /tmp/substream.token
cargo run -p substream --release --features sherpa -- serve \
  --config configs/sherpa.example.toml \
  --token-file /tmp/substream.token \
  --allow-origin 'chrome-extension://<extension-id>'
```

打开正在播放音频的标签页，点击扩展，将令牌文件内容填入弹窗后开始捕获。字幕显示在弹窗中；关闭弹窗后仍会继续捕获，重新打开即可停止。原音频会继续播放。桌面程序也可以启动服务并显示标签页字幕，选择“浏览器标签页”并填写扩展 ID 即可。

令牌文件仅允许当前用户读写，扩展在浏览器会话期间保存令牌。模型加载完成后才开始采集。扩展只能捕获启动后实际播放的音频；生成整段字幕需要使用完整媒体文件。

## 文件字幕

安装 FFmpeg 和 [whisper.cpp 的 whisper-cli](https://github.com/ggml-org/whisper.cpp/tree/master/examples/cli)，并准备语音模型：

```bash
cargo run -p substream --release -- transcribe input.mp4 \
  --model /path/to/ggml-model.bin \
  --whisper-bin /path/to/whisper-cli \
  --language auto --format srt --output captions.srt
```

`--format` 支持 `srt`、`vtt` 和 `json`。省略 `--output` 时写入标准输出，指定文件时不会覆盖已有文件。输入目前只支持本地文件。

字幕保留识别结果的段落时间，长文本按显示宽度换行，尚不支持按词调整时间或限制为两行。临时音频会在任务结束后删除；按 Ctrl+C 可取消任务，`--timeout-secs` 分别限制音频转换和识别的等待时间。

## 开发

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked

cd browser/extension
npm run format:check
npm run check
npm run build
npm test
```

默认测试不需要语音模型。安装 FFmpeg 和 Python 3 后，可在仓库根目录额外检查文件转换与字幕导出：

```bash
cargo test -p substream --test offline -- --ignored
```

该测试使用真实 FFmpeg 和固定识别结果，验证音频转换、字幕内容及时间，不评估模型准确率。CI 单独构建 `sherpa` 功能以检查原生库链接。

测试应围绕音频完整性、字幕内容与时间、协议兼容和失败处理。优先通过公开接口检查结果，避免固定内部调用顺序、队列容量或字幕更新次数。

修改浏览器捕获流程后，还需在实际浏览器中检查：开始和停止、弹窗关闭后重开、停止时保留末尾字幕、原音频继续播放，以及启动失败后可以重试。模型效果和延迟的测量方法见 [性能测试](docs/performance.md)。

## 代码布局

| 目录 | 职责 |
| --- | --- |
| `crates/core` | 音频与字幕数据、识别接口、字幕更新和导出 |
| `crates/protocol` | JSON 消息和二进制音频格式 |
| `crates/backends` | 识别引擎、FFmpeg 和子进程管理 |
| `apps/substream` | 命令行、配置、认证和本地服务 |
| `apps/overlay` | 参数界面、设备发现、进程控制和 KDE 悬浮字幕 |
| `browser/extension` | 标签页音频捕获与字幕预览 |
| `fixtures` | 协议和识别结果的共用样本 |

接入识别引擎时，实现 `StreamingRecognizer` 或 `BatchRecognizer`，再在应用的配置入口注册。核心模块不依赖具体引擎或网络库。
