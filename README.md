# Substream

Substream 是面向 Linux 的本地语音识别与字幕工具，支持系统音频和浏览器标签页的实时字幕，以及本地文件、网页视频的完整字幕导出。

实时识别使用 sherpa-onnx，文件识别使用 whisper.cpp，网页视频通过 yt-dlp 下载。提供命令行、本地服务、Chromium 扩展、桌面设置窗口和 KDE Wayland 悬浮字幕。视频字幕可通过兼容 OpenAI Chat Completions 的接口生成 AI 总结。GNOME 显示端和翻译尚未实现。

技术选择和模块职责见 [架构设计](docs/architecture.md)，客户端接入方式见 [本地协议](docs/protocol.md)。

## 桌面程序

需要 Linux、Rust 1.88+、C++20 编译器、CMake、Qt 6.7+ 的 Widgets、Quick、WebSockets、LinguistTools 模块和 LayerShellQt。系统音频采集需要 PipeWire 工具。

```bash
cargo build -p substream --release --features sherpa
cmake -S apps/overlay -B build/overlay -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build/overlay --parallel 2
build/overlay/substream-overlay
```

在“音频源”中选择输出设备或输入设备，在“模型”中选择准备好的模型配置文件，再点击“开始字幕”。窗口显示当前捕获源、实际加载的模型、支持的语言、线程数、音频时长和最新字幕。悬浮字幕可设置屏幕、字号、宽度、底部距离和隐藏时间。

模型需单独准备，程序不附带模型。配置方法见下文，桌面操作、浏览器服务和安装方法见 [桌面字幕](docs/desktop.md)。

桌面程序和浏览器扩展支持简体中文与英语，分别跟随系统和浏览器的界面语言，未支持的语言回退为英语。界面语言不影响语音识别和字幕内容。语言设置与翻译维护方法见 [界面语言](docs/i18n.md)。

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

令牌文件仅允许当前用户读写，扩展在浏览器会话期间保存令牌。模型加载完成后才开始采集。扩展弹窗只能捕获启动后实际播放的音频；生成整段字幕可使用下述视频任务接口。

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

## 网页视频字幕

