use crate::config::AppConfig;
use crate::models::Session;
use crate::store::SessionStore;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::TableState;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Sessions,
    Trash,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusPanel {
    List,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortField {
    Project,
    Name,
    Messages,
    Date,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    DeleteToTrash(String),     // Session ID to move to trash
    DeletePermanently(String), // Session ID to delete permanently
    EmptyTrash,                // Empty entire trash
    TrashZeroMessages,         // Move all 0-message sessions to trash
}

/// Aktion, die durch einen Mausklick auf eine registrierte Region ausgelöst wird.
#[derive(Debug, Clone, PartialEq)]
pub enum ClickAction {
    SwitchTab(Tab),
    ResumeSession,
    DeleteSession,
    ExportSession,
    CleanZeroMessages,
    ToggleSearch,
    ToggleSort,
    OpenSettings,
    ToggleHelp,
    Quit,
    RestoreFromTrash,
    EmptyTrash,
    RenameSession,
    SaveSettings,
    CancelSettings,
    ConfirmYes,
    ConfirmNo,
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
    pub focus: FocusPanel,
    pub sort_field: SortField,
    pub sort_direction: SortDirection,
    pub show_help: bool,
    pub help_scroll: u16,
    pub show_settings: bool,
    pub settings_input: String,
    pub show_rename: bool,
    pub rename_input: String,
    pub config: AppConfig,
    pub list_table_state: TableState,
    pub terminal_size: (u16, u16),
    /// Klickbare Regionen, die bei jedem Frame neu berechnet werden.
    pub click_regions: Vec<(Rect, ClickAction)>,
}

impl App {
    pub fn new(sessions: Vec<Session>, trash: Vec<Session>) -> Self {
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
            focus: FocusPanel::List,
            sort_field: SortField::Date,
            sort_direction: SortDirection::Descending,
            show_help: false,
            help_scroll: 0,
            show_settings: false,
            settings_input: String::new(),
            show_rename: false,
            rename_input: String::new(),
            config: AppConfig::load(),
            list_table_state: TableState::default(),
            terminal_size: (0, 0),
            click_regions: Vec::new(),
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
            focus: FocusPanel::List,
            sort_field: SortField::Date,
            sort_direction: SortDirection::Descending,
            show_help: false,
            help_scroll: 0,
            show_settings: false,
            settings_input: String::new(),
            show_rename: false,
            rename_input: String::new(),
            config: AppConfig::default(),
            list_table_state: TableState::default(),
            terminal_size: (0, 0),
            click_regions: Vec::new(),
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

    pub fn switch_to_tab(&mut self, tab: Tab) {
        self.current_tab = tab;
        self.selected_session_idx = 0;
        self.preview_scroll = 0;
    }

    /// Prüft ob ein Klick eine registrierte Region trifft und gibt die Aktion zurück.
    pub fn get_click_action(&self, col: u16, row: u16) -> Option<ClickAction> {
        let pos = Position { x: col, y: row };
        for (rect, action) in &self.click_regions {
            if rect.contains(pos) {
                return Some(action.clone());
            }
        }
        None
    }

    /// Behandelt einen Mausklick auf die Session-Liste oder das Preview-Panel.
    /// Tab-Bar und Command-Bar werden über click_regions in get_click_action abgehandelt.
    pub fn handle_list_click(&mut self, col: u16, row: u16) {
        let (width, height) = self.terminal_size;
        if width == 0 || height == 0 {
            return;
        }
        // Tab-Bar und Command-Bar überspringen
        if row < 3 || row >= height.saturating_sub(3) {
            return;
        }
        let list_width = width * 30 / 100;
        if col < list_width {
            // Zeile 3=Border, 4=Header, 5+=Session-Einträge
            if row >= 5 {
                let clicked = self.list_table_state.offset() + (row - 5) as usize;
                let len = self.filtered_sessions().len();
                if clicked < len {
                    self.selected_session_idx = clicked;
                    self.preview_scroll = 0;
                    self.focus = FocusPanel::List;
                }
            }
        } else {
            self.focus = FocusPanel::Preview;
        }
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

    pub fn focus_left(&mut self) {
        self.focus = FocusPanel::List;
    }

    pub fn focus_right(&mut self) {
        self.focus = FocusPanel::Preview;
    }

    pub fn page_down(&mut self, page_size: usize) {
        match self.focus {
            FocusPanel::List => {
                let list = self.filtered_sessions();
                if !list.is_empty() {
                    self.selected_session_idx =
                        (self.selected_session_idx + page_size).min(list.len() - 1);
                    self.preview_scroll = 0;
                }
            }
            FocusPanel::Preview => {
                self.preview_scroll = self.preview_scroll.saturating_add(page_size as u16);
            }
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        match self.focus {
            FocusPanel::List => {
                self.selected_session_idx = self.selected_session_idx.saturating_sub(page_size);
                self.preview_scroll = 0;
            }
            FocusPanel::Preview => {
                self.preview_scroll = self.preview_scroll.saturating_sub(page_size as u16);
            }
        }
    }

    pub fn preview_scroll_up(&mut self, amount: u16) {
        self.preview_scroll = self.preview_scroll.saturating_sub(amount);
    }

    pub fn preview_scroll_down(&mut self, amount: u16) {
        self.preview_scroll = self.preview_scroll.saturating_add(amount);
    }

    pub fn filtered_sessions(&self) -> Vec<&Session> {
        let list = self.current_list();

        let mut filtered: Vec<&Session> = if self.search_query.is_empty() {
            list.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            list.iter()
                .filter(|s| {
                    s.id.to_lowercase().contains(&q)
                        || s.project_name.to_lowercase().contains(&q)
                        || s.slug
                            .as_deref()
                            .map(|sl| sl.to_lowercase().contains(&q))
                            .unwrap_or(false)
                        || s.messages
                            .iter()
                            .any(|m| m.content.to_lowercase().contains(&q))
                })
                .collect()
        };

        filtered.sort_by(|a, b| {
            let ordering = match self.sort_field {
                SortField::Project => a.project_name.cmp(&b.project_name),
                SortField::Name => a.slug.cmp(&b.slug),
                SortField::Messages => a.messages.len().cmp(&b.messages.len()),
                SortField::Date => a.updated_at.cmp(&b.updated_at),
            };

            match self.sort_direction {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            }
        });

        filtered
    }

    pub fn toggle_sort(&mut self) {
        self.sort_field = match self.sort_field {
            SortField::Project => SortField::Name,
            SortField::Name => SortField::Date,
            SortField::Date => SortField::Messages,
            SortField::Messages => SortField::Project,
        };
        self.sort_direction = SortDirection::Descending;
    }

    pub fn toggle_sort_direction(&mut self) {
        self.sort_direction = match self.sort_direction {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        };
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        if !self.show_help {
            self.help_scroll = 0;
        }
    }

    pub fn help_scroll_up(&mut self, amount: u16) {
        self.help_scroll = self.help_scroll.saturating_sub(amount);
    }

    pub fn help_scroll_down(&mut self, amount: u16) {
        self.help_scroll = self.help_scroll.saturating_add(amount);
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

                let store = SessionStore::new();
                let _ = store.move_to_trash(&removed.project_name, &removed.id);

                self.trash.push(removed);

                self.set_status(format!("Moved to trash: {}", id));
                if self.selected_session_idx > 0 && self.selected_session_idx >= self.sessions.len()
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

                let store = SessionStore::new();
                let _ = store.restore_session_file(&removed);

                self.sessions.push(removed);

                self.set_status(format!("Restored: {}", id));
                if self.selected_session_idx > 0 && self.selected_session_idx >= self.trash.len() {
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
            "PERMANENTLY delete ALL {} sessions in trash? Press 't' or 'y' to confirm, 'n' or Esc to cancel",
            count
        ));
    }

    pub fn request_trash_zero_messages(&mut self) {
        if self.current_tab != Tab::Sessions {
            return;
        }

        let count = self
            .sessions
            .iter()
            .filter(|s| s.messages.is_empty())
            .count();
        if count == 0 {
            self.set_status("No empty sessions found".to_string());
            return;
        }

        self.confirm_action = Some(ConfirmAction::TrashZeroMessages);
        self.set_status(format!(
            "Move {} session(s) with 0 messages to trash? Press 'y' to confirm, 'n' or Esc to cancel",
            count
        ));
    }

    pub fn trash_zero_messages(&mut self) {
        let (empty, non_empty): (Vec<_>, Vec<_>) =
            self.sessions.drain(..).partition(|s| s.messages.is_empty());
        let count = empty.len();
        self.sessions = non_empty;

        let store = SessionStore::new();
        for session in &empty {
            let _ = store.move_to_trash(&session.project_name, &session.id);
        }

        self.trash.extend(empty);

        if self.selected_session_idx >= self.sessions.len() && !self.sessions.is_empty() {
            self.selected_session_idx = self.sessions.len() - 1;
        } else if self.sessions.is_empty() {
            self.selected_session_idx = 0;
        }

        self.confirm_action = None;
        self.set_status(format!("Moved {} empty session(s) to trash", count));
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
                ConfirmAction::TrashZeroMessages => {
                    self.trash_zero_messages();
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

            self.set_status(format!("Permanently deleted: {}", session_id));
            self.confirm_action = None;

            if self.selected_session_idx > 0 && self.selected_session_idx >= self.trash.len() {
                self.selected_session_idx -= 1;
            }
        }
    }

    fn empty_trash(&mut self) {
        let count = self.trash.len();

        let store = SessionStore::new();
        let _ = store.empty_trash();

        self.trash.clear();

        self.set_status(format!("Permanently deleted {} sessions", count));
        self.confirm_action = None;
        self.selected_session_idx = 0;
    }

    pub fn open_settings(&mut self) {
        self.settings_input = self.config.export_path.clone();
        self.show_settings = true;
    }

    pub fn save_settings(&mut self) {
        self.config.export_path = self.settings_input.clone();
        let _ = self.config.save();
        self.show_settings = false;
        self.set_status(format!("Settings saved: {}", self.settings_input));
    }

    pub fn cancel_settings(&mut self) {
        self.show_settings = false;
    }

    pub fn settings_add_char(&mut self, c: char) {
        self.settings_input.push(c);
    }

    pub fn settings_pop_char(&mut self) {
        self.settings_input.pop();
    }

    pub fn open_rename(&mut self) {
        let current_name = self
            .get_selected_session()
            .map(|s| s.slug.clone().unwrap_or_default())
            .unwrap_or_default();
        self.rename_input = current_name;
        self.show_rename = true;
    }

    pub fn save_rename(&mut self) -> Option<crate::models::Session> {
        self.show_rename = false;
        self.get_selected_session().cloned()
    }

    pub fn cancel_rename(&mut self) {
        self.show_rename = false;
        self.rename_input.clear();
    }

    pub fn rename_add_char(&mut self, c: char) {
        self.rename_input.push(c);
    }

    pub fn rename_pop_char(&mut self) {
        self.rename_input.pop();
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
            jsonl_path: std::path::PathBuf::new(),
            slug: None,
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
        let mut app = App::with_sessions(vec![make_session("s1", "p1"), make_session("s2", "p2")]);
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
    fn test_search_filters_by_slug() {
        let mut s1 = make_session("s1", "proj1");
        s1.slug = Some("delme".to_string());
        let s2 = make_session("s2", "proj2");
        let mut app = App::with_sessions(vec![s1, s2]);
        app.search_query = "delme".to_string();
        let filtered = app.filtered_sessions();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "s1");
    }

    #[test]
    fn test_open_rename_prefills_custom_title() {
        let mut s1 = make_session("s1", "proj1");
        s1.slug = Some("my-title".to_string());
        let mut app = App::with_sessions(vec![s1]);
        app.open_rename();
        assert_eq!(app.rename_input, "my-title");
    }

    #[test]
    fn test_open_rename_prefills_empty_when_no_title() {
        let mut app = App::with_sessions(vec![make_session("s1", "proj1")]);
        app.open_rename();
        assert_eq!(app.rename_input, "");
    }

    #[test]
    fn test_sort_by_name() {
        let mut s1 = make_session("s1", "proj");
        s1.slug = Some("beta".to_string());
        let mut s2 = make_session("s2", "proj");
        s2.slug = Some("alpha".to_string());
        let mut app = App::with_sessions(vec![s1, s2]);
        app.sort_field = SortField::Name;
        app.sort_direction = crate::app::SortDirection::Ascending;
        let filtered = app.filtered_sessions();
        assert_eq!(filtered[0].slug.as_deref(), Some("alpha"));
        assert_eq!(filtered[1].slug.as_deref(), Some("beta"));
    }

    #[test]
    fn test_display_name_shows_custom_title() {
        let mut s = make_session("abcdef12-long-id", "my-project");
        s.slug = Some("my-label".to_string());
        assert!(s.display_name().contains("my-label"));
    }

    #[test]
    fn test_display_name_without_custom_title() {
        let s = make_session("abcdef12-long-id", "my-project");
        assert!(!s.display_name().contains('['));
        assert!(s.display_name().contains("abcdef12"));
    }

    #[test]
    fn test_custom_title_visible_in_filtered_sessions() {
        let mut s1 = make_session("s1", "proj1");
        s1.slug = Some("visible-name".to_string());
        let app = App::with_sessions(vec![s1]);
        let filtered = app.filtered_sessions();
        assert_eq!(filtered[0].slug.as_deref(), Some("visible-name"));
    }

    #[test]
    fn test_sort_none_titles_come_first_ascending() {
        let mut s1 = make_session("s1", "proj");
        s1.slug = Some("zebra".to_string());
        let s2 = make_session("s2", "proj"); // slug = None
        let mut app = App::with_sessions(vec![s1, s2]);
        app.sort_field = SortField::Name;
        app.sort_direction = crate::app::SortDirection::Ascending;
        let filtered = app.filtered_sessions();
        // None < Some, so s2 (None) comes first
        assert_eq!(filtered[0].slug, None);
        assert_eq!(filtered[1].slug.as_deref(), Some("zebra"));
    }

    #[test]
    fn test_move_to_trash() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1"), make_session("s2", "p2")]);
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
        app.focus = FocusPanel::Preview;
        app.page_down(3);
        assert_eq!(app.preview_scroll, 3);
        app.page_down(3);
        assert_eq!(app.preview_scroll, 6);
        app.page_up(3);
        assert_eq!(app.preview_scroll, 3);
        app.page_up(3);
        assert_eq!(app.preview_scroll, 0);
        app.page_up(3); // no underflow
        assert_eq!(app.preview_scroll, 0);
    }

    #[test]
    fn test_select_next_resets_scroll() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1"), make_session("s2", "p2")]);
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
        let mut app = App::with_sessions(vec![make_session("s1", "p1"), make_session("s2", "p2")]);
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
    fn test_trash_zero_messages_moves_empty_sessions() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1"), make_session("s2", "p2")]);
        // Make s2 have 0 messages
        app.sessions[1].messages.clear();

        app.trash_zero_messages();

        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.sessions[0].id, "s1");
        assert_eq!(app.trash.len(), 1);
        assert_eq!(app.trash[0].id, "s2");
    }

    #[test]
    fn test_trash_zero_messages_moves_all_empty() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
            make_session("s3", "p3"),
        ]);
        app.sessions[0].messages.clear();
        app.sessions[2].messages.clear();

        app.trash_zero_messages();

        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.sessions[0].id, "s2");
        assert_eq!(app.trash.len(), 2);
    }

    #[test]
    fn test_trash_zero_messages_none_empty() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);

        app.trash_zero_messages();

        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.trash.len(), 0);
    }

    #[test]
    fn test_trash_zero_messages_adjusts_selection() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
            make_session("s3", "p3"),
        ]);
        app.sessions[0].messages.clear();
        app.sessions[1].messages.clear();
        app.selected_session_idx = 2;

        app.trash_zero_messages();

        // Only s3 remains, selection should be 0
        assert_eq!(app.selected_session_idx, 0);
    }

    #[test]
    fn test_request_trash_zero_messages_sets_confirmation() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1"), make_session("s2", "p2")]);
        app.sessions[1].messages.clear();

        app.request_trash_zero_messages();

        assert_eq!(app.confirm_action, Some(ConfirmAction::TrashZeroMessages));
        assert!(app.status_message.unwrap().contains("1"));
    }

    #[test]
    fn test_request_trash_zero_messages_none_found() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);

        app.request_trash_zero_messages();

        assert_eq!(app.confirm_action, None);
        assert!(app.status_message.unwrap().contains("No empty sessions"));
    }

    #[test]
    fn test_open_settings_copies_export_path_to_input() {
        let mut app = App::with_sessions(vec![]);
        app.config.export_path = "~/my-exports".to_string();
        app.open_settings();
        assert!(app.show_settings);
        assert_eq!(app.settings_input, "~/my-exports");
    }

    #[test]
    fn test_save_settings_updates_config_and_closes_modal() {
        let mut app = App::with_sessions(vec![]);
        app.open_settings();
        app.settings_input = "/new/path".to_string();
        app.save_settings();
        assert!(!app.show_settings);
        assert_eq!(app.config.export_path, "/new/path");
    }

    #[test]
    fn test_cancel_settings_closes_modal_without_saving() {
        let mut app = App::with_sessions(vec![]);
        app.config.export_path = "~/original".to_string();
        app.open_settings();
        app.settings_input = "~/changed".to_string();
        app.cancel_settings();
        assert!(!app.show_settings);
        assert_eq!(app.config.export_path, "~/original");
    }

    #[test]
    fn test_settings_add_and_pop_char() {
        let mut app = App::with_sessions(vec![]);
        app.open_settings();
        app.settings_add_char('/');
        app.settings_add_char('f');
        app.settings_add_char('o');
        assert_eq!(app.settings_input, "~/claude-exports/fo");
        app.settings_pop_char();
        assert_eq!(app.settings_input, "~/claude-exports/f");
    }

    #[test]
    fn test_default_focus_is_list() {
        let app = App::with_sessions(vec![make_session("s1", "p1")]);
        assert_eq!(app.focus, FocusPanel::List);
    }

    #[test]
    fn test_focus_switches_to_preview() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.focus_right();
        assert_eq!(app.focus, FocusPanel::Preview);
    }

    #[test]
    fn test_focus_switches_back_to_list() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.focus_right();
        app.focus_left();
        assert_eq!(app.focus, FocusPanel::List);
    }

    #[test]
    fn test_focus_left_stays_at_list() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.focus_left();
        assert_eq!(app.focus, FocusPanel::List);
    }

    #[test]
    fn test_focus_right_stays_at_preview() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.focus_right();
        app.focus_right();
        assert_eq!(app.focus, FocusPanel::Preview);
    }

    #[test]
    fn test_page_down_list_moves_selection() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
            make_session("s3", "p3"),
            make_session("s4", "p4"),
            make_session("s5", "p5"),
        ]);
        app.focus = FocusPanel::List;
        app.page_down(10); // page size larger than list
        assert_eq!(app.selected_session_idx, 4); // clamped to last
    }

    #[test]
    fn test_page_up_list_moves_selection() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
            make_session("s3", "p3"),
            make_session("s4", "p4"),
            make_session("s5", "p5"),
        ]);
        app.focus = FocusPanel::List;
        app.selected_session_idx = 4;
        app.page_up(10);
        assert_eq!(app.selected_session_idx, 0);
    }

    #[test]
    fn test_page_down_preview_scrolls() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.focus = FocusPanel::Preview;
        app.page_down(10);
        assert_eq!(app.preview_scroll, 10);
    }

    #[test]
    fn test_page_up_preview_scrolls() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.focus = FocusPanel::Preview;
        app.preview_scroll = 15;
        app.page_up(10);
        assert_eq!(app.preview_scroll, 5);
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

    #[test]
    fn test_toggle_sort_cycles_fields() {
        let mut app = App::with_sessions(vec![]);
        assert_eq!(app.sort_field, SortField::Date);
        app.toggle_sort();
        assert_eq!(app.sort_field, SortField::Messages);
        app.toggle_sort();
        assert_eq!(app.sort_field, SortField::Project);
        app.toggle_sort();
        assert_eq!(app.sort_field, SortField::Name);
        app.toggle_sort();
        assert_eq!(app.sort_field, SortField::Date);
    }

    #[test]
    fn test_toggle_sort_resets_direction_to_descending() {
        let mut app = App::with_sessions(vec![]);
        app.sort_direction = SortDirection::Ascending;
        app.toggle_sort();
        assert_eq!(app.sort_direction, SortDirection::Descending);
    }

    #[test]
    fn test_toggle_sort_direction() {
        let mut app = App::with_sessions(vec![]);
        assert_eq!(app.sort_direction, SortDirection::Descending);
        app.toggle_sort_direction();
        assert_eq!(app.sort_direction, SortDirection::Ascending);
        app.toggle_sort_direction();
        assert_eq!(app.sort_direction, SortDirection::Descending);
    }

    #[test]
    fn test_toggle_help_opens_and_closes() {
        let mut app = App::with_sessions(vec![]);
        assert!(!app.show_help);
        app.toggle_help();
        assert!(app.show_help);
        app.toggle_help();
        assert!(!app.show_help);
    }

    #[test]
    fn test_toggle_help_resets_scroll_on_close() {
        let mut app = App::with_sessions(vec![]);
        app.toggle_help();
        app.help_scroll = 42;
        app.toggle_help();
        assert_eq!(app.help_scroll, 0);
    }

    #[test]
    fn test_help_scroll_up_and_down() {
        let mut app = App::with_sessions(vec![]);
        app.help_scroll_down(10);
        assert_eq!(app.help_scroll, 10);
        app.help_scroll_up(3);
        assert_eq!(app.help_scroll, 7);
        app.help_scroll_up(100);
        assert_eq!(app.help_scroll, 0);
    }

    #[test]
    fn test_preview_scroll_up_and_down() {
        let mut app = App::with_sessions(vec![]);
        app.preview_scroll_down(5);
        assert_eq!(app.preview_scroll, 5);
        app.preview_scroll_up(3);
        assert_eq!(app.preview_scroll, 2);
        app.preview_scroll_up(100);
        assert_eq!(app.preview_scroll, 0);
    }

    #[test]
    fn test_toggle_search_opens_and_closes() {
        let mut app = App::with_sessions(vec![]);
        assert!(!app.show_search);
        app.toggle_search();
        assert!(app.show_search);
        app.toggle_search();
        assert!(!app.show_search);
    }

    #[test]
    fn test_toggle_search_clears_query_on_close() {
        let mut app = App::with_sessions(vec![]);
        app.toggle_search();
        app.search_query = "test".to_string();
        app.toggle_search();
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn test_add_search_char_resets_selection() {
        let mut app =
            App::with_sessions(vec![make_session("s1", "p1"), make_session("s2", "p2")]);
        app.selected_session_idx = 1;
        app.add_search_char('x');
        assert_eq!(app.selected_session_idx, 0);
        assert_eq!(app.search_query, "x");
    }

    #[test]
    fn test_pop_search_char_resets_selection() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.search_query = "abc".to_string();
        app.selected_session_idx = 1;
        app.pop_search_char();
        assert_eq!(app.search_query, "ab");
        assert_eq!(app.selected_session_idx, 0);
    }

    #[test]
    fn test_cancel_confirmation() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.request_delete_confirmation();
        assert!(app.is_confirmation_pending());
        app.cancel_confirmation();
        assert!(!app.is_confirmation_pending());
        assert!(app.status_message.unwrap().contains("cancelled"));
    }

    // --- confirm_and_execute / delete_permanently / empty_trash ---

    #[test]
    fn test_confirm_execute_delete_permanently() {
        let mut app = App::with_sessions(vec![]);
        app.trash = vec![make_session("trash-1", "p1")];
        app.current_tab = Tab::Trash;
        app.confirm_action = Some(ConfirmAction::DeletePermanently("trash-1".to_string()));
        app.confirm_and_execute();
        assert!(app.trash.is_empty());
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_confirm_execute_empty_trash() {
        let mut app = App::with_sessions(vec![]);
        app.trash = vec![make_session("t1", "p1"), make_session("t2", "p2")];
        app.current_tab = Tab::Trash;
        app.confirm_action = Some(ConfirmAction::EmptyTrash);
        app.confirm_and_execute();
        assert!(app.trash.is_empty());
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_confirm_execute_trash_zero_messages() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1"), make_session("s2", "p2")]);
        app.sessions[1].messages.clear();
        app.confirm_action = Some(ConfirmAction::TrashZeroMessages);
        app.confirm_and_execute();
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.trash.len(), 1);
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_confirm_execute_delete_to_trash_is_noop() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.confirm_action = Some(ConfirmAction::DeleteToTrash("s1".to_string()));
        app.confirm_and_execute();
        // DeleteToTrash is handled in main.rs, so this is a no-op
        assert_eq!(app.sessions.len(), 1);
        assert!(app.confirm_action.is_some()); // not cleared by this path
    }

    #[test]
    fn test_delete_permanently_adjusts_selection() {
        let mut app = App::with_sessions(vec![]);
        app.trash = vec![make_session("t1", "p1"), make_session("t2", "p2")];
        app.current_tab = Tab::Trash;
        app.selected_session_idx = 1;
        app.confirm_action = Some(ConfirmAction::DeletePermanently("t2".to_string()));
        app.delete_permanently();
        assert_eq!(app.trash.len(), 1);
        assert_eq!(app.selected_session_idx, 0);
    }

    #[test]
    fn test_delete_permanently_wrong_action_early_return() {
        let mut app = App::with_sessions(vec![]);
        app.trash = vec![make_session("t1", "p1")];
        app.confirm_action = Some(ConfirmAction::EmptyTrash);
        app.delete_permanently();
        // Should do nothing because action is not DeletePermanently
        assert_eq!(app.trash.len(), 1);
    }

    #[test]
    fn test_empty_trash_clears_all_and_resets() {
        let mut app = App::with_sessions(vec![]);
        app.trash = vec![make_session("t1", "p1"), make_session("t2", "p2")];
        app.current_tab = Tab::Trash;
        app.selected_session_idx = 1;
        app.confirm_action = Some(ConfirmAction::EmptyTrash);
        app.empty_trash();
        assert!(app.trash.is_empty());
        assert_eq!(app.selected_session_idx, 0);
        assert!(app.confirm_action.is_none());
        assert!(app.status_message.unwrap().contains("2 sessions"));
    }

    #[test]
    fn test_move_to_trash_noop_in_trash_tab() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.current_tab = Tab::Trash;
        app.move_selected_to_trash();
        assert_eq!(app.sessions.len(), 1);
        assert!(app.trash.is_empty());
    }

    #[test]
    fn test_request_empty_trash_noop_in_sessions_tab() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.trash = vec![make_session("t1", "p1")];
        app.request_empty_trash();
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_request_empty_trash_when_already_empty() {
        let mut app = App::with_sessions(vec![]);
        app.current_tab = Tab::Trash;
        app.request_empty_trash();
        assert!(app.confirm_action.is_none());
        assert!(app.status_message.unwrap().contains("already empty"));
    }

    #[test]
    fn test_request_delete_confirmation_in_trash_tab() {
        let mut app = App::with_sessions(vec![]);
        app.trash = vec![make_session("t1", "p1")];
        app.current_tab = Tab::Trash;
        app.request_delete_confirmation();
        assert_eq!(
            app.confirm_action,
            Some(ConfirmAction::DeletePermanently("t1".to_string()))
        );
        assert!(app.status_message.unwrap().contains("PERMANENTLY"));
    }

    #[test]
    fn test_request_trash_zero_messages_noop_in_trash_tab() {
        let mut app = App::with_sessions(vec![]);
        app.current_tab = Tab::Trash;
        app.request_trash_zero_messages();
        assert!(app.confirm_action.is_none());
    }

    // --- filtered_sessions sorting ---

    #[test]
    fn test_filtered_sessions_sorts_by_project() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "zulu"),
            make_session("s2", "alpha"),
        ]);
        app.sort_field = SortField::Project;
        app.sort_direction = SortDirection::Ascending;
        let filtered = app.filtered_sessions();
        assert_eq!(filtered[0].project_name, "alpha");
        assert_eq!(filtered[1].project_name, "zulu");
    }

    #[test]
    fn test_filtered_sessions_sorts_by_messages() {
        let mut s1 = make_session("s1", "p1");
        s1.messages.push(Message {
            role: "user".to_string(),
            content: "extra".to_string(),
        });
        let s2 = make_session("s2", "p2");
        let mut app = App::with_sessions(vec![s1, s2]);
        app.sort_field = SortField::Messages;
        app.sort_direction = SortDirection::Ascending;
        let filtered = app.filtered_sessions();
        assert_eq!(filtered[0].messages.len(), 1); // s2 has 1
        assert_eq!(filtered[1].messages.len(), 2); // s1 has 2
    }

    #[test]
    fn test_filtered_sessions_descending_reverses() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "alpha"),
            make_session("s2", "zulu"),
        ]);
        app.sort_field = SortField::Project;
        app.sort_direction = SortDirection::Descending;
        let filtered = app.filtered_sessions();
        assert_eq!(filtered[0].project_name, "zulu");
        assert_eq!(filtered[1].project_name, "alpha");
    }

    #[test]
    fn test_confirm_execute_with_no_action() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.confirm_action = None;
        app.confirm_and_execute(); // should not panic
        assert_eq!(app.sessions.len(), 1);
    }
}
