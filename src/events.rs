use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::app::{App, Focus, Mode};
use eyre::Result;
use tui_input::backend::crossterm::EventHandler;
use tui_input::InputRequest;

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    // 1. Global / Exit keys
    match key.code {
        KeyCode::Esc => return Ok(true),
        KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => return Ok(true),
        KeyCode::Char('g') if key.modifiers == KeyModifiers::CONTROL => return Ok(true),
        
        // Toggle mode (Global)
        KeyCode::Char('m') if key.modifiers == KeyModifiers::CONTROL => {
            app.toggle_mode();
            return Ok(false);
        }
        KeyCode::Char('t') if key.modifiers == KeyModifiers::CONTROL => {
            app.toggle_mode();
            return Ok(false);
        }
        KeyCode::Tab => {
            app.toggle_mode();
            return Ok(false);
        }
        
        // Launch
        KeyCode::Enter => {
            if let Some(app_entry) = get_selected_app(app) {
                app.app_to_launch = Some(app_entry.exec.clone());
                app.should_quit = true;
                return Ok(true);
            }
        }
        _ => {}
    }

    // 2. Navigation (Arrows) - Independent of input focus
    match key.code {
        KeyCode::Up => {
            navigate_up(app);
            return Ok(false);
        }
        KeyCode::Down => {
            navigate_down(app);
            return Ok(false);
        }
        KeyCode::Left => {
            navigate_left(app);
            return Ok(false);
        }
        KeyCode::Right => {
            navigate_right(app);
            return Ok(false);
        }
        _ => {}
    }

    // 3. Input Handling - Everything else goes to search
    // Helper for Emacs bindings
    let req = match key {
        KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::CONTROL, .. } => Some(InputRequest::GoToStart),
        KeyEvent { code: KeyCode::Char('e'), modifiers: KeyModifiers::CONTROL, .. } => Some(InputRequest::GoToEnd),
        KeyEvent { code: KeyCode::Char('b'), modifiers: KeyModifiers::CONTROL, .. } => Some(InputRequest::GoToPrevChar),
        KeyEvent { code: KeyCode::Char('f'), modifiers: KeyModifiers::CONTROL, .. } => Some(InputRequest::GoToNextChar),
        KeyEvent { code: KeyCode::Char('w'), modifiers: KeyModifiers::CONTROL, .. } => Some(InputRequest::DeletePrevWord),
        // Ctrl+D -> DeleteNextChar (Delete)
        KeyEvent { code: KeyCode::Char('d'), modifiers: KeyModifiers::CONTROL, .. } => Some(InputRequest::DeleteNextChar),
        // Ctrl+H -> DeletePrevChar (Backspace)
        KeyEvent { code: KeyCode::Char('h'), modifiers: KeyModifiers::CONTROL, .. } => Some(InputRequest::DeletePrevChar),
        _ => None,
    };

    // Manual handling for missing InputRequest variants (Ctrl+U, Ctrl+K)
    if key.modifiers == KeyModifiers::CONTROL {
        match key.code {
                KeyCode::Char('u') => {
                    let cursor = app.input.cursor();
                    let val = app.input.value();
                    if cursor > 0 && cursor <= val.len() {
                        let suffix = &val[cursor..];
                        let mut new_input = tui_input::Input::new(suffix.to_string());
                        new_input.handle(InputRequest::GoToStart);
                        app.input = new_input;
                        update_selection_after_search(app);
                    }
                    return Ok(false);
                }
                KeyCode::Char('k') => {
                    let cursor = app.input.cursor();
                    let val = app.input.value();
                    if cursor < val.len() {
                        let prefix = &val[..cursor];
                        app.input = tui_input::Input::new(prefix.to_string());
                        update_selection_after_search(app);
                    }
                    return Ok(false);
                }
                _ => {}
        }
    }

    if let Some(req) = req {
        app.input.handle(req);
        app.reset_cursor_blink();
        update_selection_after_search(app);
    } else {
        // Only pass to input if it's not a reserved key we missed or modifier
        // tui_input handles most things well.
        app.input.handle_event(&Event::Key(key));
        app.reset_cursor_blink();
        update_selection_after_search(app);
    }

    Ok(false)
}

fn navigate_up(app: &mut App) {
    match app.mode {
        Mode::SinglePane => {
            if app.selected_app > 0 {
                app.selected_app -= 1;
            }
        }
        Mode::DualPane => {
            match app.focus {
                Focus::Categories => {
                    let matching_categories = get_matching_category_indices(app);
                    if let Some(current_pos) = matching_categories.iter().position(|&idx| idx == app.selected_category) {
                        if current_pos > 0 {
                            app.selected_category = matching_categories[current_pos - 1];
                            app.selected_app = 0;
                        }
                    }
                }
                _ => { // Focus::Apps or Search (effectively Apps)
                    if app.selected_app > 0 {
                        app.selected_app -= 1;
                    }
                }
            }
        }
    }
}

