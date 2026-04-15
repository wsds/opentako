use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{Html, IntoResponse},
    Router,
    routing::get,
};
use axum::http::Method;
use tower_http::{
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
};

use crate::config::{WsEvent, WsRequest};
use crate::engine::TtsEngine;

pub async fn start(engine: TtsEngine) {
    // 🛡️ 核心：配置 CORS 和 PNA (Private Network Access)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any)
        .allow_private_network(true);
    println!("🐙 OpenTako 准备开启服务器!");

    let serve_dir = ServeDir::new("assets/html")
        .not_found_service(ServeFile::new("assets/html/index.html"));
    // 构建路由
    let app = Router::new()
        // .route("/", get(mock_config_page))
        .route("/api/ws", get(ws_handler))
        .fallback_service(serve_dir)
        .with_state(engine)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("🐙 OpenTako 引擎已就绪!");
    println!("🌐 配置页面: http://127.0.0.1:3000");
    println!("🔌 WebSocket: ws://127.0.0.1:3000/api/ws");

    axum::serve(listener, app).await.unwrap();
}

// 一个简单的配置页，这样从托盘唤起浏览器时不会看到 404
async fn mock_config_page() -> Html<&'static str> {
    Html(
        r#"
        <!DOCTYPE html>
        <html lang="zh-CN">
        <head>
            <meta charset="utf-8">
            <title>OpenTako 零延迟语音测试</title>
            <style>
                body { font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; padding: 40px; max-width: 600px; margin: 0 auto; background: #f5f7f9; }
                .card { background: white; padding: 30px; border-radius: 12px; box-shadow: 0 4px 12px rgba(0,0,0,0.1); }
                textarea { width: 100%; height: 100px; padding: 10px; border: 1px solid #ddd; border-radius: 8px; margin-bottom: 20px; font-size: 16px; resize: none; box-sizing: border-box; }
                button { background: #007bff; color: white; border: none; padding: 12px 24px; font-size: 16px; border-radius: 8px; cursor: pointer; transition: 0.2s; width: 100%; font-weight: bold; }
                button:hover { background: #0056b3; }
                button:disabled { background: #ccc; cursor: not-allowed; }
                #status { margin-top: 20px; font-size: 14px; color: #666; font-family: monospace; white-space: pre-wrap; background: #f8f9fa; padding: 10px; border-radius: 6px; border: 1px solid #eee; }
                .highlight { color: #28a745; font-weight: bold; }
            </style>
        </head>
        <body>
            <div class="card">
                <h2>🐙 OpenTako 零延迟测试</h2>
                <textarea id="textInput" placeholder="输入你想让 OpenTako 朗读的英文内容...">The quick brown fox jumps over the lazy dog.</textarea>
                <button id="playBtn" disabled>正在连接引擎...</button>
                <div id="status">连接状态: 初始化中...</div>
            </div>

            <script>
                const textInput = document.getElementById('textInput');
                const playBtn = document.getElementById('playBtn');
                const statusDiv = document.getElementById('status');

                let ws;
                let audioCtx;
                // Piper VITS 模型的标准采样率通常是 22050
                const SAMPLE_RATE = 22050;

                function log(msg) {
                    console.log(msg);
                    statusDiv.innerHTML = msg + '\n' + statusDiv.innerHTML;
                }

                function initWebSocket() {
                    ws = new WebSocket("ws://127.0.0.1:3000/api/ws");
                    // 🚀 核心：明确告诉浏览器我们期望接收原生二进制数组
                    ws.binaryType = "arraybuffer";

                    ws.onopen = () => {
                        log("<span class='highlight'>[🟢 连接成功] 已连接到 OpenTako 引擎通道</span>");
                        playBtn.innerText = "⚡ 立刻生成并播放";
                        playBtn.disabled = false;
                    };

                    ws.onmessage = (event) => {
                        // 1. 处理 JSON 控制帧
                        if (typeof event.data === "string") {
                            const data = JSON.parse(event.data);
                            log(`[📡 控制帧] 收到事件: ${data.event}`);
                        }
                        // 2. 处理纯二进制音频帧
                        else if (event.data instanceof ArrayBuffer) {
                            const byteSize = event.data.byteLength;
                            log(`<span class='highlight'>[🎵 音频帧] 接收到 ${byteSize} 字节 PCM 数据!</span>`);
                            playAudio(event.data);
                        }
                    };

                    ws.onclose = () => {
                        log("[🔴 连接断开] 与引擎失去连接");
                        playBtn.innerText = "连接已断开";
                        playBtn.disabled = true;
                    };
                }

                let nextStartTime = 0;

                // Web Audio API 播放 Float32LE 原生二进制数据 (无缝排队版)
                function playAudio(arrayBuffer) {
                    if (!audioCtx) return;

                    const float32Data = new Float32Array(arrayBuffer);
                    const audioBuffer = audioCtx.createBuffer(1, float32Data.length, SAMPLE_RATE);
                    audioBuffer.getChannelData(0).set(float32Data);

                    const source = audioCtx.createBufferSource();
                    source.buffer = audioBuffer;
                    source.connect(audioCtx.destination);

                    // 🚀 核心调度算法：
                    // currentTime 是声卡当前的时间。
                    // 如果 nextStartTime 落后于当前时间，说明队列空了，立刻播放。
                    // 否则，就把这段音频排在 nextStartTime 的位置。
                    let currentTime = audioCtx.currentTime;
                    if (nextStartTime < currentTime) {
                        nextStartTime = currentTime;
                    }

                    // 安排在未来的确切时间点播放
                    source.start(nextStartTime);

                    // 更新下一段音频的起始时间 (当前片段时长 + 0.1秒的自然呼吸停顿)
                    nextStartTime += audioBuffer.duration + 0.1;
                }

                playBtn.addEventListener('click', () => {
                    // 浏览器安全策略：AudioContext 必须在用户点击事件中初始化
                    if (!audioCtx) {
                        audioCtx = new (window.AudioContext || window.webkitAudioContext)();
                    }

                    const text = textInput.value.trim();
                    if (!text) return;

                    nextStartTime = 0; // 🎯 重置播放队列

                    log(`\n[🚀 发送请求] 推理文本: "${text}"`);

                    // 构造我们之前约定的 JSON 协议
                    const request = {
                        task_id: "req_" + Date.now(),
                        action: "tts",
                        payload: {
                            text: text,
                            model_id: "vits-piper-en_US-hfc_female-medium"
                        }
                    };

                    ws.send(JSON.stringify(request));
                });

                // 启动！
                initWebSocket();
            </script>
        </body>
        </html>
        "#
    )
}

// 处理 WS 升级请求
async fn ws_handler(ws: WebSocketUpgrade, axum::extract::State(engine): axum::extract::State<TtsEngine>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, engine))
}

// 核心的 WebSocket 状态机
async fn handle_socket(mut socket: WebSocket, engine: TtsEngine) {
    println!("🟢 客户端已连接 WebSocket");

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            println!("收到指令: {}", text);

            // 解析 JSON
            if let Ok(req) = serde_json::from_str::<WsRequest>(&text) {
                if req.action == "tts" {
                    println!("📚 接收到长文本，开始切片推理流水线...");

                    let start_event = WsEvent {
                        task_id: req.task_id.clone(),
                        event: "audio_start".to_string(),
                        format: Some("pcm_f32le".to_string()),
                        sample_rate: Some(22050),
                        channels: Some(1),
                    };
                    let _ = socket.send(Message::Text(serde_json::to_string(&start_event).unwrap())).await;

                    // 1. 调用引擎的切片器
                    let chunks = crate::engine::normalize_and_chunk(&req.payload.text);
                    let total_chunks = chunks.len();
                    println!("🔪 文章已被切分为 {} 个句子片段", total_chunks);

                    let mut chunk_idx = 1;

                    // 2. 循环遍历每一个句子，边推理边发送
                    for chunk in chunks {
                        let start_time = std::time::Instant::now();

                        match engine.generate_audio(&chunk) {
                            Ok(audio_bytes) => {
                                let elapsed = start_time.elapsed().as_millis();
                                println!("⚡ [{}/{}] 推理完成: {} ms -> {}", chunk_idx, total_chunks, elapsed, chunk);

                                // 核心：只要算出一段，立刻通过 WebSocket 扔给前端！
                                let _ = socket.send(Message::Binary(audio_bytes)).await;
                            }
                            Err(e) => {
                                println!("❌ 推理失败: {}", e);
                            }
                        }
                        chunk_idx += 1;
                    }

                    let end_event = WsEvent {
                        task_id: req.task_id,
                        event: "audio_end".to_string(),
                        format: None,
                        sample_rate: None,
                        channels: None,
                    };
                    let _ = socket.send(Message::Text(serde_json::to_string(&end_event).unwrap())).await;
                    println!("🏁 全文流式下发完毕！\n");
                }
            } else {
                println!("⚠️ 无法解析 JSON 格式");
            }
        }
    }
    println!("🔴 客户端已断开 WebSocket");
}