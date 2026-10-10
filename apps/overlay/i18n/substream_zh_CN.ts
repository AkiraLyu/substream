<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE TS>
<TS version="2.1" language="zh_CN" sourcelanguage="en">
<context>
    <name>AudioDevices</name>
    <message>
        <location filename="../src/audio_devices.cpp" line="23"/>
        <source>Cannot list audio devices. Check that pw-dump is installed.</source>
        <translation>无法读取音频设备，请确认已安装 PipeWire 工具（pw-dump）。</translation>
    </message>
    <message>
        <location filename="../src/audio_devices.cpp" line="31"/>
        <source>Cannot list PipeWire devices. Check that the audio service is running.</source>
        <translation>无法读取 PipeWire 设备，请检查音频服务是否运行。</translation>
    </message>
    <message>
        <location filename="../src/audio_devices.cpp" line="48"/>
        <source>System output · %1</source>
        <translation>系统输出 · %1</translation>
    </message>
    <message>
        <location filename="../src/audio_devices.cpp" line="48"/>
        <source>Audio input · %1</source>
        <translation>音频输入 · %1</translation>
    </message>
</context>
<context>
    <name>CaptionModel</name>
    <message>
        <location filename="../src/caption_model.cpp" line="33"/>
        <source>Invalid caption event</source>
        <translation>字幕消息无效</translation>
    </message>
    <message>
        <location filename="../src/caption_model.cpp" line="57"/>
        <source>Unsupported display protocol</source>
        <translation>不支持此显示协议</translation>
    </message>
    <message>
        <location filename="../src/caption_model.cpp" line="72"/>
        <source>Unknown display status</source>
        <translation>未知的显示状态</translation>
    </message>
    <message>
        <location filename="../src/caption_model.cpp" line="84"/>
        <source>Unsupported stream protocol</source>
        <translation>不支持此音频流协议</translation>
    </message>
    <message>
        <location filename="../src/caption_model.cpp" line="98"/>
        <source>Caption source failed</source>
        <translation>字幕源出错</translation>
    </message>
    <message>
        <location filename="../src/caption_model.cpp" line="101"/>
        <source>Unknown caption event</source>
        <translation>未知的字幕消息</translation>
    </message>
</context>
<context>
    <name>EventSource</name>
    <message>
        <location filename="../src/event_source.cpp" line="42"/>
        <source>Invalid JSON caption message</source>
        <translation>字幕消息不是有效的 JSON</translation>
    </message>
    <message>
        <location filename="../src/event_source.cpp" line="52"/>
        <source>Pipe caption events into --stdin or redirect an event file</source>
        <translation>请通过管道或文件重定向向 --stdin 传入字幕消息</translation>
    </message>
    <message>
        <location filename="../src/event_source.cpp" line="146"/>
        <location filename="../src/event_source.cpp" line="154"/>
        <source>Caption message is too large</source>
        <translation>字幕消息过大</translation>
    </message>
</context>
<context>
    <name>FilePicker</name>
    <message>
        <location filename="../src/widgets.cpp" line="25"/>
        <source>Browse…</source>
        <translation>选择…</translation>
    </message>
    <message>
        <location filename="../src/widgets.cpp" line="31"/>
        <source>Choose a token file location</source>
        <translation>选择令牌位置</translation>
    </message>
    <message>
        <location filename="../src/widgets.cpp" line="34"/>
        <source>Choose a file</source>
        <translation>选择文件</translation>
    </message>
</context>
<context>
    <name>JsonProcess</name>
    <message>
        <location filename="../src/json_process.cpp" line="11"/>
        <source>Cancellation timed out. Substream was terminated.</source>
        <translation>取消超时，已强制结束 Substream。</translation>
    </message>
    <message>
        <location filename="../src/json_process.cpp" line="29"/>
        <source>Cannot start Substream: %1</source>
        <translation>无法启动 Substream：%1</translation>
    </message>
    <message>
        <location filename="../src/json_process.cpp" line="37"/>
        <source>Substream returned an incomplete JSON message.</source>
        <translation>Substream 返回的 JSON 消息不完整。</translation>
    </message>
    <message>
        <location filename="../src/json_process.cpp" line="41"/>
        <source>Substream did not finish normally.
