mod app;
mod event;
mod ui;

use anyhow::Result;
use std::time::Duration;

use app::App;
use event::{EventHandler, handle_events, restore_terminal, setup_terminal};

fn main() -> Result<()> {
    // 创建应用程序
    let mut app = App::new()?;
    
    // 设置终端
    let mut terminal = setup_terminal()?;
    
    // 创建事件处理器
    let mut event_handler = EventHandler::new(Duration::from_millis(100));
    
    // 主循环
    loop {
        // 检查播放状态，自动播放下一曲
        app.check_and_auto_next()?;
        
        // 更新播放时间
        app.update_play_time();
        
        // 渲染界面
        terminal.draw(|frame| {
            ui::render(frame, &app).unwrap();
        })?;
        
        // 处理事件
        if let Some(event) = event_handler.next()? {
            if let Err(_) = handle_events(&mut app, event) {
                break;
            }
        }
    }
    
    // 恢复终端
    restore_terminal(&mut terminal)?;
    
    Ok(())
}
