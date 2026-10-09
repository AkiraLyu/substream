# 本地协议 v1

服务默认监听 `127.0.0.1:9743`，只允许绑定本机回环地址。`GET /health` 返回版本文本，`GET /v1/stream` 建立 WebSocket 连接。每个连接处理一条从零开始的音频时间轴。

## 连接与认证

浏览器请求的 `Origin` 必须与 `--allow-origin` 精确匹配。本地客户端可以不发送 `Origin`，但所有客户端都需要令牌认证。

连接后五秒内，第一条消息必须是以下 JSON：

```json
{"type":"authenticate","version":1,"token":"<64 位十六进制令牌>"}
```

认证消息可附带 `source` 字符串，最多 512 字节，用于显示捕获来源。浏览器扩展传入标签页标题；它只作为纯文本标签，不作为设备或权限依据。

认证成功后，服务加载识别模型。准备完成时发送：

```json
{"type":"ready","version":1,"backend":{"name":"…","model":"/models/encoder.onnx","threads":2,"languages":["ja"]}}
```

`name` 是识别引擎名称，`model` 是已加载的编码器文件路径，`threads` 是实际推理线程数，`languages` 是模型配置声明的语言。

客户端收到 `ready` 后才能发送音频。发送完毕后，用 `{"type":"finish"}` 请求结束。服务处理完已接收的音频，输出剩余字幕，再发送 `finished` 并关闭连接。

连接意外关闭表示取消，末尾字幕可能尚未定稿。重新连接会开始新的时间轴，不支持续传。

## 音频格式

一条二进制消息包含一个音频帧，由 24 字节头和音频采样数据组成。多字节整数使用小端序，即低位字节在前。

| 偏移（字节） | 长度（字节） | 内容 |
| --- | --- | --- |
| 0 | 4 | ASCII 字符 `SUBS` |
| 4 | 1 | 版本，固定为 `1` |
| 5 | 1 | 音频格式，固定为 `1`：16 位有符号整数 PCM |
| 6 | 1 | 声道数，固定为 `1` |
| 7 | 1 | 保留字段，固定为 `0` |
| 8 | 4 | 采样率，固定为 `16000` |
| 12 | 4 | `sequence`：帧序号，从 `0` 递增，超过 32 位无符号整数上限后回到 `0` |
| 16 | 8 | `start_sample`：首个采样的位置，从 `0` 开始，等于此前所有帧的采样数之和 |
| 24 | N × 2 | 音频采样，1 ≤ N ≤ 3200 |

建议每帧 320 个采样，即 20 毫秒；最后一帧可以更短。单条消息最多 6424 字节。服务会检查格式和音频是否连续，缺帧或时间不连续会结束会话。

转换为浮点音频时，每个采样值除以 `32768.0`。毫秒时间按累计采样位置换算，避免逐帧取整产生误差。

Rust 和 TypeScript 共用 [二进制样本](../fixtures/audio-v1.bin) 检查兼容性：帧序号为 `7`，起始采样位置为 `2240`，音频值为 `[-32768, 0, 32767]`。

## 字幕更新

```json
{
  "type": "caption",
  "caption": {
    "segment_id": 0,
    "revision": 2,
    "start_ms": 0,
    "end_ms": 400,
    "stable_text": "你好",
    "unstable_text": "世界",
    "is_final": false
  }
}
```

`segment_id` 标识一段字幕，`revision` 随该段更新递增。完整文本是 `stable_text` 与 `unstable_text` 的拼接结果。两部分都可能在后续更新中被修正。

客户端应替换同一段的旧文本，避免将每次更新都追加为新字幕。`is_final` 为 `true` 后，该段不再变化，可保存到字幕记录中。下一段的时间不能与它重叠。字幕应作为纯文本显示。

## 完成与错误

```json
{"type":"finished","samples_processed":16000,"processing_ms":12}
```

`samples_processed` 是已处理的采样总数。`processing_ms` 是音频处理和结束处理的累计耗时，不包含模型加载、网络传输、排队或界面渲染；测量用户看到字幕的延迟需要单独计时。

错误消息包含 `code` 和 `message`，随后服务结束当前连接：

```json
{"type":"error","code":"unauthorized","message":"…"}
```

| 错误代码 | 含义 |
| --- | --- |
| `unauthorized` | 首条消息无效、认证超时或令牌错误 |
| `busy` | 已有实时识别会话 |
| `stream_disabled` | 服务未配置实时识别模型 |
| `not_ready` | 模型就绪前发送了音频或结束请求 |
| `bad_audio` | 音频帧格式错误 |
| `bad_control` | 控制消息无效 |
| `overloaded` | 音频无法继续排队 |
| `worker_failed` | 识别或音频处理失败 |
| `worker_stopped` | 识别任务提前退出 |
| `timeout` | 初始化、等待输入或结束处理超时 |

服务限制音频缓冲量，处理不及时会结束会话。目前无输入的等待时间上限为 30 秒，也用于限制初始化和结束处理；单次网络发送最多等待 3 秒。客户端应按实时速度发送音频，并持续接收字幕。

## 桌面字幕订阅

`GET /v1/display` 建立只读 WebSocket 连接，供 KDE 显示端、GNOME Shell 扩展等客户端使用。它采用相同的来源检查和首条 `authenticate` 消息，但不启动模型，也不占用音频连接的名额。

认证成功后立即返回当前显示状态，之后在状态变化时继续发送：

