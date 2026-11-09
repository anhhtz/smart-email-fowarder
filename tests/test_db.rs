use smart_email_forwarder::db::Database;
use smart_email_forwarder::models::{ExchangeConfig, ForwardingConfig, SmtpConfig};

#[test]
fn test_database_initialization_and_dedup() {
    let db = Database::new_in_memory().expect("in-memory db initialization failed");

    // Initially not processed
    assert!(!db.is_processed("msg-1001", 1).unwrap());

    // Mark as processed
    db.mark_processed("msg-1001", 1, "Invoice #1001").unwrap();

    // Now it should be processed by message_id or uid
    assert!(db.is_processed("msg-1001", 99).unwrap());
    assert!(db.is_processed("other-id", 1).unwrap());
    assert!(!db.is_processed("unseen-id", 2).unwrap());
}

#[test]
fn test_config_save_and_password_preservation() {
    let db = Database::new_in_memory().unwrap();

    // 1. Initial save of Exchange config with password
    let mut ex_cfg = ExchangeConfig {
        host: "mail.company.local".to_string(),
        port: 993,
        security: "ssl".to_string(),
        username: "domain\\user1".to_string(),
        password: Some("SecretPass123!".to_string()),
        folder: "INBOX".to_string(),
        mark_as_read: true,
        allow_self_signed: true,
        smtp_port: 587,
        smtp_security: "starttls".to_string(),
        from_email: None,
        from_name: "Exchange Email Forwarder".to_string(),
    };
    db.save_exchange_config(&mut ex_cfg).unwrap();

    let loaded = db.get_config().unwrap().exchange.unwrap();
    assert_eq!(loaded.host, "mail.company.local");
    assert_eq!(loaded.password.as_deref(), Some("SecretPass123!"));
    assert!(loaded.allow_self_signed);

    // 2. Update host without re-entering password
    let mut update_cfg = ExchangeConfig {
        host: "mail2.company.local".to_string(),
        port: 993,
        security: "ssl".to_string(),
        username: "domain\\user1".to_string(),
        password: None, // blank password from UI
        folder: "INBOX".to_string(),
        mark_as_read: true,
        allow_self_signed: true,
        smtp_port: 587,
        smtp_security: "starttls".to_string(),
        from_email: None,
        from_name: "Exchange Email Forwarder".to_string(),
    };
    db.save_exchange_config(&mut update_cfg).unwrap();

    let updated = db.get_config().unwrap().exchange.unwrap();
    assert_eq!(updated.host, "mail2.company.local");
    assert_eq!(updated.password.as_deref(), Some("SecretPass123!")); // preserved!

    // 3. Save Forwarding config
    let fwd_cfg = ForwardingConfig {
        target_emails: vec!["admin@gmail.com".to_string(), "backup@gmail.com".to_string()],
        subject_prefix: "[FWD] ".to_string(),
        poll_interval_minutes: 5,
        include_banner: true,
        preserve_reply_to: true,
    };
    db.save_forwarding_config(&fwd_cfg).unwrap();

    let loaded_fwd = db.get_config().unwrap().forwarding.unwrap();
    assert_eq!(loaded_fwd.target_emails.len(), 2);
    assert_eq!(loaded_fwd.poll_interval_minutes, 5);

    // 4. Save SMTP relay config
    let mut smtp_cfg = SmtpConfig {
        host: "mail.company.local".to_string(),
        port: 25,
        security: "plain".to_string(),
        from_email: "forwarder@company.com".to_string(),
        from_name: "Relay Service".to_string(),
        auth_required: false,
        use_exchange_credentials: false,
        allow_self_signed: false,
        username: String::new(),
        password: None,
    };
    db.save_smtp_config(&mut smtp_cfg).unwrap();

    let loaded_smtp = db.get_config().unwrap().smtp.unwrap();
    assert_eq!(loaded_smtp.host, "mail.company.local");
    assert_eq!(loaded_smtp.port, 25);
    assert!(!loaded_smtp.auth_required);
    assert!(!loaded_smtp.use_exchange_credentials);
}

