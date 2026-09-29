# 本地协议 v1

监听默认 `127.0.0.1:9743`。`GET /health` 返回版本文本；`GET /v1/stream` 升级为 WebSocket。非 loopback 绑定被拒绝。一个连接就是一条音频时间轴，不复用 stream_id。

## 控制与状态

浏览器 Origin 必须精确匹配 `--allow-origin`；缺失 Origin 仅用于 native client，仍需令牌认证。不提供通配 CORS。第一条消息必须在五秒内是：

```json
{"type":"authenticate","version":1,"token":"<64 hex characters>"}
```

认证成功后获取单路推理配额，加载模型，然后发送：

```json
{"type":"ready","version":1,"backend":{"name":"…","synthetic":false,"languages":["ja"]}}
```

收到 ready 后才能送二进制音频。服务不会缓存模型加载期间的音频；客户端不得提前排队。每个连接最多等待 30 秒初始化/空闲/结束处理，网络发送超过三秒视为慢消费者。

```text
connect → authenticate → ready → audio* → finish → caption(final)* → finished → close
                                  └──── malformed/overload/disconnect → cancel
```

正常结束发送 `{"type":"finish"}`。服务处理完已接收音频并 flush 后才发送 finished。连接突然关闭不等于成功完成，不保证最后一个 partial 已定稿。finished 后重连会从新的零时钟开始。音频之后再次 authenticate 等非法控制消息会结束会话。

## 音频头

每条 binary message 是一个帧。整数为 little endian，头长 24 字节。

| 偏移 | 大小 | 值 |
| --- | --- | --- |
| 0 | 4 | ASCII `SUBS` |
| 4 | 1 | version = 1 |
| 5 | 1 | format = 1，PCM signed 16-bit LE |
| 6 | 1 | channels = 1 |
| 7 | 1 | reserved = 0 |
| 8 | 4 | sample_rate = 16000 |
| 12 | 4 | sequence，起始 0，按 u32 回绕递增 |
| 16 | 8 | start_sample，起始 0，等于上一帧末尾样本位置 |
| 24 | N × 2 | PCM payload，1 ≤ N ≤ 3200 |

推荐 N=320（20 ms），最后一帧可以更短。最大消息长 6424 字节。长度、格式、版本、采样率、保留位、sequence 和时钟都会检查。PCM 解码为 `sample / 32768.0`。采样数到毫秒使用绝对位置整数换算，不对每个帧长分别取整相加。

`fixtures/audio-v1.bin` 是跨 Rust/TypeScript 的 30 字节测试向量：sequence=7，start_sample=2240，payload=[-32768, 0, 32767]。

## 字幕事件

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

一段的 revision 单调增加，完整文本是 stable_text + unstable_text。稳定前缀可以被后续 revision 修正；final 才可存入已定稿 Transcript。消费者按 segment_id/revision 替换 partial，不将 partial 当新字幕追加。final 后该段不可再变，下一段时间不得与其重叠。文本按纯文本显示，禁止通过 innerHTML 插入。

```json
{"type":"finished","samples_processed":16000,"processing_ms":12}
```

processing_ms 是此会话各次 pipeline 调用及 flush 的耗时之和，不包括模型加载、网络、排队或 UI 渲染，不能作为端到端字幕延迟。`error` 含 code 和 message；可能的 code 有 unauthorized、busy、not_ready、bad_audio、bad_control、overloaded、worker_failed、worker_stopped、timeout。错误后当前会话终止，无自动降级后端。

初版没有 renderer 订阅端点、离线 job API、断线续传、视频 seek 映射或持久化 transcript。扩展只保存最新状态与字幕预览，完整历史应由后续 transcript sink 保存。