fn navigate_down(app: &mut App) {
    match app.mode {
        Mode::SinglePane => {
            let count = count_filtered_apps_in_current_category(app);
            if count > 0 && app.selected_app + 1 < count {
                app.selected_app += 1;
            }
        }
        Mode::DualPane => {
            match app.focus {
                Focus::Categories => {
                    let matching_categories = get_matching_category_indices(app);
                    if let Some(current_pos) = matching_categories.iter().position(|&idx| idx == app.selected_category) {
                        if current_pos + 1 < matching_categories.len() {
                            app.selected_category = matching_categories[current_pos + 1];
                            app.selected_app = 0;
                        }
                    }
                }
                _ => { // Focus::Apps
                    let count = count_filtered_apps_in_current_category(app);
                    if count > 0 && app.selected_app + 1 < count {
                        app.selected_app += 1;
                    }
                }
            }
        }
    }
}

fn navigate_left(app: &mut App) {
    if app.mode == Mode::DualPane {
        // If focusing apps, go to categories
        if app.focus == Focus::Apps {
             app.focus = Focus::Categories;
        }
    }
}

fn navigate_right(app: &mut App) {
    if app.mode == Mode::DualPane {
        // If focusing categories, go to apps
        if app.focus == Focus::Categories {
             app.focus = Focus::Apps;
        }
    }
}

fn get_matching_category_indices(app: &App) -> Vec<usize> {
    let query = app.query();
    if query.is_empty() {
        (0..app.categories.len()).collect()
    } else {
        let query_lower = query.to_lowercase();
        app.categories
            .iter()
            .enumerate()
            .filter(|(_, cat_name)| {
                if *cat_name == "Recent" {
                    app.recent_apps.iter().any(|recent_name| {
                        app.apps.iter()
                            .find(|a| &a.name == recent_name)
                            .and_then(|a| app.matches_search(&a.name, &query_lower))
                            .is_some()
                    })
                } else {
                    app.apps.iter().any(|a| {
                        &a.category == *cat_name && app.matches_search(&a.name, &query_lower).is_some()
                    })
                }
            })
            .map(|(idx, _)| idx)
            .collect()
    }
}

fn update_selection_after_search(app: &mut App) {
    if app.query().is_empty() {
        app.selected_category = 0;
        app.selected_app = 0;
        return;
    }

    match app.mode {
        Mode::DualPane => {
            let matching_indices = get_matching_category_indices(app);
            if let Some(&first_match) = matching_indices.first() {
                app.selected_category = first_match;
                app.selected_app = 0;
            }
        }
        Mode::SinglePane => { app.selected_app = 0; }
    }
}

fn get_selected_app(app: &App) -> Option<&crate::app::AppEntry> {
    match app.mode {
        Mode::SinglePane => {
            app.visible_apps().get(app.selected_app).map(|v| &**v)
        }
        Mode::DualPane => {
            let cat_name = app.categories.get(app.selected_category)?;
            let query = app.query();
            
            if cat_name == "Recent" {
                let apps_in_order: Vec<&crate::app::AppEntry> = app.recent_apps.iter()
                    .filter_map(|recent_name| {
                        app.apps.iter().find(|a| &a.name == recent_name)
                    })
                    .collect();
                
                if !query.is_empty() {
                    let mut apps_with_scores: Vec<(&crate::app::AppEntry, i64)> = apps_in_order
                        .into_iter()
                        .filter_map(|a| app.matches_search(&a.name, &query).map(|score| (a, score)))
                        .collect();
                    apps_with_scores.sort_by(|a, b| b.1.cmp(&a.1));
                    return apps_with_scores.get(app.selected_app).map(|(entry, _)| *entry);
                }
                
                apps_in_order.get(app.selected_app).copied()
            } else {
                let mut apps_with_scores: Vec<(&crate::app::AppEntry, i64)> = app.apps.iter()
                    .filter(|a| &a.category == cat_name)
                    .filter_map(|a| app.matches_search(&a.name, &query).map(|score| (a, score)))
                    .collect();

                if !query.is_empty() {
                    apps_with_scores.sort_by(|a, b| b.1.cmp(&a.1));
                }

                apps_with_scores.get(app.selected_app).map(|(entry, _)| *entry)
            }
        }
    }
}

fn count_filtered_apps_in_current_category(app: &App) -> usize {
    match app.mode {
        Mode::SinglePane => {
            app.visible_apps().len()
        }
        Mode::DualPane => {
            let cat_name = match app.categories.get(app.selected_category) {
                Some(c) => c,
                None => return 0,
            };
            let query = app.query();
            
            if cat_name == "Recent" {
                app.recent_apps.iter()
                    .filter_map(|recent_name| {
                        app.apps.iter().find(|a| &a.name == recent_name)
                    })
                    .filter(|a| app.matches_search(&a.name, &query).is_some())
                    .count()
            } else {
                app.apps.iter()
                    .filter(|a| &a.category == cat_name)
                    .filter(|a| app.matches_search(&a.name, &query).is_some())
                    .count()
            }
        }
    }
}
