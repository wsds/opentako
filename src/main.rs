#![windows_subsystem = "windows"]

use std::env;
use std::path::PathBuf;
// 引入所有模块
mod config;
mod server;
mod tray;
mod engine;

fn get_asset_path(relative_path: &str) -> String {
    // 拿到当前 open-tako.exe 所在的绝对路径
    let mut path = env::current_exe().expect("无法获取当前程序路径");
    // 弹出 exe 自己的文件名，退回到它所在的文件夹
    path.pop();
    // 拼接上我们想要的相对路径
    path.push(relative_path);

    // 转成字符串返回
    path.to_str().unwrap().to_string()
}

fn main() {
    println!("🐙 OpenTako 准备开启服务器!");

    let model_path = get_asset_path("assets/models/en_US-hfc_female-medium.onnx");
    let lexicon_path = get_asset_path("assets/lexicon.json");

    // 1. 在后台操作系统线程中，独立启动 Tokio 运行时
    // 这样 Axum 极速网络服务就不会被 Windows 的托盘 UI 阻塞
    let tts_engine = engine::TtsEngine::new(&model_path, &lexicon_path)
        .expect("初始化 AI 引擎失败！请确保 assets 文件夹完整。");

    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().expect("初始化 Tokio 运行时失败");
        rt.block_on(async {
            server::start(tts_engine).await;
        });
    });

    // 2. 在主线程中运行托盘的事件循环 (它会一直阻塞在这里)
    tray::run_event_loop();
}