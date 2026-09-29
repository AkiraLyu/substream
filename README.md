# Substream

面向 Linux 的本地语音与字幕引擎。目标是低延迟多语言实时字幕，以及浏览器视频的完整字幕生成。本仓库是可运行的初始框架，尚不是完整桌面产品。

需求拆解、对原方案的修正和下一步验收条件见 [架构决策](docs/architecture.md)；二进制格式、会话生命周期见 [协议说明](docs/protocol.md)。

## 已实现的边界

| 部分 | 当前实现 |
| --- | --- |
| 核心 | 16 kHz 单声道 PCM、采样时钟、partial/final 状态、Unicode 字幕稳定器、统一 Transcript |
| 实时识别 | 显式 demo 后端；可选 sherpa-onnx streaming transducer CPU 适配器 |
| 离线识别 | 本地媒体 → FFmpeg → whisper.cpp CLI → 校验后的 Transcript |
| 输出 | NDJSON 实时事件，SRT、WebVTT、JSON 文件；禁止覆盖已有文件 |
| 本地服务 | loopback WebSocket、令牌认证、Origin 白名单、有界队列、单路推理、超时与断开处理 |
| 浏览器 | Chromium MV3 扩展：tabCapture → offscreen → AudioWorklet → PCM16；弹窗预览字幕 |
| 验证 | Rust 单元/CLI/WebSocket 测试、Rust/TypeScript 共用协议样本、AudioWorklet 测试、基线 benchmark、CI |

**尚未实现：** 全局桌面悬浮层、原生 PipeWire 回调、网页整段媒体解析与任务接口、YouTube/Bilibili 专用解析器、词级对齐与阅读速度重分段、GPU 调度、翻译、AI 总结、持久化历史。浏览器捕获只有从启动时起实际播放的音频；不能用它提前生成整段视频字幕。

## 快速运行

Linux，Rust 1.88+。默认构建不需要模型、GTK、PipeWire 开发库或 GPU SDK。

```bash
cargo run -p substream -- demo
cargo run -p substream -- demo --format vtt --output demo.vtt
cargo run -p substream -- demo --format json
```

`demo` 输出的是固定合成字幕，完全不执行语音识别。JSON 与 WebSocket `ready` 事件包含 `synthetic: true`，浏览器会显示演示标识。任何真实后端失败都不会自动切换成 demo。

## 实时识别与系统音频

从 [sherpa-onnx 模型目录](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/index.html) 自行选择 streaming transducer 模型，将 encoder/decoder/joiner/tokens 路径填入 `configs/sherpa.example.toml`，同时修改其真实语言覆盖。日语模型和中英模型不能因使用同一运行时而视为具有相同语言能力。

```bash
cargo build -p substream --release --features sherpa
ffmpeg -nostdin -i input.mp4 -vn -ac 1 -ar 16000 -f s16le - |
  target/release/substream stream --backend sherpa --config configs/sherpa.example.toml
```

该 `stream` 命令输出逐行 JSON，不限速读取文件。测试实时节奏时可给 FFmpeg 添加 `-re`（放在 `-i` 之前）。

在 PipeWire 会话中，先用 `wpctl status -n` 确认输出设备，再将实际 sink 的 node name 填入：

```bash
pw-cat --record --raw --rate 16000 --channels 1 --format s16 --latency 20ms \
  --properties '{ stream.capture.sink=true }' --target '<sink-node-name>' - |
  target/release/substream stream --backend sherpa --config configs/sherpa.example.toml
```

