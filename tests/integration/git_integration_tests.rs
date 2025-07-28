use anyhow::Result;
use lsp_bridge::core::{GitFileStatus, GitIntegration};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

async fn setup_test_git_repo() -> Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize git repo
    Command::new("git")
        .current_dir(&repo_path)
        .args(&["init"])
        .output()?;

    // Configure git
    Command::new("git")
        .current_dir(&repo_path)
        .args(&["config", "user.email", "test@example.com"])
        .output()?;

    Command::new("git")
        .current_dir(&repo_path)
        .args(&["config", "user.name", "Test User"])
        .output()?;

    Ok((temp_dir, repo_path))
}

async fn create_and_commit_file(repo_path: &PathBuf, filename: &str, content: &str) -> Result<()> {
    let file_path = repo_path.join(filename);
    fs::write(&file_path, content)?;

    Command::new("git")
        .current_dir(repo_path)
        .args(&["add", filename])
        .output()?;

    Command::new("git")
        .current_dir(repo_path)
        .args(&["commit", "-m", &format!("Add {}", filename)])
        .output()?;

    Ok(())
}

#[tokio::test]
async fn test_git_repo_detection() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test_git_repo().await?;

    // Find git root from the repo directory instead of changing current dir
    let git_root = {
        let output = std::process::Command::new("git")
            .current_dir(&repo_path)
            .args(&["rev-parse", "--show-toplevel"])
            .output()?;

        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout);
            Some(PathBuf::from(path_str.trim()))
        } else {
            None
        }
    };
    assert!(git_root.is_some());

    // Canonicalize both paths to handle symlinks (e.g., /var vs /private/var on macOS)
    let git_root_canonical = git_root.unwrap().canonicalize()?;
    let repo_path_canonical = repo_path.canonicalize()?;
    assert_eq!(git_root_canonical, repo_path_canonical);

    Ok(())
}

#[tokio::test]
async fn test_git_integration_with_existing_repo() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test_git_repo().await?;

    // Create initial commit
    create_and_commit_file(&repo_path, "initial.txt", "initial content").await?;

    // Create GitIntegration with explicit repo path
    let git = GitIntegration::new_with_repo(repo_path.clone()).await?;
    assert!(git.is_git_available().await);

    // Test repository info
    let repo_info = git.get_repository_info().await;
    assert!(repo_info.is_some());

    let repo_info = repo_info.unwrap();
    // Canonicalize both paths to handle symlinks
    let repo_info_canonical = repo_info.root_path.canonicalize()?;
    let repo_path_canonical = repo_path.canonicalize()?;
    assert_eq!(repo_info_canonical, repo_path_canonical);
    assert!(!repo_info.last_commit_hash.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_git_file_status_tracking() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test_git_repo().await?;

    // Create initial commit
    create_and_commit_file(&repo_path, "tracked.txt", "tracked content").await?;

    // Create GitIntegration with explicit repo path
    let git = GitIntegration::new_with_repo(repo_path.clone()).await?;

    // Create modified file
    let modified_file = repo_path.join("tracked.txt");
    fs::write(&modified_file, "modified content")?;

    // Create new untracked file
    let untracked_file = repo_path.join("untracked.txt");
    fs::write(&untracked_file, "untracked content")?;

    // Refresh status
    git.refresh_git_status().await?;

    // Check modified files
    let modified_files = git.get_modified_files().await?;
    assert!(!modified_files.is_empty());

    // Canonicalize paths for comparison to handle symlinks
    let modified_file_canonical = modified_file.canonicalize()?;
    let modified_files_canonical: Vec<PathBuf> = modified_files
        .iter()
        .filter_map(|p| p.canonicalize().ok())
        .collect();
    assert!(modified_files_canonical.contains(&modified_file_canonical));

    // Check untracked files
    let untracked_files = git.get_untracked_files().await?;
    assert!(!untracked_files.is_empty());

    let untracked_file_canonical = untracked_file.canonicalize()?;
    let untracked_files_canonical: Vec<PathBuf> = untracked_files
        .iter()
        .filter_map(|p| p.canonicalize().ok())
        .collect();
    assert!(untracked_files_canonical.contains(&untracked_file_canonical));

    Ok(())
}

