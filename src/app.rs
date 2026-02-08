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
    pub preview_scroll: u16,
    pub search_query: String,
    pub show_search: bool,
    pub status_message: Option<String>,
    pub confirm_delete: Option<String>, // Session ID pending deletion
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
            status_message: None,
            confirm_delete: None,
        }
    }

    #[cfg(test)]
    pub fn with_sessions(sessions: Vec<Session>) -> Self {
        Self {
            current_tab: Tab::Sessions,
            sessions,
            trash: Vec::new(),
            selected_session_idx: 0,
            preview_scroll: 0,
            search_query: String::new(),
            show_search: false,
            status_message: None,
            confirm_delete: None,
        }
    }

    pub fn select_next(&mut self) {
        let list = self.filtered_sessions();
        if !list.is_empty() && self.selected_session_idx < list.len() - 1 {
            self.selected_session_idx += 1;
            self.preview_scroll = 0;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_session_idx > 0 {
            self.selected_session_idx -= 1;
            self.preview_scroll = 0;
        }
    }

    pub fn switch_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Sessions => Tab::Trash,
            Tab::Trash => Tab::Sessions,
        };
        self.selected_session_idx = 0;
        self.preview_scroll = 0;
    }

    pub fn get_selected_session(&self) -> Option<&Session> {
        let filtered = self.filtered_sessions();
        filtered.get(self.selected_session_idx).copied()
    }

    pub fn current_list(&self) -> &Vec<Session> {
        match self.current_tab {
            Tab::Sessions => &self.sessions,
            Tab::Trash => &self.trash,
        }
    }

    pub fn scroll_preview_down(&mut self) {
        self.preview_scroll = self.preview_scroll.saturating_add(3);
    }

    pub fn scroll_preview_up(&mut self) {
        self.preview_scroll = self.preview_scroll.saturating_sub(3);
    }

    pub fn filtered_sessions(&self) -> Vec<&Session> {
        let list = self.current_list();

        if self.search_query.is_empty() {
            list.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            list.iter()
                .filter(|s| {
                    s.id.to_lowercase().contains(&q)
                        || s.project_name.to_lowercase().contains(&q)
                        || s.messages
                            .iter()
                            .any(|m| m.content.to_lowercase().contains(&q))
                })
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
        self.selected_session_idx = 0;
    }

    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
        self.selected_session_idx = 0;
    }

    pub fn move_selected_to_trash(&mut self) {
        if self.current_tab != Tab::Sessions {
            return;
        }
        let filtered = self.filtered_sessions();
        if let Some(session) = filtered.get(self.selected_session_idx) {
            let id = session.id.clone();
            if let Some(pos) = self.sessions.iter().position(|s| s.id == id) {
                let removed = self.sessions.remove(pos);
                self.trash.push(removed);
                self.status_message = Some(format!("Moved to trash: {}", id));
                if self.selected_session_idx > 0
                    && self.selected_session_idx >= self.sessions.len()
                {
                    self.selected_session_idx -= 1;
                }
            }
        }
    }

    pub fn restore_selected_from_trash(&mut self) {
        if self.current_tab != Tab::Trash {
            self.status_message = Some("Switch to Trash tab first".to_string());
            return;
        }
        let filtered = self.filtered_sessions();
        if let Some(session) = filtered.get(self.selected_session_idx) {
            let id = session.id.clone();
            if let Some(pos) = self.trash.iter().position(|s| s.id == id) {
                let removed = self.trash.remove(pos);
                self.sessions.push(removed);
                self.status_message = Some(format!("Restored: {}", id));
                if self.selected_session_idx > 0
                    && self.selected_session_idx >= self.trash.len()
                {
                    self.selected_session_idx -= 1;
                }
            }
        }
    }

    pub fn switch_to_selected_session(&mut self) {
        if let Some(session) = self.get_selected_session() {
            self.status_message = Some(format!(
                "Session: {} | claude --resume {}",
                session.project_name, session.id
            ));
        }
    }

    pub fn request_delete_confirmation(&mut self) {
        if let Some(session) = self.get_selected_session() {
            let session_id = session.id.clone();
            let project_name = session.project_name.clone();
            self.confirm_delete = Some(session_id);
            self.status_message = Some(format!(
                "Delete session '{}'? Press 'd' or 'y' to confirm, 'n' or Esc to cancel",
                project_name
            ));
        }
    }

    pub fn cancel_delete_confirmation(&mut self) {
        self.confirm_delete = None;
        self.status_message = Some("Delete cancelled".to_string());
    }

    pub fn is_delete_pending(&self) -> bool {
        self.confirm_delete.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Message;

    fn make_session(id: &str, project: &str) -> Session {
        Session {
            id: id.to_string(),
            project_path: format!("/home/g/{}", project),
            project_name: project.to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            size: 0,
            messages: vec![Message {
                role: "user".to_string(),
                content: format!("msg in {}", id),
            }],
        }
    }

    #[test]
    fn test_select_next_and_prev() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "proj1"),
            make_session("s2", "proj2"),
            make_session("s3", "proj3"),
        ]);

        assert_eq!(app.selected_session_idx, 0);
        app.select_next();
        assert_eq!(app.selected_session_idx, 1);
        app.select_next();
        assert_eq!(app.selected_session_idx, 2);
        app.select_next();
        assert_eq!(app.selected_session_idx, 2); // stays at end
        app.select_prev();
        assert_eq!(app.selected_session_idx, 1);
        app.select_prev();
        assert_eq!(app.selected_session_idx, 0);
        app.select_prev();
        assert_eq!(app.selected_session_idx, 0); // stays at start
    }

    #[test]
    fn test_switch_tab_resets_selection() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
        ]);
        app.selected_session_idx = 1;
        app.switch_tab();
        assert_eq!(app.current_tab, Tab::Trash);
        assert_eq!(app.selected_session_idx, 0);
        app.switch_tab();
        assert_eq!(app.current_tab, Tab::Sessions);
    }

    #[test]
    fn test_search_filters_by_id() {
        let mut app = App::with_sessions(vec![
            make_session("auto-service", "proj1"),
            make_session("dms-project", "proj2"),
        ]);
        app.search_query = "auto".to_string();
        let filtered = app.filtered_sessions();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "auto-service");
    }

    #[test]
    fn test_search_filters_by_project_name() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "auto-service"),
            make_session("s2", "dms-project"),
        ]);
        app.search_query = "dms".to_string();
        let filtered = app.filtered_sessions();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].project_name, "dms-project");
    }

    #[test]
    fn test_search_filters_by_message_content() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "proj1"),
            make_session("s2", "proj2"),
        ]);
        app.search_query = "msg in s2".to_string();
        let filtered = app.filtered_sessions();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "s2");
    }

    #[test]
    fn test_move_to_trash() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
        ]);
        app.selected_session_idx = 0;
        app.move_selected_to_trash();
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.trash.len(), 1);
        assert_eq!(app.trash[0].id, "s1");
    }

    #[test]
    fn test_restore_from_trash() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.move_selected_to_trash();
        assert_eq!(app.sessions.len(), 0);
        assert_eq!(app.trash.len(), 1);

        app.switch_tab(); // to Trash
        app.restore_selected_from_trash();
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.trash.len(), 0);
    }

    #[test]
    fn test_restore_only_works_in_trash_tab() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.restore_selected_from_trash();
        assert!(app.status_message.as_ref().unwrap().contains("Trash tab"));
    }

    #[test]
    fn test_scroll_preview() {
        let mut app = App::with_sessions(vec![]);
        app.scroll_preview_down();
        assert_eq!(app.preview_scroll, 3);
        app.scroll_preview_down();
        assert_eq!(app.preview_scroll, 6);
        app.scroll_preview_up();
        assert_eq!(app.preview_scroll, 3);
        app.scroll_preview_up();
        assert_eq!(app.preview_scroll, 0);
        app.scroll_preview_up(); // no underflow
        assert_eq!(app.preview_scroll, 0);
    }

    #[test]
    fn test_select_next_resets_scroll() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
        ]);
        app.preview_scroll = 10;
        app.select_next();
        assert_eq!(app.preview_scroll, 0);
    }

    #[test]
    fn test_switch_to_selected_session() {
        let mut app = App::with_sessions(vec![make_session("abc-123", "myproject")]);
        app.switch_to_selected_session();
        let msg = app.status_message.unwrap();
        assert!(msg.contains("claude --resume abc-123"));
        assert!(msg.contains("myproject"));
    }

    #[test]
    fn test_move_to_trash_adjusts_index() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
        ]);
        app.selected_session_idx = 1;
        app.move_selected_to_trash();
        assert_eq!(app.selected_session_idx, 0);
    }
}