#[test]
fn test_exchange_to_smtp_config_default() {
    let ex = ExchangeConfig {
        host: "mail.company.local".to_string(),
        port: 993,
        security: "ssl".to_string(),
        username: "user@company.com".to_string(),
        password: Some("Pass123".to_string()),
        folder: "INBOX".to_string(),
        mark_as_read: true,
        allow_self_signed: true,
        smtp_port: 587,
        smtp_security: "starttls".to_string(),
        from_email: None,
        from_name: "Company Forwarder".to_string(),
    };

    let smtp = ex.to_smtp_config();
    assert_eq!(smtp.host, "mail.company.local");
    assert_eq!(smtp.port, 587);
    assert_eq!(smtp.security, "starttls");
    assert_eq!(smtp.username, "user@company.com");
    assert_eq!(smtp.password.as_deref(), Some("Pass123"));
    assert_eq!(smtp.from_email, "user@company.com");
    assert_eq!(smtp.from_name, "Company Forwarder");
    assert!(smtp.auth_required);
    assert!(smtp.use_exchange_credentials);
    assert!(smtp.allow_self_signed);
}

#[test]
fn test_smtp_use_exchange_credentials() {
    let db = Database::new_in_memory().unwrap();

    let mut ex_cfg = ExchangeConfig {
        host: "mail.company.local".to_string(),
        port: 993,
        security: "ssl".to_string(),
        username: "CORP\\jdoe".to_string(),
        password: Some("ExchangeSecret123!".to_string()),
        folder: "INBOX".to_string(),
        mark_as_read: true,
        allow_self_signed: false,
        smtp_port: 587,
        smtp_security: "starttls".to_string(),
        from_email: Some("jdoe@company.com".to_string()),
        from_name: "John Doe".to_string(),
    };
    db.save_exchange_config(&mut ex_cfg).unwrap();

    let mut smtp_cfg = SmtpConfig {
        host: "mail.company.local".to_string(),
        port: 25,
        security: "plain".to_string(),
        from_email: "jdoe@company.com".to_string(),
        from_name: "John Doe".to_string(),
        auth_required: true,
        use_exchange_credentials: true,
        allow_self_signed: false,
        username: String::new(),
        password: None,
    };
    db.save_smtp_config(&mut smtp_cfg).unwrap();

    let loaded_cfg = db.get_config().unwrap();
    let loaded_smtp = loaded_cfg.smtp.unwrap();
    assert!(loaded_smtp.use_exchange_credentials);
    assert!(loaded_smtp.auth_required);

    // Test resolve_with_exchange
    let resolved = loaded_smtp.resolve_with_exchange(loaded_cfg.exchange.as_ref());
    assert!(resolved.auth_required);
    assert_eq!(resolved.username, "CORP\\jdoe");
    assert_eq!(resolved.password.as_deref(), Some("ExchangeSecret123!"));
}

#[test]
fn test_forward_logs_and_stats() {
    let db = Database::new_in_memory().unwrap();

    let (s, f) = db.get_stats().unwrap();
    assert_eq!(s, 0);
    assert_eq!(f, 0);

    // Record 1 success and 1 failure
    db.record_forward_log(
        "msg-01",
        "sender@test.com",
        "Subject 1",
        "target@gmail.com",
        "SUCCESS",
        None,
    ).unwrap();

    db.record_forward_log(
        "msg-02",
        "sender@test.com",
        "Subject 2",
        "target@gmail.com",
        "FAILED",
        Some("Connection refused by SMTP relay"),
    ).unwrap();

    let (s2, f2) = db.get_stats().unwrap();
    assert_eq!(s2, 1);
    assert_eq!(f2, 1);

    let logs = db.get_forward_logs(10).unwrap();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].message_id, "msg-02");
    assert_eq!(logs[0].status, "FAILED");
    assert_eq!(logs[1].message_id, "msg-01");
    assert_eq!(logs[1].status, "SUCCESS");
}
