use goose_tui::utils::file_completion::derive_job_id_from_path;

#[test]
fn derive_job_id_simple() {
    assert_eq!(
        derive_job_id_from_path("/path/to/my-recipe.yaml"),
        "my-recipe"
    );
    assert_eq!(derive_job_id_from_path("daily_report.yaml"), "daily_report");
    assert_eq!(
        derive_job_id_from_path("~/recipes/Weekly Sync.yaml"),
        "weekly-sync"
    );
}

#[test]
fn derive_job_id_edge_cases() {
    assert_eq!(derive_job_id_from_path(""), "");
    assert_eq!(derive_job_id_from_path("/"), "");
    assert_eq!(derive_job_id_from_path("no-extension"), "no-extension");
}