%1</source>
        <translation>Substream 未正常结束。
%1</translation>
    </message>
    <message>
        <location filename="../src/json_process.cpp" line="105"/>
        <location filename="../src/json_process.cpp" line="118"/>
        <source>Substream returned a JSON message larger than 4 MiB.</source>
        <translation>Substream 返回的 JSON 消息超过 4 MiB。</translation>
    </message>
    <message>
        <location filename="../src/json_process.cpp" line="112"/>
        <source>Substream returned an invalid JSON message.</source>
        <translation>Substream 返回的 JSON 消息无效。</translation>
    </message>
</context>
<context>
    <name>KdeWindow</name>
    <message>
        <location filename="../src/kde_window.cpp" line="26"/>
        <source>Display not found; use --list-screens to see available displays</source>
        <translation>未找到屏幕，请使用 --list-screens 查看可用屏幕</translation>
    </message>
    <message>
        <location filename="../src/kde_window.cpp" line="44"/>
        <source>Cannot load the subtitle view</source>
        <translation>无法加载字幕视图</translation>
    </message>
    <message>
        <location filename="../src/kde_window.cpp" line="53"/>
        <source>Subtitle display disconnected. Restart with an available display.</source>
        <translation>字幕屏幕已断开，请选择可用屏幕后重新启动。</translation>
    </message>
</context>
<context>
    <name>MainWindow</name>
    <message>
        <location filename="../src/main_window.cpp" line="69"/>
        <source>Live captions</source>
        <translation>实时字幕</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="92"/>
        <source>System audio</source>
        <translation>系统音频</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="92"/>
        <source>Browser tab</source>
        <translation>浏览器标签页</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="94"/>
        <source>Capture source</source>
        <translation>捕获方式</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="99"/>
        <source>Audio device</source>
        <translation>音频设备</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="105"/>
        <source>Refresh</source>
        <translation>刷新</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="113"/>
        <source>32-character ID from the extensions page</source>
        <translation>扩展管理页中的 32 位 ID</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="114"/>
        <source>Chromium extension ID</source>
        <translation>Chromium 扩展 ID</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="119"/>
        <source>Pairing token file</source>
        <translation>配对令牌文件</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="120"/>
        <location filename="../src/main_window.cpp" line="248"/>
        <source>All files (*)</source>
        <translation>所有文件 (*)</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="121"/>
        <source>Copy pairing token</source>
        <translation>复制配对令牌</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="140"/>
        <source>%1
Start the service before pairing for the first time.</source>
        <translation>%1