这只是 `pw-cat` 进程桥接，实时预算也包含系统管道的缓存；尚未达到原生采集回调零分配的目标。系统音频节点与权限取决于会话管理器。[PipeWire 属性说明](https://docs.pipewire.org/group__pw__keys.html)

`sherpa` feature 固定依赖 `sherpa-onnx 1.13.8`，首次编译会由上游构建脚本下载匹配的原生库；也可用 `SHERPA_ONNX_LIB_DIR` 指定已有库。模型需单独提供。当前适配器明确使用 CPU。[原生库安装说明](https://k2-fsa.github.io/sherpa/onnx/rust-api/advanced-install.html)

## 浏览器链路

需要 Chromium 116+ 和 Node.js 24+（开发测试）。浏览器与 daemon 在同一台机器上。

```bash
cd browser/extension
npm ci --ignore-scripts
npm run check
npm run build
```

在浏览器扩展管理页启用开发者模式，加载 `browser/extension/dist`，复制扩展 ID。回到仓库根目录运行：

```bash
cargo run -p substream -- token --output /tmp/substream.token
cargo run -p substream -- serve --backend demo \
  --token-file /tmp/substream.token \
  --allow-origin chrome-extension://<extension-id>
```

打开有权处理、无保护限制的视频，点击扩展，把令牌文件内容填入弹窗并开始捕获。关闭弹窗不会停止捕获；重新打开后点击“停止”。字幕在弹窗中预览。真实 ASR 使用：

```bash
cargo run -p substream --release --features sherpa -- serve \
  --backend sherpa --config configs/sherpa.example.toml \
  --token-file /tmp/substream.token \
  --allow-origin chrome-extension://<extension-id>
```

令牌文件以 0600 创建；不要把令牌写进 URL。扩展只用 `storage.session` 保留配对信息，不向网页注入令牌。浏览器端使用 16 kHz AudioContext 完成重采样，原播放声音通过另一条原采样率监听链路保留。服务每次连接加载一次模型，`ready` 后才开始送音频；首轮加载不计作稳态字幕延迟。

此扩展没有全局站点读取权限和抓流逻辑，也没有 DRM 绕过功能。只实现 Chromium；Firefox 需要独立捕获适配器。

## 本地文件生成完整字幕

另行安装 [whisper.cpp 的 whisper-cli](https://github.com/ggml-org/whisper.cpp/tree/master/examples/cli) 和 FFmpeg，并准备所需模型：

```bash
cargo run -p substream --release -- transcribe input.mp4 \
  --model /path/to/ggml-model.bin \
  --whisper-bin /path/to/whisper-cli \
  --language auto --format srt --output captions.srt
```

`--format` 可选 `srt/vtt/json`；省略 `--output` 则输出到 stdout。原文识别和翻译保持独立。当前只接受本地文件；网页 URL、blob URL、带签名的 DASH/HLS 音轨还需媒体解析适配器。临时 WAV 随任务结束清理，Ctrl+C 取消子进程；`--timeout-secs` 是每个外部阶段的超时。

离线适配器读取 whisper.cpp JSON 中的毫秒 offsets，保留真实段落时间。它不把 BPE token 当成词，也不伪造词级时间。长段落会按显示宽度换行，目前不承诺最多两行或阅读速度限制。whisper-cli 的整段内存占用、模型重载和 GPU 吞吐尚待测量。

## 开发与验证

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
# 需要 ffmpeg 与 python3；真实音频归一化 + 模拟 whisper-cli 契约，不测试识别质量
cargo test -p substream --test offline -- --ignored
# 仅测传输解码和演示 pipeline 的框架开销
cargo run -p substream --release --example ingest_bench

cd browser/extension
npm run format:check
npm run check
npm run build
npm test
```

默认测试不下载语音模型。编译通过不等于识别质量、系统采集或浏览器权限流程已经得到实际验证。性能验收方法见 [性能计划](docs/performance.md)。

## 代码布局

```text
crates/core          音频与字幕领域模型、纯状态机、识别接口
crates/protocol      JSON 控制协议与二进制 PCM 编解码
crates/backends      demo、sherpa、FFmpeg、whisper.cpp、子进程生命周期
apps/substream      CLI、配置、认证、WebSocket、独立推理 worker
browser/extension   TypeScript MV3 捕获与字幕预览
configs             模型配置示例
fixtures            跨语言协议和后端契约样本
docs                设计依据、协议、性能验收与迭代顺序
```

添加模型只需实现 `StreamingRecognizer` 或 `BatchRecognizer`，并在配置入口注册。领域核心不依赖网络运行时或具体推理 SDK。未来字幕显示、翻译与总结应消费已定稿的 Transcript/事件，不访问采集回调。
