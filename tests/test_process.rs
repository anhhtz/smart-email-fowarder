use smart_email_forwarder::db::Database;
use smart_email_forwarder::services::poller::PollerManager;
use smart_email_forwarder::services::process::{
    collect_health_report, collect_process_status, format_bytes, format_duration,
};
use chrono::Utc;

#[test]
fn test_format_helpers() {
    // format_bytes
    assert_eq!(format_bytes(500), "500 B");
    assert_eq!(format_bytes(1024), "1.0 KB");
    assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
    assert_eq!(format_bytes(5 * 1024 * 1024 * 1024), "5.00 GB");

    // format_duration
    assert_eq!(format_duration(45), "45s");
    assert_eq!(format_duration(90), "1m 30s");
    assert_eq!(format_duration(3665), "1h 1m 5s");
    assert_eq!(format_duration(90065), "1d 1h 1m 5s");
}

#[tokio::test]
async fn test_process_status_collection() {
    let db = Database::new_in_memory().unwrap();
    let (poller, _rx) = PollerManager::new(db.clone());
    let status = poller.get_status().await;
    let start_time = Utc::now();

    let proc_status = collect_process_status(&db, status, start_time);
    println!("Process Status JSON:\n{}", serde_json::to_string_pretty(&proc_status).unwrap());

    assert!(proc_status.pid > 0);
    assert_eq!(proc_status.status, "Running");
    assert_eq!(proc_status.database.is_healthy, true);
    assert_eq!(proc_status.app_version, "0.1.0");
    assert!(proc_status.cpu_cores >= 1);
}

#[tokio::test]
async fn test_health_report_generation() {
    let db = Database::new_in_memory().unwrap();
    let (poller, _rx) = PollerManager::new(db.clone());

    let report = collect_health_report(&db, &poller).await;
    println!("Health Report JSON:\n{}", serde_json::to_string_pretty(&report).unwrap());

    assert!(report.checks.len() >= 5);
    // SQLite, Runtime, and Poller checks should be present
    assert!(report.checks.iter().any(|c| c.name.contains("SQLite")));
    assert!(report.checks.iter().any(|c| c.name.contains("Tokio")));
    assert!(report.checks.iter().any(|c| c.name.contains("Poller")));
}