首次配对前，请先启动服务。</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="142"/>
        <location filename="../src/main_window.cpp" line="205"/>
        <source>Audio source</source>
        <translation>音频源</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="149"/>
        <source>Model configuration (.toml)</source>
        <translation>模型配置（.toml）</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="152"/>
        <source>Use model settings</source>
        <translation>使用模型配置</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="153"/>
        <source>Model configuration file</source>
        <translation>模型配置文件</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="154"/>
        <source>Model configuration (*.toml);;All files (*)</source>
        <translation>模型配置 (*.toml);;所有文件 (*)</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="155"/>
        <source>CPU threads</source>
        <translation>CPU 推理线程数</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="249"/>
        <source>Substream executable</source>
        <translation>Substream 程序</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="158"/>
        <source>Model</source>
        <translation>模型</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="71"/>
        <source>Video subtitles</source>
        <translation>视频字幕</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="73"/>
        <source>AI summary</source>
        <translation>AI 总结</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="164"/>
        <source>Show desktop captions</source>
        <translation>显示桌面悬浮字幕</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="169"/>
        <source>Desktop captions require KDE Wayland.</source>
        <translation>悬浮字幕仅支持 KDE Wayland。</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="174"/>
        <location filename="../src/main_window.cpp" line="175"/>
        <location filename="../src/main_window.cpp" line="176"/>
        <source> px</source>
        <translation> 像素</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="177"/>
        <source> s</source>
        <translation> 秒</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="178"/>
        <source>Display</source>
        <translation>显示屏幕</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="179"/>
        <source>Text size</source>
        <translation>字幕字号</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="180"/>
        <source>Maximum width</source>
        <translation>最大宽度</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="181"/>
        <source>Bottom margin</source>
        <translation>底部距离</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="182"/>
        <source>Hide after inactivity</source>
        <translation>无更新后隐藏</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="185"/>
        <source>Caption display</source>
        <translation>字幕显示</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="187"/>
        <source>Current session</source>
        <translation>当前会话</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="200"/>
        <location filename="../src/main_window.cpp" line="384"/>
        <source>No capture</source>
        <translation>未捕获</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="201"/>
        <location filename="../src/main_window.cpp" line="397"/>
        <source>Not loaded</source>
        <translation>未加载</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="204"/>
        <source>0:00 · 16 kHz / mono</source>
        <translation>0:00 · 16 kHz / 单声道</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="206"/>
        <source>Speech model</source>
        <translation>识别模型</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="207"/>
        <source>Audio received</source>
        <translation>已接收音频</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="212"/>
        <source>Input level %p%</source>
        <translation>输入电平 %p%</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="216"/>
        <source>Waiting for captions</source>
        <translation>等待字幕</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="225"/>
        <source>Runtime messages</source>
        <translation>运行消息</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="228"/>
        <source>Logs</source>
        <translation>运行日志</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="238"/>
        <source>Start captions</source>
        <translation>开始字幕</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="241"/>
        <location filename="../src/main_window.cpp" line="379"/>
        <source>Stop</source>
        <translation>停止</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="325"/>
        <source>Primary display</source>
        <translation>主屏幕</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="379"/>
        <source>Stop service</source>
        <translation>停止服务</translation>
    </message>
    <message numerus="yes">
        <location filename="../src/main_window.cpp" line="393"/>
        <source>%n thread(s)</source>
        <translation>
            <numerusform>%n 个线程</numerusform>
        </translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="394"/>
        <source>%1
%2 · %3</source>
        <translation>%1
%2 · %3</translation>
    </message>
    <message>
        <location filename="../src/main_window.cpp" line="402"/>
        <source>%1:%2 · 16 kHz / mono</source>
        <translation>%1:%2 · 16 kHz / 单声道</translation>
    </message>
</context>
<context>
    <name>SessionController</name>
    <message>
        <location filename="../src/session_controller.h" line="67"/>
        <source>Not started</source>
        <translation>尚未启动</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="20"/>
        <source>Stopping timed out. The speech process was terminated.</source>
        <translation>停止超时，识别进程已结束。</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="21"/>
        <source>Startup timed out. Check the model and audio device.</source>
        <translation>启动超时，请检查模型和音频设备。</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="27"/>
        <source>The audio device is not sending data. Check its connection and try again.</source>
        <translation>音频设备未提供数据，请检查连接后重试。</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="68"/>
        <source>Waiting for audio…</source>
        <translation>等待音频数据…</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="80"/>
        <source>Cannot start audio capture. Check that pw-cat is installed.</source>
        <translation>无法启动音频采集，请确认已安装 pw-cat。</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="74"/>
        <source>Cannot start the speech process: %1</source>
        <translation>无法启动识别程序：%1</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="76"/>
        <source>Cannot communicate with the speech process: %1</source>
        <translation>识别进程通信失败：%1</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="87"/>
        <source>Audio capture stopped unexpectedly. Check the device.
%1</source>
        <translation>音频采集意外结束，请检查设备。
