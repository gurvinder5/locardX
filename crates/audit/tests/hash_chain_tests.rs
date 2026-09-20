use locardx_audit::{AuditService, GENESIS_HASH};
use locardx_database::Database;
use rusqlite::params;
use std::sync::Arc;

fn setup_test_audit() -> (AuditService, Arc<Database>) {
    let db = Arc::new(Database::open(":memory:").expect("Failed to open test in-memory database"));
    let audit = AuditService::new(Arc::clone(&db));
    (audit, db)
}

#[test]
fn test_01_empty_chain_is_valid() {
    let (audit, _) = setup_test_audit();
    let res = audit.verify_chain().expect("verify_chain failed");
    assert!(res.is_valid);
    assert_eq!(res.total_events, 0);
    assert_eq!(res.last_verified_sequence, 0);
}

#[test]
fn test_02_sequential_events_create_unbroken_chain() {
    let (audit, _) = setup_test_audit();

    let e1 = audit
        .log_structured_event(
            "TEST_START",
            Some("admin"),
            Some("/dev/sda"),
            "Initial start",
        )
        .expect("log 1 failed");
    assert_eq!(e1.sequence_number, 1);
    assert_eq!(e1.prev_hash, GENESIS_HASH);
    assert!(!e1.current_hash.is_empty());

    let e2 = audit
        .log_structured_event(
            "HASH_CALC",
            Some("admin"),
            Some("/data/file.img"),
            "SHA256 calculated",
        )
        .expect("log 2 failed");
    assert_eq!(e2.sequence_number, 2);
    assert_eq!(e2.prev_hash, e1.current_hash);

    let e3 = audit
        .log_structured_event(
            "VERIFY",
            Some("admin"),
            Some("/data/file.img"),
            "Integrity verified",
        )
        .expect("log 3 failed");
    assert_eq!(e3.sequence_number, 3);
    assert_eq!(e3.prev_hash, e2.current_hash);

    let verification = audit.verify_chain().expect("verify_chain failed");
    assert!(verification.is_valid);
    assert_eq!(verification.total_events, 3);
    assert_eq!(verification.last_verified_sequence, 3);
    assert!(verification.broken_sequence.is_none());
}

#[test]
fn test_03_tamper_detection_on_content_modification() {
    let (audit, db) = setup_test_audit();

    audit.log_event("EVENT_1", "Legitimate entry 1").unwrap();
    audit.log_event("EVENT_2", "Legitimate entry 2").unwrap();
    audit.log_event("EVENT_3", "Legitimate entry 3").unwrap();

    assert!(audit.verify_chain().unwrap().is_valid);

    // Tamper with details of event sequence 2 directly in the database
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE audit_events SET details = 'Malicious alteration' WHERE sequence_number = 2",
            params![],
        )?;
        Ok(())
    })
    .unwrap();

    let verification = audit.verify_chain().expect("verify_chain failed");
    assert!(!verification.is_valid, "Tampered chain must be detected");
    assert_eq!(verification.broken_sequence, Some(2));
    assert!(verification.details.contains("tampered"));
}

#[test]
fn test_04_tamper_detection_on_record_deletion() {
    let (audit, db) = setup_test_audit();

    audit.log_event("EVENT_1", "Legitimate entry 1").unwrap();
    audit.log_event("EVENT_2", "Legitimate entry 2").unwrap();
    audit.log_event("EVENT_3", "Legitimate entry 3").unwrap();

    // Delete event sequence 2 directly from SQLite
    db.with_conn(|conn| {
        conn.execute(
            "DELETE FROM audit_events WHERE sequence_number = 2",
            params![],
        )?;
        Ok(())
    })
    .unwrap();

    let verification = audit.verify_chain().expect("verify_chain failed");
    assert!(
        !verification.is_valid,
        "Deleted chain link must be detected"
    );
    assert!(verification.broken_sequence.is_some());
}

#[test]
fn test_05_listing_events_respects_limit_and_order() {
    let (audit, _) = setup_test_audit();

    for i in 1..=5 {
        audit
            .log_event("SEQ_TEST", &format!("Event {}", i))
            .unwrap();
    }

    let list = audit.list_events(3).expect("list_events failed");
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].sequence_number, 5);
    assert_eq!(list[1].sequence_number, 4);
    assert_eq!(list[2].sequence_number, 3);
}
