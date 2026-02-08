// src/app.rs

use crate::models::Session;
use crate::store::SessionStore;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Sessions,
    Trash,
}

pub struct App {
    pub current_tab: Tab,
    pub sessions: Vec<Session>,
    pub trash: Vec<Session>,
    pub selected_session_idx: usize,
    #[allow(dead_code)]
    pub preview_scroll: u16,
    #[allow(dead_code)]
    pub search_query: String,
    #[allow(dead_code)]
    pub show_search: bool,
}

impl App {
    pub fn new() -> Self {
        let store = SessionStore::new();
        let sessions = store.load_sessions().unwrap_or_default();

        Self {
            current_tab: Tab::Sessions,
            sessions,
            trash: Vec::new(),
            selected_session_idx: 0,
            preview_scroll: 0,
            search_query: String::new(),
            show_search: false,
        }
    }

    #[allow(dead_code)]
    pub fn select_next(&mut self) {
        let list = match self.current_tab {
            Tab::Sessions => &self.sessions,
            Tab::Trash => &self.trash,
        };

        if !list.is_empty() && self.selected_session_idx < list.len() - 1 {
            self.selected_session_idx += 1;
        }
    }

    #[allow(dead_code)]
    pub fn select_prev(&mut self) {
        if self.selected_session_idx > 0 {
            self.selected_session_idx -= 1;
        }
    }

    #[allow(dead_code)]
    pub fn switch_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Sessions => Tab::Trash,
            Tab::Trash => Tab::Sessions,
        };
        self.selected_session_idx = 0;
    }

    #[allow(dead_code)]
    pub fn get_selected_session(&self) -> Option<&Session> {
        match self.current_tab {
            Tab::Sessions => self.sessions.get(self.selected_session_idx),
            Tab::Trash => self.trash.get(self.selected_session_idx),
        }
    }

    pub fn scroll_preview_down(&mut self) {
        self.preview_scroll = self.preview_scroll.saturating_add(3);
    }

    pub fn scroll_preview_up(&mut self) {
        self.preview_scroll = self.preview_scroll.saturating_sub(3);
    }

    pub fn filtered_sessions(&self) -> Vec<&Session> {
        let list = match self.current_tab {
            Tab::Sessions => &self.sessions,
            Tab::Trash => &self.trash,
        };

        if self.search_query.is_empty() {
            list.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            list.iter()
                .filter(|s| s.id.to_lowercase().contains(&q))
                .collect()
        }
    }

    pub fn toggle_search(&mut self) {
        self.show_search = !self.show_search;
        if !self.show_search {
            self.search_query.clear();
        }
    }

    pub fn add_search_char(&mut self, c: char) {
        self.search_query.push(c);
    }

    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = App::new();
        assert_eq!(app.current_tab, Tab::Sessions);
        assert_eq!(app.selected_session_idx, 0);
    }

    #[test]
    fn test_select_next() {
        let mut app = App::new();
        app.sessions = vec![
            Session::new("s1".to_string(), "/p1".to_string()),
            Session::new("s2".to_string(), "/p2".to_string()),
        ];

        app.select_next();
        assert_eq!(app.selected_session_idx, 1);

        app.select_next();
        assert_eq!(app.selected_session_idx, 1); // stays at end
    }

    #[test]
    fn test_switch_tab() {
        let mut app = App::new();
        assert_eq!(app.current_tab, Tab::Sessions);

        app.switch_tab();
        assert_eq!(app.current_tab, Tab::Trash);
    }

    #[test]
    fn test_search_filters_sessions() {
        let mut app = App::new();
        app.sessions = vec![
            Session::new("auto-service".to_string(), "/p1".to_string()),
            Session::new("dms-project".to_string(), "/p2".to_string()),
        ];

        app.search_query = "auto".to_string();
        let filtered = app.filtered_sessions();
        assert_eq!(filtered.len(), 1);
    }
}
