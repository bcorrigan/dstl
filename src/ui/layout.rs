use ratatui::{
    Frame,
    layout::{Layout, Constraint, Direction, Rect},
    widgets::{Block, Borders, List, ListItem, ListState},
    style::{Style, Color},
};
use tui_textarea::TextArea;
use crate::app::Focus;
use crate::config::{DstlConfig, LauncherTheme, SearchPosition};

pub fn vertical_split(f: &Frame, search_height: u16, search_position: SearchPosition) -> (Rect, Rect) {
    let full_area = f.area();
    match search_position {
        SearchPosition::Top => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(search_height), Constraint::Min(0)])
                .split(full_area);
            (chunks[0], chunks[1])
        }
        SearchPosition::Bottom => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(search_height)])
                .split(full_area);
            (chunks[1], chunks[0])
        }
    }
}

pub fn horizontal_split(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Percentage(70)])
        .split(area);
    (chunks[0], chunks[1])
}

pub fn render_search_bar(
    f: &mut Frame,
    area: Rect,
    text_area: &mut TextArea<'static>,
    focus: Focus,
    config: &DstlConfig,
) {
    let border_color = if focus == Focus::Search {
        LauncherTheme::parse_color(&config.colors.focus)
    } else {
        LauncherTheme::parse_color(&config.colors.border)
    };

    let block = Block::default()
        .title(" Search ")
        .borders(Borders::ALL)
        .border_type(LauncherTheme::parse_border_type(&config.colors.border_style))
        .border_style(Style::default().fg(border_color));

    text_area.set_block(block);
    text_area.set_style(Style::default().fg(border_color));
    
    // We can rely on tui-textarea's internal cursor rendering.
    // However, if we want to sync the hardware cursor, we can do it here or in main loop.
    // Given main loop does manual cursor handling, let's try to just render the widget here.
    // The main loop's manual cursor code relies on calculated positions which are now gone.
    // So we should set the cursor here if we want hardware cursor.
    // But wait, tui-textarea doesn't expose screen coordinates easily unless we calculate them.
    // Actually, we can just let tui-textarea render its own cursor (visual block/line).
    
    // Check if we need to hide the cursor (blinking handled by App)
    // We don't have access to app.cursor_visible here directly, but the main loop
    // calls this. We should probably pass cursor_visible or handle it outside.
    // For now, let's assume the text_area has the correct cursor style set.

    f.render_widget(&*text_area, area);
}



pub fn render_list(
    f: &mut Frame,
    area: Rect,
    title: &str,
    items: &[String],
    selected: usize,
    focus_on_title: bool,
    config: &DstlConfig,
) {
    let mut state = ListState::default();
    let sel = if selected >= items.len() { 0 } else { selected };
    state.select(Some(sel));
    
    let border_color = if focus_on_title {
        LauncherTheme::parse_color(&config.colors.focus)
    } else {
        LauncherTheme::parse_color(&config.colors.border)
    };
    
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(LauncherTheme::parse_border_type(&config.colors.border_style))
        .border_style(Style::default().fg(border_color));
    
    let list_items: Vec<ListItem> = items.iter()
        .map(|a| ListItem::new(format!(" {} ", a)))
        .collect();
    
    let highlight_color = LauncherTheme::parse_color(&config.colors.highlight);
    let highlight_style = match config.colors.highlight_type.to_lowercase().as_str() {
        "foreground" => Style::default().fg(highlight_color),
        "background" | _ => Style::default().bg(highlight_color).fg(Color::Black),
    };
    
    let list = List::new(list_items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("");
    
    f.render_stateful_widget(list, area, &mut state);
}
