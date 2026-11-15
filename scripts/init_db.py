import os
import sqlite3
import glob

db_path = os.path.join("data", "locardx.db")
conn = sqlite3.connect(db_path)
cursor = conn.cursor()

# Baseline tables
cursor.executescript("""
CREATE TABLE IF NOT EXISTS app_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
INSERT OR IGNORE INTO app_metadata (key, value, updated_at)
VALUES ('schema_version', '15', datetime('now'));
""")

# Apply migrations from crates/database/migrations
migration_files = sorted(glob.glob("crates/database/migrations/*.sql"))
for mf in migration_files:
    with open(mf, "r", encoding="utf-8") as f:
        sql = f.read().strip()
        if sql:
            print(f"Applying migration: {mf}")
            try:
                cursor.executescript(sql)
            except Exception as e:
                print(f"Error executing {mf}: {e}")

# Insert default admin user if not exists
cursor.execute("""
INSERT OR IGNORE INTO users (user_id, username, password_hash, role, enabled, created_at, updated_at)
VALUES ('admin-01', 'admin', '$argon2id$v=19$m=19456,t=2,p=1$locardx$samplehash', 'administrator', 1, datetime('now'), datetime('now'));
""")

# Insert sample case
cursor.execute("""
INSERT OR IGNORE INTO cases (case_id, case_reference, title, description, status, lead_investigator, created_at, updated_at)
VALUES ('case-001', 'CASE-2026-001', 'Operation Ironclad - Forensic Triage', 'Investigation into seized media and forensic artifacts', 'open', 'admin', datetime('now'), datetime('now'));
""")

conn.commit()
print("Database schema successfully initialized!")
cursor.execute("SELECT name FROM sqlite_master WHERE type='table';")
tables = [row[0] for row in cursor.fetchall()]
print(f"Tables in database ({len(tables)}): {', '.join(tables)}")
conn.close()
