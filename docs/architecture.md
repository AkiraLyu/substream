# 架构设计

Substream 分别处理实时字幕和文件字幕。实时识别需要持续接收音频并及时更新文本；文件识别可以读取完整音频，更重视识别准确率和字幕时间。两者使用不同的识别接口，共用字幕数据和导出功能。

## 技术选择

| 技术 | 选择原因 |
| --- | --- |
| Rust | 用统一的数据类型组织音频、字幕和会话状态，便于接入本地识别库，并明确管理模型、连接和子进程的生命周期。 |
| Tokio 与 Axum | 处理 WebSocket 连接、超时和取消。同步推理在独立工作线程中运行，避免阻塞网络通信。 |
| sherpa-onnx | 提供连续音频的增量识别接口和 Rust 接口。当前使用 CPU 上的流式 Transducer 模型，支持的语言由模型决定。 |
| whisper.cpp CLI | 用于完整文件识别，提供段落文本和时间。进程接口便于独立安装和替换引擎，每次任务都需要重新加载模型。 |
| FFmpeg | 统一解码常见媒体格式，将音轨转换为识别所需的采样率和声道数。 |
| TypeScript 与 Chromium 扩展 API | 通过 `tabCapture` 获取用户选择的标签页音频，用后台文档维持捕获，以 `AudioWorklet` 处理音频；类型检查帮助约束各部分的消息格式。 |
| Qt Quick 与 LayerShellQt | Qt Quick 负责文本排版和屏幕缩放，LayerShellQt 将窗口放入 KDE Wayland 的悬浮层。C++ 显示端独立运行，通过 JSON 接收字幕，不加载识别模型。 |

接口用法可查阅 [sherpa-onnx Rust 文档](https://k2-fsa.github.io/sherpa/onnx/rust-api/index.html)、[whisper.cpp CLI](https://github.com/ggml-org/whisper.cpp/tree/master/examples/cli) 和 [Chrome 音频捕获说明](https://developer.chrome.com/docs/extensions/how-to/web-platform/screen-capture)。

## 数据流与模块职责

```mermaid
flowchart LR
    P[系统音频 / pw-cat] --> R[标准输入]
    B[浏览器标签页] --> W[AudioWorklet]
    W --> N[本地 WebSocket 服务]
    R --> S[实时识别]
    N --> S
    S --> C[字幕更新与时间检查]
    C --> U[JSON 事件 / 弹窗预览]
    C --> V[字幕显示接口]
    V --> K[KDE 悬浮字幕]
    F[本地媒体文件] --> D[FFmpeg 转换音频]
    D --> O[文件识别]
    O --> T[字幕文本与时间]
    T --> X[SRT / WebVTT / JSON]
```

`core` 定义音频、字幕和识别接口，负责时间检查、文本更新及导出。它不依赖网络、设备或具体识别引擎。`backends` 接入识别库和外部程序；`protocol` 定义客户端消息；应用层负责配置、认证和任务调度。

实时识别器在工作线程中创建、调用和销毁。网络服务通过队列交换音频与字幕。队列限制容量和等待时间，处理不及时便报告错误并结束连接，避免字幕持续落后于声音。目前一个服务只运行一个实时识别会话，以限制模型的内存占用。

正常停止时，服务处理完已接收的音频，再输出末尾字幕和完成消息。意外断开会取消任务。已进入本地识别库的计算仍需等待调用返回，不能立即强制中断。

## 字幕内容与时间

音频统一为 16 kHz 单声道，时间按累计采样数计算。静音也计入时间；模型结束识别所需的补充静音不延长字幕时间。

一段字幕可以多次更新。`stable_text` 表示近期较少变化的前缀，供界面区分显示；后续结果仍可修正它。客户端按段落编号替换旧文本，收到 `is_final: true` 后再保存最终结果。定稿后的段落不再修改。

文件字幕保留模型给出的段落时间。换行按显示宽度处理，避免拆开组合字符和表情。按词调整字幕时间需要额外的语音对齐结果，当前只进行换行和时间检查。

## 浏览器与桌面显示

扩展先等待本地服务加载模型，再申请标签页音频，避免短期有效的捕获凭据在加载期间过期。后台文档持有音频流，因此关闭弹窗后仍能继续捕获。音频在浏览器中转换为 16 kHz，同时通过独立音频输出保留原声音。[tabCapture 接口说明](https://developer.chrome.com/docs/extensions/reference/api/tabCapture)

标签页捕获的时间从开始采集时算起。视频暂停、跳转或变速后，它不再对应视频时间，因此整段视频字幕应从完整媒体文件生成。扩展目前只保存最新状态和字幕预览。

KDE 显示端通过 LayerShellQt 创建悬浮窗口，设置鼠标穿透、无键盘焦点和不占用桌面布局。窗口按所选屏幕的逻辑尺寸排版，Qt 处理缩放。[LayerShellQt](https://github.com/KDE/layer-shell-qt)、[Qt 窗口属性](https://doc.qt.io/qt-6/qt.html#WindowType-enum)。

显示程序有两种输入：`stream` 命令的逐行 JSON，以及本地服务的 `/v1/display` 订阅。订阅使用相同的令牌认证，只返回最新字幕状态，不接收音频，也不占用识别会话。服务合并尚未发送的更新，显示程序关闭或读取缓慢不会阻塞推理。

`CaptionModel` 处理段落替换和显示时长，`EventSource` 接收消息，`KdeWindow` 负责 KDE 窗口设置，QML 负责排版。桌面无关的消息类型定义在 `substream-protocol::display` 中。GNOME Shell 扩展可以直接订阅该接口，不需要使用 Qt 或修改识别核心；当前尚未提供 GNOME 显示端。

## 其他扩展接口

`StreamingRecognizer` 接收连续音频，`BatchRecognizer` 接收完整音频文件，两者返回统一的字幕数据。系统音频目前通过标准输入接入，媒体输入目前只接受本地文件。音频采集和网站媒体获取应分别放在输入模块中。

已定稿的 `Transcript` 包含文本和来源时间，可供翻译、总结或历史记录使用。这些功能不需要接触音频采集回调。
