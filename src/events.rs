use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::app::{App, Focus, Mode};
use crate::config::SearchPosition;
use eyre::Result;
use tui_input::backend::crossterm::EventHandler;
use tui_input::InputRequest;

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    // Global keys
    match key.code {
        KeyCode::Esc => return Ok(true),
        KeyCode::Char('q') if app.focus != Focus::Search => return Ok(true),
        KeyCode::Char('m') if app.focus != Focus::Search => {
            app.toggle_mode();
            if app.config.focus_search_on_switch {
                app.focus = Focus::Search;
            }
            return Ok(false);
        }
        KeyCode::Char('t') if key.modifiers == KeyModifiers::CONTROL => {
            app.toggle_mode();
            if app.config.focus_search_on_switch {
                app.focus = Focus::Search;
            }
            return Ok(false);
        }
        _ => {}
    }

    // Search focus handling
    if app.focus == Focus::Search {
        // Navigation / Action keys that override typing
        match key.code {
            KeyCode::Enter => {
                if let Some(app_entry) = get_selected_app(app) {
                    app.app_to_launch = Some(app_entry.exec.clone());
                    app.should_quit = true;
                    return Ok(true);
                }
            }
            KeyCode::Tab => {
                 app.focus = match app.mode {
                    Mode::SinglePane => Focus::Apps,
                    Mode::DualPane => Focus::Categories,
                };
                return Ok(false);
            }
            KeyCode::Up | KeyCode::Char('k') if key.modifiers.is_empty() || key.modifiers == KeyModifiers::CONTROL => {
                // If Ctrl+K, it might be "Delete to end" for readline. 
                // But traditionally Up/Ctrl+K in this app was navigation.
                // User asked for readline style. Ctrl+K should be delete to end line.
                // Up is navigation.
                
                if key.code == KeyCode::Up {
                    // Allow Up from search to go to list only if search is at bottom
                    if app.config.search_position == SearchPosition::Bottom {
                        app.focus = match app.mode {
                            Mode::SinglePane => Focus::Apps,
                            Mode::DualPane => Focus::Apps, 
                        };
                        return Ok(false);
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') if key.modifiers.is_empty() => {
                 if key.code == KeyCode::Down {
                     // Allow Down from search to go to list only if search is at top
                    if app.config.search_position == SearchPosition::Top {
                        app.focus = match app.mode {
                            Mode::SinglePane => Focus::Apps,
                            Mode::DualPane => Focus::Categories,
                        };
                        return Ok(false);
                    }
                 }
                 // 'j' types 'j'.
            }
            _ => {}
        }

        // Emacs / Readline bindings & Typing
        // We match explicitly for control keys
        
        // Manual handling for missing InputRequest variants
        if key.modifiers == KeyModifiers::CONTROL {
            match key.code {
                 KeyCode::Char('u') => {
                     // Delete to start
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
                     // Delete to end
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
            // Default handling for other keys (typing, backspace, etc.)
            _ => None,
        };

        if let Some(req) = req {
            app.input.handle(req);
            app.reset_cursor_blink();
            update_selection_after_search(app);
        } else {
             // Pass to default handler
             app.input.handle_event(&Event::Key(key));
             app.reset_cursor_blink();
             update_selection_after_search(app);
        }
        
        return Ok(false);
    }

    // Navigation handling (when NOT in search)
    match key.code {
        KeyCode::Enter => {
            if let Some(app_entry) = get_selected_app(app) {
                app.app_to_launch = Some(app_entry.exec.clone());
                app.should_quit = true;
                return Ok(true);
            }
        }

        KeyCode::Tab => {
            app.focus = Focus::Search;
        }

        // Up/Down navigation
        KeyCode::Up | KeyCode::Char('k') => {
            // In list navigation
            match app.mode {
                Mode::SinglePane => {
                    match app.focus {
                        Focus::Apps => {
                            if app.selected_app > 0 {
                                app.selected_app -= 1;
                            } else if app.config.search_position == SearchPosition::Top {
                                app.focus = Focus::Search;
                            }
                        }
                        _ => {}
                    }
                }

                Mode::DualPane => {
                    match app.focus {
                        Focus::Apps => {
                            if app.selected_app > 0 {
                                app.selected_app -= 1;
                            } else if app.config.search_position == SearchPosition::Top {
                                app.focus = Focus::Search;
                            }
                        }
                        Focus::Categories => {
                            let matching_categories = get_matching_category_indices(app);
                            if let Some(current_pos) = matching_categories.iter().position(|&idx| idx == app.selected_category) {
                                if current_pos > 0 {
                                    app.selected_category = matching_categories[current_pos - 1];
                                    app.selected_app = 0;
                                } else if app.config.search_position == SearchPosition::Top {
                                    app.focus = Focus::Search;
                                }
                            } else if app.config.search_position == SearchPosition::Top {
                                app.focus = Focus::Search;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        KeyCode::Down | KeyCode::Char('j') => {
            // In list navigation
            match app.mode {
                Mode::SinglePane => {
                    match app.focus {
                        Focus::Apps => {
                            let count = count_filtered_apps_in_current_category(app);
                            if count > 0 && app.selected_app + 1 < count {
                                app.selected_app += 1;
                            } else if app.config.search_position == SearchPosition::Bottom {
                                app.focus = Focus::Search;
                            }
                        }
                        _ => {}
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
                                } else if app.config.search_position == SearchPosition::Bottom {
                                    app.focus = Focus::Search;
                                }
                            } else if app.config.search_position == SearchPosition::Bottom {
                                app.focus = Focus::Search;
                            }
                        }
                        Focus::Apps => {
                            let count = count_filtered_apps_in_current_category(app);
                            if count > 0 && app.selected_app + 1 < count {
                                app.selected_app += 1;
                            } else if app.config.search_position == SearchPosition::Bottom {
                                app.focus = Focus::Search;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // h/l keys only work for list navigation when NOT in search
        KeyCode::Char('h') => {
            match app.focus {
                Focus::Apps => {
                    if app.mode == Mode::DualPane {
                        app.focus = Focus::Categories;
                    } else {
                        if app.selected_app > 0 {
                            app.selected_app -= 1;
                        }
                    }
                }
                Focus::Categories => {
                    let matching_categories = get_matching_category_indices(app);
                    if let Some(current_pos) = matching_categories.iter().position(|&idx| idx == app.selected_category) {
                        if current_pos > 0 {
                            app.selected_category = matching_categories[current_pos - 1];
                            app.selected_app = 0;
                        }
                    }
                }
                _ => {}
            }
        }

        KeyCode::Char('l') => {
            match app.focus {
                Focus::Categories => {
                    if app.mode == Mode::DualPane {
                        app.focus = Focus::Apps;
                    } else {
                        let matching_categories = get_matching_category_indices(app);
                        if let Some(current_pos) = matching_categories.iter().position(|&idx| idx == app.selected_category) {
                            if current_pos + 1 < matching_categories.len() {
                                app.selected_category = matching_categories[current_pos + 1];
                                app.selected_app = 0;
                            }
                        }
                    }
                }
                Focus::Apps => {
                    let count = count_filtered_apps_in_current_category(app);
                    if count > 0 && app.selected_app + 1 < count {
                        app.selected_app += 1;
                    }
                }
                _ => {}
            }
        }

        _ => {}
    }

    Ok(false)
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