%1</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="104"/>
        <source>The speech process did not finish normally.
%1</source>
        <translation>识别进程未正常完成。
%1</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="160"/>
        <source>Cannot start</source>
        <translation>无法启动</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="166"/>
        <source>Choose the Substream executable and a valid model configuration.</source>
        <translation>请选择 Substream 程序和有效的模型配置文件。</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="170"/>
        <source>Choose an audio device to capture.</source>
        <translation>请选择要捕获的音频设备。</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="175"/>
        <source>Choose a valid video configuration.</source>
        <translation>请选择有效的视频任务配置文件。</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="180"/>
        <source>Enter the 32-character Chromium extension ID.</source>
        <translation>请输入 Chromium 扩展的 32 位 ID。</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="184"/>
        <source>Cannot create the token directory.</source>
        <translation>无法创建令牌所在目录。</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="203"/>
        <source>Connecting to the local service…</source>
        <translation>正在连接本地服务…</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="213"/>
        <source>Loading the model…</source>
        <translation>正在加载模型…</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="214"/>
        <source>Starting the browser caption service…</source>
        <translation>正在启动浏览器字幕服务…</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="252"/>
        <source>The speech process returned an invalid ready message.</source>
        <translation>识别程序返回了无效的就绪消息。</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="277"/>
        <source>Browser connected. Loading the model…</source>
        <translation>浏览器已连接，正在加载模型…</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="279"/>
        <source>Capturing tab audio</source>
        <translation>正在捕获标签页音频</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="280"/>
        <source>Model ready. Waiting for tab audio…</source>
        <translation>模型已就绪，等待标签页音频…</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="282"/>
        <source>Tab capture finished. Ready to start again.</source>
        <translation>标签页捕获已结束，等待下一次开始</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="284"/>
        <source>Browser recognition failed. Try again in the extension.</source>
        <translation>浏览器识别失败，可在扩展中重试</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="287"/>
        <source>Service started. Waiting for the browser.</source>
        <translation>服务已启动，等待浏览器连接</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="306"/>
        <source>Capture stopped because recognition is too slow. Reduce system load or choose a smaller model.</source>
        <translation>识别速度跟不上音频，已停止采集。请减少负载或选择较小的模型。</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="311"/>
        <source>Cannot send audio to the speech process.</source>
        <translation>无法向识别程序发送音频。</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="321"/>
        <source>Capturing audio</source>
        <translation>正在捕获音频</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="329"/>
        <source>Stopping and saving final captions…</source>
        <translation>正在停止，保存末尾字幕…</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="352"/>
        <source>The capture device disconnected. Choose an audio source again.</source>
        <translation>捕获设备已断开，请重新选择音频源。</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="360"/>
        <source>Stopping processes…</source>
        <translation>正在结束进程…</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="381"/>
        <source>Stopped</source>
        <translation>已停止</translation>
    </message>
    <message>
        <location filename="../src/session_controller.cpp" line="381"/>
        <source>Startup or capture failed</source>
        <translation>启动或运行失败</translation>
    </message>
</context>
<context>
    <name>SummaryController</name>
    <message>
        <location filename="../src/summary_controller.cpp" line="12"/>
        <source>Substream returned an invalid summary.</source>
        <translation>Substream 返回的总结无效。</translation>
    </message>
    <message>
        <location filename="../src/summary_controller.cpp" line="24"/>
        <source>Substream did not return a summary.</source>
        <translation>Substream 未返回总结。</translation>
    </message>
    <message>
        <location filename="../src/summary_controller.cpp" line="41"/>
        <source>Choose Substream, a subtitle document, and a summary configuration.</source>
        <translation>请选择 Substream 程序、字幕文档和总结配置。</translation>
    </message>
    <message>
        <location filename="../src/summary_controller.cpp" line="51"/>
        <source>Cancelling…</source>
        <translation>正在取消…</translation>
    </message>
    <message>
        <location filename="../src/summary_controller.cpp" line="53"/>
        <source>Generating summary…</source>
        <translation>正在生成总结…</translation>
    </message>
    <message>
        <location filename="../src/summary_controller.cpp" line="55"/>
        <source>Summary failed</source>
        <translation>总结失败</translation>
    </message>
    <message>
        <location filename="../src/summary_controller.cpp" line="57"/>
        <source>Cancelled</source>
        <translation>已取消</translation>
    </message>
    <message>
        <location filename="../src/summary_controller.cpp" line="59"/>
        <source>Completed</source>
        <translation>已完成</translation>
    </message>
    <message>
        <location filename="../src/summary_controller.cpp" line="60"/>
        <source>Not started</source>
        <translation>尚未启动</translation>
    </message>
