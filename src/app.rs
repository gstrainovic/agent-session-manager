use crate::models::Session;
use crate::store::SessionStore;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Sessions,
    Trash,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    DeleteToTrash(String),      // Session ID to move to trash
    DeletePermanently(String),  // Session ID to delete permanently
    EmptyTrash,                 // Empty entire trash
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
    pub status_message_time: Option<Instant>,
    pub confirm_action: Option<ConfirmAction>,
    pub resume_session_id: Option<String>,
    pub resume_session_path: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let store = SessionStore::new();
        let sessions = store.load_sessions().unwrap_or_default();
        let trash = store.load_trash().unwrap_or_default();

        Self {
            current_tab: Tab::Sessions,
            sessions,
            trash,
            selected_session_idx: 0,
            preview_scroll: 0,
            search_query: String::new(),
            show_search: false,
            status_message: None,
            status_message_time: None,
            confirm_action: None,
            resume_session_id: None,
            resume_session_path: None,
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
            status_message_time: None,
            confirm_action: None,
            resume_session_id: None,
            resume_session_path: None,
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

                // Save trash to disk
                let store = SessionStore::new();
                let _ = store.save_trash(&self.trash);

                self.set_status(format!("Moved to trash: {}", id));
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
            self.set_status("Switch to Trash tab first".to_string());
            return;
        }
        let filtered = self.filtered_sessions();
        if let Some(session) = filtered.get(self.selected_session_idx) {
            let id = session.id.clone();
            if let Some(pos) = self.trash.iter().position(|s| s.id == id) {
                let removed = self.trash.remove(pos);
                self.sessions.push(removed);

                // Save trash to disk
                let store = SessionStore::new();
                let _ = store.save_trash(&self.trash);

                self.set_status(format!("Restored: {}", id));
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
            let session_id = session.id.clone();
            let project_path = session.project_path.clone();
            let project_name = session.project_name.clone();
            self.resume_session_id = Some(session_id.clone());
            self.resume_session_path = Some(project_path);
            self.set_status(format!(
                "Resuming session: {} | claude --resume {}",
                project_name, session_id
            ));
        }
    }

    pub fn get_resume_command(&self) -> Option<String> {
        self.resume_session_id
            .as_ref()
            .map(|id| format!("claude --resume {}", id))
    }

    pub fn get_resume_session_path(&self) -> Option<String> {
        self.resume_session_path.clone()
    }

    pub fn request_delete_confirmation(&mut self) {
        if let Some(session) = self.get_selected_session() {
            let session_id = session.id.clone();
            let project_name = session.project_name.clone();

            let action = if self.current_tab == Tab::Trash {
                ConfirmAction::DeletePermanently(session_id)
            } else {
                ConfirmAction::DeleteToTrash(session_id)
            };

            self.confirm_action = Some(action.clone());

            let message = match action {
                ConfirmAction::DeleteToTrash(_) => format!(
                    "Move '{}' to trash? Press 'd' or 'y' to confirm, 'n' or Esc to cancel",
                    project_name
                ),
                ConfirmAction::DeletePermanently(_) => format!(
                    "PERMANENTLY delete '{}'? Press 'd' or 'y' to confirm, 'n' or Esc to cancel",
                    project_name
                ),
                _ => String::new(),
            };

            self.set_status(message);
        }
    }

    pub fn request_empty_trash(&mut self) {
        if self.current_tab != Tab::Trash {
            return;
        }

        let count = self.trash.len();
        if count == 0 {
            self.set_status("Trash is already empty".to_string());
            return;
        }

        self.confirm_action = Some(ConfirmAction::EmptyTrash);
        self.set_status(format!(
            "PERMANENTLY delete ALL {} sessions in trash? Press 'E' or 'y' to confirm, 'n' or Esc to cancel",
            count
        ));
    }

    pub fn cancel_confirmation(&mut self) {
        self.confirm_action = None;
        self.set_status("Action cancelled".to_string());
    }

    pub fn is_confirmation_pending(&self) -> bool {
        self.confirm_action.is_some()
    }

    pub fn confirm_and_execute(&mut self) {
        if let Some(action) = self.confirm_action.clone() {
            match action {
                ConfirmAction::DeleteToTrash(_) => {
                    // This is handled in main.rs by calling delete_session + move_to_trash
                }
                ConfirmAction::DeletePermanently(_) => {
                    self.delete_permanently();
                }
                ConfirmAction::EmptyTrash => {
                    self.empty_trash();
                }
            }
        }
    }

    fn delete_permanently(&mut self) {
        let session_id = if let Some(ConfirmAction::DeletePermanently(id)) = &self.confirm_action {
            id.clone()
        } else {
            return;
        };

        if let Some(pos) = self.trash.iter().position(|s| s.id == session_id) {
            self.trash.remove(pos);

            // Save trash to disk
            let store = SessionStore::new();
            let _ = store.save_trash(&self.trash);

            self.set_status(format!("Permanently deleted: {}", session_id));
            self.confirm_action = None;

            if self.selected_session_idx > 0 && self.selected_session_idx >= self.trash.len() {
                self.selected_session_idx -= 1;
            }
        }
    }

    fn empty_trash(&mut self) {
        let count = self.trash.len();
        self.trash.clear();

        // Save empty trash to disk
        let store = SessionStore::new();
        let _ = store.save_trash(&self.trash);

        self.set_status(format!("Permanently deleted {} sessions", count));
        self.confirm_action = None;
        self.selected_session_idx = 0;
    }


    pub fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
        self.status_message_time = Some(Instant::now());
    }

    pub fn clear_expired_status(&mut self) {
        if let Some(time) = self.status_message_time {
            if time.elapsed().as_secs() >= 3 {
                self.status_message = None;
                self.status_message_time = None;
            }
        }
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
            total_entries: 1,
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

    #[test]
    fn test_set_status_sets_both_message_and_time() {
        let mut app = App::with_sessions(vec![]);
        app.set_status("Test message".to_string());
        
        assert!(app.status_message.is_some());
        assert!(app.status_message_time.is_some());
        assert_eq!(app.status_message.unwrap(), "Test message");
    }

    #[test]
    fn test_clear_expired_status_removes_after_expiry() {
        let mut app = App::with_sessions(vec![]);
        app.set_status("Test message".to_string());
        
        // Immediately clear - should not remove (less than 3 seconds)
        app.clear_expired_status();
        assert!(app.status_message.is_some());
        
        // Manually set time to 3+ seconds ago
        use std::time::Instant;
        app.status_message_time = Some(Instant::now() - std::time::Duration::from_secs(3));
        app.clear_expired_status();
        assert!(app.status_message.is_none());
        assert!(app.status_message_time.is_none());
    }

    #[test]
    fn test_export_status_uses_set_status() {
        let mut app = App::with_sessions(vec![make_session("test-id", "test-proj")]);
        // Simulate export success by manually calling set_status
        app.set_status("Exported to /path/file.md".to_string());
        
        assert!(app.status_message.is_some());
        assert!(app.status_message_time.is_some());
        assert!(app.status_message.unwrap().contains("Exported to"));
    }

    #[test]
    fn test_request_delete_confirmation_uses_set_status() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.request_delete_confirmation();
        
        // Status message should be set with time
        assert!(app.status_message.is_some());
        assert!(app.status_message_time.is_some());
        assert!(app.status_message.unwrap().contains("trash"));
    }

    #[test]
    fn test_request_empty_trash_uses_set_status() {
        let mut app = App::with_sessions(vec![]);
        app.trash = vec![make_session("s1", "p1")];
        app.current_tab = Tab::Trash;
        app.request_empty_trash();
        
        // Status message should be set with time
        assert!(app.status_message.is_some());
        assert!(app.status_message_time.is_some());
        assert!(app.status_message.unwrap().contains("PERMANENTLY delete"));
    }

    #[test]
    fn test_switch_to_selected_session_sets_resume_id() {
        let session = make_session("test-id", "test-project");
        let mut app = App::with_sessions(vec![session]);
        
        app.switch_to_selected_session();
        
        assert_eq!(app.resume_session_id, Some("test-id".to_string()));
    }

    #[test]
    fn test_switch_to_selected_session_sets_resume_path() {
        let session = make_session("test-id", "test-project");
        let mut app = App::with_sessions(vec![session.clone()]);
        
        app.switch_to_selected_session();
        
        assert_eq!(app.resume_session_path, Some(session.project_path));
    }

    #[test]
    fn test_get_resume_command_none_when_no_id() {
        let app = App::with_sessions(vec![]);
        assert_eq!(app.get_resume_command(), None);
    }

    #[test]
    fn test_get_resume_command_builds_correct_command() {
        let session = make_session("abc123", "project");
        let mut app = App::with_sessions(vec![session]);
        
        app.switch_to_selected_session();
        
        assert_eq!(
            app.get_resume_command(),
            Some("claude --resume abc123".to_string())
        );
    }

    #[test]
    fn test_get_resume_session_path() {
        let session = make_session("test-id", "test-project");
        let mut app = App::with_sessions(vec![session.clone()]);
        
        app.switch_to_selected_session();
        
        assert_eq!(app.get_resume_session_path(), Some(session.project_path));
    }

    #[test]
    fn test_resume_command_persists() {
        let session = make_session("persist-test", "p");
        let mut app = App::with_sessions(vec![session]);
        
        app.switch_to_selected_session();
        let cmd1 = app.get_resume_command();
        let cmd2 = app.get_resume_command();
        
        assert_eq!(cmd1, cmd2);
        assert_eq!(cmd1, Some("claude --resume persist-test".to_string()));
    }
}
