use locardx_database::Database;

#[test]
fn test_database_migrations_and_schema() {
    let db = Database::open(":memory:").expect("Failed to open in-memory database");
    assert!(db.is_healthy());

    db.with_conn(|conn| {
        // Verify schema_migrations table has version 5
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 5",
                [],
                |row| row.get(0),
            )
            .expect("Failed to query migration 5");
        assert_eq!(count, 1);

        // Verify safety_evaluations table exists
        let eval_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM safety_evaluations", [], |row| {
                row.get(0)
            })
            .expect("Failed to query safety_evaluations");
        assert_eq!(eval_count, 0);

        // Verify operation_confirmations table exists
        let conf_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM operation_confirmations", [], |row| {
                row.get(0)
            })
            .expect("Failed to query operation_confirmations");
        assert_eq!(conf_count, 0);

        Ok(())
    })
    .expect("with_conn failed");
}
