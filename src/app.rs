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
}