</context>
<context>
    <name>SummaryPage</name>
    <message>
        <location filename="../src/summary_page.cpp" line="57"/>
        <location filename="../src/summary_page.cpp" line="59"/>
        <source>Cannot read the saved AI settings.</source>
        <translation>无法读取已保存的 AI 设置。</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="83"/>
        <source> s</source>
        <translation> 秒</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="87"/>
        <source>API endpoint</source>
        <translation>接口地址</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="88"/>
        <source>API key</source>
        <translation>API 密钥</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="89"/>
        <source>Model</source>
        <translation>模型</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="90"/>
        <source>Timeout</source>
        <translation>超时时间</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="93"/>
        <source>API</source>
        <translation>接口</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="98"/>
        <source>System prompt</source>
        <translation>系统提示词</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="102"/>
        <source>User prompt</source>
        <translation>用户提示词</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="105"/>
        <source>Insert variable</source>
        <translation>插入变量</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="106"/>
        <source>Subtitles</source>
        <translation>字幕</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="107"/>
        <source>Title</source>
        <translation>标题</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="108"/>
        <source>Source link</source>
        <translation>来源链接</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="118"/>
        <source>Reset prompts</source>
        <translation>重置提示词</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="124"/>
        <source>Prompts</source>
        <translation>提示词</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="131"/>
        <source>API key environment variable</source>
        <translation>API 密钥环境变量</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="132"/>
        <source>Input character limit</source>
        <translation>输入字符上限</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="134"/>
        <source>Request parameters (JSON)</source>
        <translation>请求参数（JSON）</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="140"/>
        <source>Advanced</source>
        <translation>高级</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="142"/>
        <source>Summary</source>
        <translation>总结</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="147"/>
        <source>Subtitle document</source>
        <translation>字幕文档</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="149"/>
        <source>Subtitle documents (*.json)</source>
        <translation>字幕文档 (*.json)</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="168"/>
        <source>Save settings</source>
        <translation>保存设置</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="171"/>
        <source>Save Markdown</source>
        <translation>保存 Markdown</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="172"/>
        <source>Generate summary</source>
        <translation>生成总结</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="173"/>
        <source>Cancel</source>
        <translation>取消</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="181"/>
        <source>Settings saved</source>
        <translation>设置已保存</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="216"/>
        <source>Request parameters must be a JSON object.</source>
        <translation>请求参数必须是 JSON 对象。</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="222"/>
        <source>Cannot create the settings directory.</source>
        <translation>无法创建设置目录。</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="230"/>
        <source>Cannot save the AI settings: %1</source>
        <translation>无法保存 AI 设置：%1</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="263"/>
        <source>Save summary</source>
        <translation>保存总结</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="264"/>
        <source>Markdown (*.md)</source>
        <translation>Markdown (*.md)</translation>
    </message>
    <message>
        <location filename="../src/summary_page.cpp" line="270"/>
        <source>Cannot save the summary: %1</source>
        <translation>无法保存总结：%1</translation>
    </message>