安装 [yt-dlp](https://github.com/yt-dlp/yt-dlp) 和 FFmpeg。任务先检查已有字幕，优先使用人工字幕，其次使用平台自动字幕；没有可用字幕时才通过 whisper-cli 识别完整音轨，此时需要 whisper.cpp 的 GGML 模型。视频任务不需要启用 `sherpa`。

复制 [视频配置](configs/video.example.toml)，填写输出目录、cookies 来源和备用识别模型。在桌面程序的“视频字幕”页面选择配置、填入链接，即可开始；也可使用命令行：

```bash
cargo run -p substream --release -- video 'https://example.com/video' \
  --config configs/video.example.toml
```

`cookies_from_browser` 指定读取登录状态的浏览器，例如 `firefox`、`chromium` 或 `chrome+kwallet6`。指定用户配置目录时，使用 `firefox:/path/to/profile`。也可用 `cookies_file` 指定 Netscape 格式的 cookies 文件，两项不能同时设置。省略两项即可匿名下载。Cookie 由本机 yt-dlp 读取，不经过扩展或字幕接口；浏览器格式和密钥环支持见 [yt-dlp 参数说明](https://github.com/yt-dlp/yt-dlp#filesystem-options)。

桌面页面和命令行均可覆盖本次任务的 cookies 来源。命令行使用 `--cookies-from-browser`、`--cookies-file` 或 `--no-cookies`，三者只能选一个。cookies 文件会复制到私有任务目录，原文件不会被修改，任务结束后删除副本。

`language` 指定字幕和识别语言。设为 `auto` 时，在同类字幕中优先匹配视频原始语言，其次选择英语，再选择其他可用语言。指定语言时只使用匹配的字幕，包括同一语言的地区变体。字幕下载或解析失败会报告错误，不会静默改为语音识别。当前支持 yt-dlp 提供的 SRT、WebVTT、TTML、ASS 和 SSA 字幕；不检测画面内嵌文字。

每个任务创建独立目录，保存下载的视频、`subtitles.srt`、`subtitles.vtt` 和 `document.json`。JSON 文档包含来源链接、标题、语言、字幕来源和段落时间，可直接用于 AI 总结。已有字幕出现重叠时，按时间边界合并同时显示的文字。命令按行输出 JSON 任务状态，完成状态包含结果路径。重复下载不会覆盖已有结果。

按 Ctrl+C 可取消任务。`timeout_secs` 分别限制字幕检查、下载、音频转换和识别时间；取消、超时或失败会结束子进程并删除任务目录，成功后删除中间文件。每次请求处理一个视频，纯播放列表链接只取第一项，不支持直播。

在本地服务中启用视频任务：

```bash
cargo run -p substream --release -- serve \
  --video-config configs/video.example.toml \
  --token-file /tmp/substream.token \
  --allow-origin 'chrome-extension://<extension-id>'
```

令牌需先用 `token` 命令创建。同时使用实时字幕时，再加上 `--config` 并以 `--features sherpa` 构建。桌面程序启动浏览器服务时，使用“视频字幕”页面选中的视频配置。

服务提供提交、查询、取消和读取字幕文档的 [HTTP 接口](docs/protocol.md#视频任务)。扩展的 `video.ts` 和后台消息接口可直接调用这些功能；当前扩展弹窗仅提供实时字幕操作。

## AI 总结

视频任务完成后，点击“AI 总结”，填写接口地址、API 密钥和模型，再点击“生成总结”。也可直接在“AI 总结”页面选择已有的 `document.json`。系统提示词和用户提示词均可编辑，结果可预览并保存为 Markdown。

命令行使用独立的 [总结配置](configs/summary.example.json)：

```bash
cargo run -p substream --release -- summarize /path/to/document.json \
  --config /path/to/summary.json --output summary.md
```

配置支持本地或远程的 Chat Completions 接口，以及温度、输出长度等请求参数。只有主动生成总结时才向配置的接口发送字幕。密钥配置、提示词变量和长度限制见 [AI 总结](docs/summary.md)，客户端接入方式见 [总结接口](docs/protocol.md#ai-总结)。

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

默认测试不需要语音模型。安装 FFmpeg、yt-dlp 和 Python 3 后，可在仓库根目录额外检查文件转换及视频任务：

```bash
cargo test -p substream --test offline --test video -- --ignored
```

这些测试使用真实 FFmpeg、yt-dlp 和固定识别结果，验证完整音轨、字幕内容与时间。视频来自本地测试服务，Cookie 来自临时 Firefox 资料目录。总结测试使用本地 HTTP 服务检查请求和结果，不调用付费模型，也不评估模型生成质量。CI 运行上述测试，并单独构建 `sherpa` 功能以检查原生库链接。

测试应围绕音频完整性、字幕内容与时间、协议兼容和失败处理。优先通过公开接口检查结果，避免固定内部调用顺序、队列容量或字幕更新次数。

修改浏览器捕获流程后，还需在实际浏览器中检查：开始和停止、弹窗关闭后重开、停止时保留末尾字幕、原音频继续播放，以及启动失败后可以重试。模型效果和延迟的测量方法见 [性能测试](docs/performance.md)。

## 代码布局

| 目录 | 职责 |
| --- | --- |
| `crates/core` | 音频与字幕数据、识别接口、字幕更新和导出 |
| `crates/protocol` | JSON 消息和二进制音频格式 |
| `crates/backends` | 识别引擎、FFmpeg、yt-dlp、LLM 客户端和子进程管理 |
| `apps/substream` | 命令行、配置、认证和本地服务 |
| `apps/overlay` | 参数界面、设备发现、进程控制和 KDE 悬浮字幕 |
| `browser/extension` | 标签页音频捕获、字幕预览、视频任务和总结接口 |
| `fixtures` | 协议和识别结果的共用样本 |

接入识别引擎时，实现 `StreamingRecognizer` 或 `BatchRecognizer`，再在应用的配置入口注册。核心模块不依赖具体引擎或网络库。