```json
{
  "type": "display",
  "version": 1,
  "session_id": 1,
  "status": "listening",
  "backend": {"name": "sherpa-onnx", "model": "/models/encoder.onnx", "threads": 2, "languages": ["zh"]},
  "source": "浏览器标签页标题",
  "samples_received": 6400,
  "caption": {
    "segment_id": 0, "revision": 2,
    "start_ms": 0, "end_ms": 400,
    "stable_text": "你好", "unstable_text": "世界", "is_final": false
  },
  "caption_age_ms": 120,
  "message": null
}
```

- `session_id` 区分当前服务进程内的识别会话。开始新会话时递增；服务重启后从头计数，客户端重连时应丢弃本地旧状态。
- `status` 为 `idle`、`loading`、`listening`、`finished` 或 `error`，分别表示空闲、加载模型、模型已就绪、正常结束和识别错误。`listening` 不代表已有音频到达。
- `backend` 与音频接口含义相同，模型就绪前可为 `null`。结束状态中的模型信息表示该会话曾使用的模型。
- `source` 是客户端提供的捕获来源，未提供时为 `null`。
- `samples_received` 是已接收的采样数，每秒更新一次，正常结束时更新为实际处理总数；除以 16000 得到音频秒数。
- `caption` 是最新一段字幕，没有字幕时为 `null`。正常结束后保留末尾字幕；取消或开始新会话时清空。
- `caption_age_ms` 是字幕更新到本次发送之间的毫秒数，没有字幕时为 `null`。显示端用它计算剩余显示时间，避免重连后重新显示已过期的字幕。
- `message` 在识别错误时给出原因，其他状态为 `null`。

每条消息都是完整状态，中间更新可能合并，不能用此接口保存完整字幕历史。显示端应替换旧状态，空闲或断开时清空画面。字幕隐藏时间由显示端决定。

订阅连接可以保持空闲。发送音频或控制消息会收到 `read_only` 错误并断开；关闭订阅不会停止识别。接口仅包含通用 JSON 数据，不依赖 KDE、Qt 或 GNOME 类型。

## 视频任务

服务通过 `--video-config` 启用完整视频任务。接口使用 HTTP JSON，与实时音频和桌面字幕订阅独立。请求需带 `Authorization: Bearer <令牌>`；带有 `Origin` 时还需匹配同一份来源白名单。服务允许已授权来源发送 CORS 预检请求。

| 方法与路径 | 功能 |
| --- | --- |
| `POST /v1/video/jobs` | 提交 `{"url":"https://example.com/video"}`，返回 `202` 和任务状态 |
| `GET /v1/video/jobs/{id}` | 返回当前任务状态 |
| `DELETE /v1/video/jobs/{id}` | 请求取消，返回当前状态；不删除已完成的结果 |
| `GET /v1/video/jobs/{id}/document` | 完成后返回字幕文档，尚未完成时返回 `409` |

任务状态示例：

```json
{
  "id": "0123456789abcdef0123456789abcdef",
  "stage": "completed",
  "url": "https://example.com/video",
  "result": {
    "subtitle_source": "provided",
    "directory": "/output/video-abc123",
    "video": "/output/video-abc123/media.mkv",
    "srt": "/output/video-abc123/subtitles.srt",
    "vtt": "/output/video-abc123/subtitles.vtt",
    "document": "/output/video-abc123/document.json"
  },
  "error": null
}
```

`stage` 可为 `queued`、`checking_subtitles`、`downloading`、`importing_subtitles`、`converting`、`transcribing`、`cancelling`、`completed`、`cancelled` 或 `failed`。已有字幕走 `importing_subtitles`，没有可用字幕才进入 `converting` 和 `transcribing`。客户端应按阶段显示状态，不能据此推算完成百分比。仅 `completed` 带有 `result`；失败或取消的原因在 `error` 中。

结果和文档中的 `subtitle_source` 为 `provided`（人工字幕）、`automatic`（平台自动字幕）或 `recognition`（本地语音识别）。

结果路径属于运行服务的本机，视频扩展名由实际下载格式决定。扩展获取字幕内容时应使用 `/document`，不直接读取文件路径。文档格式如下：

```json
{
  "schema_version": 1,
  "source": {"url": "https://example.com/video", "id": "video-id", "title": "视频标题"},
  "subtitle_source": "provided",
  "transcript": {
    "schema_version": 1,
    "source": "https://example.com/video",
    "language": "zh",
    "segments": [{"id": 0, "start_ms": 0, "end_ms": 1500, "text": "字幕内容"}]
  }
}
```

时间以下载视频的音轨起点为零，与浏览器的播放进度无关。文档可作为翻译、总结等功能的输入，当前服务不调用 AI 总结模型。

服务同一时间只接受一个视频任务，忙时返回 `409 video_busy`。取消为异步操作，应继续查询到 `cancelled`；如果结果已保存，则可能返回 `completed`。关闭页面或断开请求不会取消任务，停止服务会取消未完成的任务。

任务记录只保留当前服务进程内最近的 32 项。重启或记录被移除后，查询返回 `404 job_not_found`，已完成的文件不受影响。未启用视频配置返回 `503 video_disabled`，令牌错误返回 `401 unauthorized`，来源不匹配返回 `403 forbidden_origin`，无效链接返回 `400 invalid_url`。任务错误响应包含 `code` 和 `message`。

扩展中的 `video.ts` 导出 `submitVideo`、`getVideoJob`、`cancelVideoJob` 和 `getVideoDocument`。扩展页面也可以使用后台消息接口：

```typescript
const response = await chrome.runtime.sendMessage({
  target: "background",
  type: "video",
  token,
  action: { kind: "submit", url: videoUrl },
});
```

其他操作的 `action` 为 `{kind: "status" | "cancel" | "document", id}`。成功返回 `{ok: true, data}`，失败返回 `{ok: false, error}`。后台只接收本扩展的消息，普通网页不能直接调用此接口。
