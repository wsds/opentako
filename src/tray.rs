use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
};

// ... 其他引用保持不变 ...

fn load_embedded_icon() -> tray_icon::Icon {
    // 👑 核心魔法：include_bytes! 会在【编译期】将项目根目录的 icon.png
    // 直接转化为字节数组打入最终的二进制程序中。
    // 注意路径：相对于当前的 tray.rs 文件，所以用 "../icon.png"
    let icon_bytes = include_bytes!("../assets/icon.png");

    let (icon_rgba, icon_width, icon_height) = {
        // 使用 load_from_memory 替代 open
        let image = image::load_from_memory(icon_bytes)
            .expect("解码内置图标失败")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("图标格式转换失败")
}

// 辅助函数：在内存中生成一个 32x32 的纯蓝色占位图标
fn generate_dummy_icon() -> tray_icon::Icon {
    let width = 32;
    let height = 32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        // R, G, B, A (纯蓝色)
        rgba.extend_from_slice(&[0, 120, 255, 255]);
    }
    tray_icon::Icon::from_rgba(rgba, width, height).expect("生成占位图标失败")
}

pub fn run_event_loop() {
    // 1. 初始化系统事件循环
    let event_loop = EventLoopBuilder::new().build();

    // 2. 创建右键菜单项
    let open_config_item = MenuItem::new("打开配置页面", true, None);
    let quit_item = MenuItem::new("退出 OpenTako", true, None);
    let menu = Menu::new();
    let _ = menu.append(&open_config_item);
    let _ = menu.append(&PredefinedMenuItem::separator()); // 添加一条分割线
    let _ = menu.append(&quit_item);

    // 3. 构建并显示系统托盘图标
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("OpenTako AI Engine")
        .with_icon(load_embedded_icon())
        .build()
        .unwrap();

    // 4. 获取事件接收管道
    let menu_channel = MenuEvent::receiver();
    let tray_channel = TrayIconEvent::receiver();

    // 5. 启动事件循环，阻塞主线程
    event_loop.run(move |_event, _, control_flow| {
        // 设置为 Wait 模式，降低 CPU 占用
        *control_flow = ControlFlow::Wait;

        // ----------------------------------------------------
        // 【修复部分】处理托盘本身的事件（匹配鼠标左键松开）
        // ----------------------------------------------------
        if let Ok(event) = tray_channel.try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                // 左键单击托盘图标时，打开测试网页
                // let _ = webbrowser::open("https://github.com");
                let _ = webbrowser::open("http://127.0.0.1:3000");
            }
        }

        // 处理右键菜单的点击事件
        if let Ok(event) = menu_channel.try_recv() {
            if event.id == open_config_item.id() {
                // 点击“打开配置页面”
                // let _ = webbrowser::open("https://github.com");
                let _ = webbrowser::open("http://127.0.0.1:3000");
            } else if event.id == quit_item.id() {
                // 点击“退出”
                *control_flow = ControlFlow::Exit;
            }
        }
    });
}