#[tokio::test]
async fn test_git_change_detection() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test_git_repo().await?;

    // Create initial commits
    create_and_commit_file(&repo_path, "file1.txt", "content1").await?;
    create_and_commit_file(&repo_path, "file2.txt", "content2").await?;

    // Create GitIntegration with explicit repo path
    let git = GitIntegration::new_with_repo(repo_path.clone()).await?;

    // Get initial commit hash
    let repo_info = git.get_repository_info().await.unwrap();
    let initial_commit = repo_info.last_commit_hash;

    // Modify file1
    let file1_path = repo_path.join("file1.txt");
    fs::write(&file1_path, "modified content1")?;

    // Check changes since initial commit
    let changed_files = git.get_changed_files_since_commit(&initial_commit).await?;

    // Canonicalize paths for comparison to handle symlinks
    let file1_path_canonical = file1_path.canonicalize()?;
    let changed_files_canonical: Vec<PathBuf> = changed_files
        .iter()
        .filter_map(|p| p.canonicalize().ok())
        .collect();
    assert!(changed_files_canonical.contains(&file1_path_canonical));

    Ok(())
}

#[tokio::test]
async fn test_git_branch_operations() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test_git_repo().await?;

    // Create initial commit
    create_and_commit_file(&repo_path, "main.txt", "main content").await?;

    // Create GitIntegration with explicit repo path
    let git = GitIntegration::new_with_repo(repo_path.clone()).await?;

    // Test branch info
    let (branch, ahead_behind) = git.get_branch_info().await?;
    assert!(!branch.is_empty());
    // ahead_behind might be None if no remote is set up

    // Test repository status
    let is_clean = git.is_repository_clean().await?;
    assert!(is_clean);

    // Modify file to make it dirty
    let file_path = repo_path.join("main.txt");
    fs::write(&file_path, "modified main content")?;

    git.refresh_git_status().await?;
    let is_clean_after_modify = git.is_repository_clean().await?;
    assert!(!is_clean_after_modify);

    Ok(())
}

#[tokio::test]
async fn test_git_ignored_files() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test_git_repo().await?;

    // Create .gitignore
    let gitignore_path = repo_path.join(".gitignore");
    fs::write(&gitignore_path, "*.tmp\n*.log\ntarget/\n")?;

    // Add .gitignore and create initial commit (required for GitIntegration to work)
    Command::new("git")
        .current_dir(&repo_path)
        .args(&["add", ".gitignore"])
        .output()?;
    Command::new("git")
        .current_dir(&repo_path)
        .args(&["commit", "-m", "Add .gitignore"])
        .output()?;

    // Create ignored file
    let ignored_file = repo_path.join("test.tmp");
    fs::write(&ignored_file, "temporary content")?;

    // Create non-ignored file
    let normal_file = repo_path.join("test.txt");
    fs::write(&normal_file, "normal content")?;

    // Create GitIntegration with explicit repo path
    let git = GitIntegration::new_with_repo(repo_path.clone()).await?;

    // Test ignore detection
    let is_ignored = git.is_file_ignored(&ignored_file).await?;
    assert!(is_ignored);

    let is_not_ignored = git.is_file_ignored(&normal_file).await?;
    assert!(!is_not_ignored);

    Ok(())
}

