// src/commands.rs

use crate::models::Session;
use anyhow::Result;

pub fn delete_session(_session: &Session) -> Result<()> {
    // For now, just create placeholder that succeeds
    // Full implementation would move to trash
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_session_succeeds() {
        let session = Session::new("test".to_string(), "/path".to_string());
        let result = delete_session(&session);
        assert!(result.is_ok());
    }
}
