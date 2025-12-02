#[cfg(test)]
mod test_working_dir_update {
    use goose::session::SessionManager;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_update_session_working_dir() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let initial_dir = temp_dir.path().to_path_buf();
        
        // Create another temp directory to change to
        let new_temp_dir = TempDir::new().expect("Failed to create second temp dir");
        let new_dir = new_temp_dir.path().to_path_buf();

        // Create a session with the initial directory
        let session = SessionManager::create_session(
            initial_dir.clone(),
            "Test Session".to_string(),
            goose::session::SessionType::User,
        )
        .await
        .expect("Failed to create session");

        println!("Created session with ID: {}", session.id);
        println!("Initial working_dir: {:?}", session.working_dir);

        // Verify initial directory
        assert_eq!(session.working_dir, initial_dir);

        // Update the working directory
        SessionManager::update_session(&session.id)
            .working_dir(new_dir.clone())
            .apply()
            .await
            .expect("Failed to update session working directory");

        // Fetch the updated session
        let updated_session = SessionManager::get_session(&session.id, false)
            .await
            .expect("Failed to get updated session");

        println!("Updated working_dir: {:?}", updated_session.working_dir);

        // Verify the directory was updated
        assert_eq!(updated_session.working_dir, new_dir);
        
        // Clean up
        SessionManager::delete_session(&session.id)
            .await
            .expect("Failed to delete session");
    }
}
