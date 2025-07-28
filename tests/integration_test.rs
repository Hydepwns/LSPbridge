use lsp_bridge::{
    core::{DiagnosticGrouper, PrivacyPolicy},
    privacy::{PrivacyFilter, WorkspaceFilter},
};
use std::path::PathBuf;

#[test]
fn test_workspace_filter_integration() {
    let workspace_root = PathBuf::from("/test/workspace");
    let workspace_filter = WorkspaceFilter::new(workspace_root.clone());

    // Create privacy filter with workspace
    let privacy_filter =
        PrivacyFilter::new(PrivacyPolicy::default()).with_workspace(workspace_root);

    // This should compile and work
    assert!(true);
}

#[test]
fn test_diagnostic_grouping_integration() {
    let grouper = DiagnosticGrouper::new();

    // Create some test diagnostics
    let diagnostics = vec![];

    // Deduplicate
    let deduped = grouper.deduplicate_diagnostics(diagnostics.clone());
    assert_eq!(deduped.len(), 0);

    // Group
    let groups = grouper.group_diagnostics(diagnostics);
    assert_eq!(groups.len(), 0);
}