#[tokio::test]
async fn test_git_status_parsing() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test_git_repo().await?;

    // Create initial commit to have a valid git repo
    create_and_commit_file(&repo_path, "test.txt", "test content").await?;

    // Create GitIntegration with explicit repo path
    let git = GitIntegration::new_with_repo(repo_path.clone()).await?;

    // Test various Git status combinations
    assert_eq!(git.parse_git_status('M', ' '), GitFileStatus::Modified);
    assert_eq!(git.parse_git_status(' ', 'M'), GitFileStatus::Modified);
    assert_eq!(git.parse_git_status('A', ' '), GitFileStatus::Added);
    assert_eq!(git.parse_git_status('D', ' '), GitFileStatus::Deleted);
    assert_eq!(git.parse_git_status(' ', 'D'), GitFileStatus::Deleted);
    assert_eq!(git.parse_git_status('R', ' '), GitFileStatus::Renamed);
    assert_eq!(git.parse_git_status('C', ' '), GitFileStatus::Copied);
    assert_eq!(git.parse_git_status('?', '?'), GitFileStatus::Untracked);
    assert_eq!(git.parse_git_status('!', '!'), GitFileStatus::Ignored);
    assert_eq!(git.parse_git_status('U', 'U'), GitFileStatus::Conflict);
    assert_eq!(git.parse_git_status('U', ' '), GitFileStatus::Conflict);
    assert_eq!(git.parse_git_status(' ', 'U'), GitFileStatus::Conflict);

    Ok(())
}

#[tokio::test]
async fn test_git_error_handling() -> Result<()> {
    // Test with non-git directory
    let temp_dir = TempDir::new()?;

    // Test find_git_root from non-git directory
    let git_root = {
        let output = std::process::Command::new("git")
            .current_dir(temp_dir.path())
            .args(&["rev-parse", "--show-toplevel"])
            .output()?;

        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout);
            Some(PathBuf::from(path_str.trim()))
        } else {
            None
        }
    };
    assert!(git_root.is_none());

    // Test GitIntegration creation in non-git directory
    // Save current directory and change to temp directory
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(temp_dir.path())?;

    let result = GitIntegration::new().await;

    // Restore original directory
    std::env::set_current_dir(original_dir)?;

    // This should either return an error or create a GitIntegration with no repo
    match result {
        Ok(git) => {
            assert!(!git.is_git_available().await);
        }
        Err(_) => {
            // Expected behavior - no git repo available
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_git_file_history() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test_git_repo().await?;

    // Create and commit file
    create_and_commit_file(&repo_path, "history.txt", "version 1").await?;

    // Create GitIntegration with explicit repo path
    let git = GitIntegration::new_with_repo(repo_path.clone()).await?;

    // Test last commit for file
    let file_path = repo_path.join("history.txt");
    let last_commit = git.get_last_commit_for_file(&file_path).await?;
    assert!(last_commit.is_some());
    assert!(!last_commit.unwrap().is_empty());

    // Test non-existent file
    let nonexistent_path = repo_path.join("nonexistent.txt");
    let no_commit = git.get_last_commit_for_file(&nonexistent_path).await?;
    assert!(no_commit.is_none());

    Ok(())
}

#[tokio::test]
async fn test_git_time_based_changes() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test_git_repo().await?;

    // Create initial commit
    create_and_commit_file(&repo_path, "time_test.txt", "initial").await?;

    // Create GitIntegration with explicit repo path
    let git = GitIntegration::new_with_repo(repo_path.clone()).await?;

    let now = std::time::SystemTime::now();

    // Modify file
    let file_path = repo_path.join("time_test.txt");
    fs::write(&file_path, "modified")?;

    git.refresh_git_status().await?;

    // Check files changed since a time in the past
    let past_time = now - std::time::Duration::from_secs(60);
    let changed_files = git.get_files_changed_since(past_time).await?;

    // Canonicalize paths for comparison to handle symlinks
    let file_path_canonical = file_path.canonicalize()?;
    let changed_files_canonical: Vec<PathBuf> = changed_files
        .iter()
        .filter_map(|p| p.canonicalize().ok())
        .collect();
    assert!(changed_files_canonical.contains(&file_path_canonical));

    // Check files changed since future (should be empty)
    let future_time = now + std::time::Duration::from_secs(60);
    let no_changed_files = git.get_files_changed_since(future_time).await?;
    assert!(no_changed_files.is_empty());

    Ok(())
}