</context>
<context>
    <name>TokenFile</name>
    <message>
        <location filename="../src/token_file.cpp" line="15"/>
        <source>The token must be a regular file readable and writable only by its owner (permissions 0600).</source>
        <translation>令牌必须是仅当前用户可读写的普通文件（权限 0600）。</translation>
    </message>
    <message>
        <location filename="../src/token_file.cpp" line="22"/>
        <source>Cannot read the token file.</source>
        <translation>无法读取令牌文件。</translation>
    </message>
    <message>
        <location filename="../src/token_file.cpp" line="28"/>
        <source>The token must contain 64 hexadecimal characters.</source>
        <translation>令牌应包含 64 位十六进制字符。</translation>
    </message>
</context>
<context>
    <name>VideoController</name>
    <message>
        <location filename="../src/video_controller.cpp" line="20"/>
        <source>The video process did not return a completed result.</source>
        <translation>视频任务未返回完整结果。</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="36"/>
        <source>Enter a valid HTTP or HTTPS video link.</source>
        <translation>请输入有效的 HTTP 或 HTTPS 视频链接。</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="40"/>
        <source>Choose the Substream executable and a video configuration.</source>
        <translation>请选择 Substream 程序和视频配置文件。</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="52"/>
        <source>Choose a browser for cookies.</source>
        <translation>请选择读取 cookies 的浏览器。</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="59"/>
        <source>Choose an existing cookies file.</source>
        <translation>请选择有效的 cookies 文件。</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="79"/>
        <location filename="../src/video_controller.cpp" line="88"/>
        <source>The video process returned an invalid status message.</source>
        <translation>视频进程返回的状态消息无效。</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="107"/>
        <source>Cancelling…</source>
        <translation>正在取消…</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="109"/>
        <source>Starting…</source>
        <translation>正在启动…</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="111"/>
        <source>Checking available subtitles…</source>
        <translation>正在检查已有字幕…</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="113"/>
        <source>Downloading video…</source>
        <translation>正在下载视频…</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="115"/>
        <source>Using existing subtitles…</source>
        <translation>正在导入已有字幕…</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="117"/>
        <source>Converting audio…</source>
        <translation>正在转换音频…</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="119"/>
        <source>Recognizing speech…</source>
        <translation>正在识别语音…</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="121"/>
        <source>Cancelled</source>
        <translation>已取消</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="123"/>
        <source>Video task failed</source>
        <translation>视频任务失败</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="127"/>
        <source>Completed · Existing subtitles</source>
        <translation>已完成 · 已有字幕</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="129"/>
        <source>Completed · Platform auto-captions</source>
        <translation>已完成 · 平台自动字幕</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="130"/>
        <source>Completed · Speech recognition</source>
        <translation>已完成 · 语音识别</translation>
    </message>
    <message>
        <location filename="../src/video_controller.cpp" line="132"/>
        <source>Not started</source>
        <translation>尚未启动</translation>
    </message>
</context>
<context>
    <name>VideoPage</name>
    <message>
        <location filename="../src/video_page.cpp" line="40"/>
        <source>Video link</source>
        <translation>视频链接</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="43"/>
        <source>Video configuration</source>
        <translation>视频配置</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="44"/>
        <source>Video configuration (*.toml)</source>
        <translation>视频配置 (*.toml)</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="47"/>
        <source>Use configuration</source>
        <translation>使用配置文件</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="48"/>
        <source>None</source>
        <translation>不使用</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="49"/>
        <location filename="../src/video_page.cpp" line="65"/>
        <source>Browser</source>
        <translation>浏览器</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="50"/>
        <source>Cookies file</source>
        <translation>Cookies 文件</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="52"/>
        <source>Cookies</source>
        <translation>Cookies</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="64"/>
        <source>Default profile</source>
        <translation>默认用户配置</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="66"/>
        <source>Profile</source>
        <translation>用户配置</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="69"/>
        <source>Cookies files (*.txt);;All files (*)</source>
        <translation>Cookies 文件 (*.txt);;所有文件 (*)</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="80"/>
        <source>Video task</source>
        <translation>视频任务</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="97"/>
        <source>Subtitle preview</source>
        <translation>字幕预览</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="109"/>
        <source>Logs</source>
        <translation>日志</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="118"/>
        <source>Open output folder</source>
        <translation>打开输出目录</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="119"/>
        <source>AI summary</source>
        <translation>AI 总结</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="120"/>
        <source>Get video subtitles</source>
        <translation>获取视频字幕</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="121"/>
        <source>Cancel</source>
        <translation>取消</translation>
    </message>
    <message>
        <location filename="../src/video_page.cpp" line="193"/>
        <location filename="../src/video_page.cpp" line="199"/>
        <source>Cannot read the subtitle document.</source>
        <translation>无法读取字幕文档。</translation>
    </message>
