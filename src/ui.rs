use anyhow::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, PlaybackState};

pub fn render(frame: &mut Frame, app: &App) -> Result<()> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // Now playing
                Constraint::Length(3),  // Progress bar
                Constraint::Length(3),  // Controls
                Constraint::Min(0),     // Playlist
            ]
            .as_ref(),
        )
        .split(frame.size());

    render_title(frame, app, chunks[0])?;
    render_now_playing(frame, app, chunks[1])?;
    render_progress(frame, app, chunks[2])?;
    render_controls(frame, app, chunks[3])?;
    render_playlist(frame, app, chunks[4])?;

    Ok(())
}

fn render_title(frame: &mut Frame, _app: &App, area: Rect) -> Result<()> {
    let title = Paragraph::new("🎵 Cuper Music TUI  🎵")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("Cuper Music Player"));
    
    frame.render_widget(title, area);
    Ok(())
}

fn render_now_playing(frame: &mut Frame, app: &App, area: Rect) -> Result<()> {
    let current_song = app.get_current_song();
    let song_name = current_song
        .map(|song| song.name.clone())
        .unwrap_or_else(|| "没有歌曲".to_string());

    let status = match app.playback_state {
        PlaybackState::Playing => "▶️ 播放中",
        PlaybackState::Paused => "⏸️ 暂停",
        PlaybackState::Stopped => "⏹️ 停止",
    };

    let text = vec![
        Line::from(vec![
            Span::styled("当前播放: ", Style::default().fg(Color::Yellow)),
            Span::styled(song_name, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("状态: ", Style::default().fg(Color::Yellow)),
            Span::styled(status, Style::default().fg(Color::Green)),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("播放状态"));
    
    frame.render_widget(paragraph, area);
    Ok(())
}

fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

fn render_progress(frame: &mut Frame, app: &App, area: Rect) -> Result<()> {
    let current_time = app.get_current_time();
    let total_duration = app.get_total_duration();
    let progress = app.get_progress();
    let volume_percentage = (app.volume * 100.0) as u16;
    
    // 根据播放状态调整进度条颜色
    let progress_color = match app.playback_state {
        PlaybackState::Playing => Color::Blue,
        PlaybackState::Paused => Color::Yellow,
        PlaybackState::Stopped => Color::Gray,
    };
    
    let time_label = format!("{} / {}", 
        format_duration(current_time), 
        format_duration(total_duration)
    );
    
    let progress_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("播放进度"))
        .gauge_style(Style::default().fg(progress_color))
        .ratio(progress as f64)
        .label(time_label);

    let volume_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("音量"))
        .gauge_style(Style::default().fg(Color::Red))
        .ratio(app.volume as f64)
        .label(format!("{}%", volume_percentage));

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
        .split(area);

    frame.render_widget(progress_gauge, chunks[0]);
    frame.render_widget(volume_gauge, chunks[1]);
    
    Ok(())
}

fn render_controls(frame: &mut Frame, app: &App, area: Rect) -> Result<()> {
    let shuffle_status = if app.is_shuffle { "🔀 随机播放开启" } else { "🔀 随机播放关闭" };
    
    let controls_text = vec![
        Line::from(vec![
            Span::styled("空格键: ", Style::default().fg(Color::Yellow)),
            Span::styled("播放/暂停", Style::default().fg(Color::White)),
            Span::styled("  N: ", Style::default().fg(Color::Yellow)),
            Span::styled("下一曲", Style::default().fg(Color::White)),
            Span::styled("  P: ", Style::default().fg(Color::Yellow)),
            Span::styled("上一曲", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("S: ", Style::default().fg(Color::Yellow)),
            Span::styled("切换随机播放", Style::default().fg(Color::White)),
            Span::styled("  +/-: ", Style::default().fg(Color::Yellow)),
            Span::styled("调节音量", Style::default().fg(Color::White)),
            Span::styled("  Q: ", Style::default().fg(Color::Yellow)),
            Span::styled("退出", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(shuffle_status, Style::default().fg(Color::Magenta)),
        ]),
        Line::from(vec![
            Span::styled("自动播放下一曲已启用", Style::default().fg(Color::Green)),
        ]),
    ];

    let paragraph = Paragraph::new(controls_text)
        .block(Block::default().borders(Borders::ALL).title("控制说明"));
    
    frame.render_widget(paragraph, area);
    Ok(())
}

fn render_playlist(frame: &mut Frame, app: &App, area: Rect) -> Result<()> {
    let items: Vec<ListItem> = app
        .songs
        .iter()
        .enumerate()
        .map(|(index, song)| {
            let style = if index == app.current_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            
            ListItem::new(vec![Line::from(vec![
                Span::styled(format!("{:2}. ", index + 1), style),
                Span::styled(song.name.clone(), style),
            ])])
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("播放列表"))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    frame.render_widget(list, area);
    Ok(())
} 