</context>
<context>
    <name>main</name>
    <message>
        <location filename="../src/main.cpp" line="29"/>
        <source>Desktop controls and KDE Wayland captions</source>
        <translation>桌面参数设置与 KDE Wayland 悬浮字幕</translation>
    </message>
    <message>
        <location filename="../src/main.cpp" line="34"/>
        <source>Read newline-delimited caption events from standard input.</source>
        <translation>从标准输入逐行读取字幕消息。</translation>
    </message>
    <message>
        <location filename="../src/main.cpp" line="37"/>
        <source>Subscribe to the local subtitle service using a private token file.</source>
        <translation>使用私有令牌文件订阅本地字幕服务。</translation>
    </message>
    <message>
        <location filename="../src/main.cpp" line="41"/>
        <source>Display WebSocket URL (loopback only).</source>
        <translation>字幕显示的 WebSocket 地址（仅限本机回环地址）。</translation>
    </message>
    <message>
        <location filename="../src/main.cpp" line="44"/>
        <source>List available display names and exit.</source>
        <translation>列出可用屏幕名称后退出。</translation>
    </message>
    <message>
        <location filename="../src/main.cpp" line="46"/>
        <source>Display name; defaults to the primary display.</source>
        <translation>屏幕名称，默认使用主屏幕。</translation>
    </message>
    <message>
        <location filename="../src/main.cpp" line="49"/>
        <source>Maximum width in logical pixels.</source>
        <translation>最大宽度，单位为逻辑像素。</translation>
    </message>
    <message>
        <location filename="../src/main.cpp" line="52"/>
        <source>Text size in logical pixels.</source>
        <translation>文字大小，单位为逻辑像素。</translation>
    </message>
    <message>
        <location filename="../src/main.cpp" line="54"/>
        <source>Distance above the bottom edge in logical pixels.</source>
        <translation>距屏幕底部的距离，单位为逻辑像素。</translation>
    </message>
    <message>
        <location filename="../src/main.cpp" line="58"/>
        <source>Hide captions after this interval without an update.</source>
        <translation>没有新字幕时，经过指定时间后隐藏字幕。</translation>
    </message>
    <message>
        <location filename="../src/main.cpp" line="81"/>
        <source>This renderer requires a KDE Plasma Wayland session</source>
        <translation>悬浮字幕需要 KDE Plasma Wayland 会话</translation>
    </message>
    <message>
        <location filename="../src/main.cpp" line="86"/>
        <source>Choose either --stdin or --token-file</source>
        <translation>请在 --stdin 和 --token-file 中选择一种输入方式</translation>
    </message>
    <message>
        <location filename="../src/main.cpp" line="95"/>
        <source>%1 must be between %2 and %3</source>
        <translation>%1 必须在 %2 到 %3 之间</translation>
    </message>
    <message>
        <location filename="../src/main.cpp" line="117"/>
        <source>Server must be a loopback ws:// address with path /v1/display</source>
        <translation>服务地址必须使用本机回环 ws:// 地址，路径为 /v1/display</translation>
    </message>
</context>
</TS>
