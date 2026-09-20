-- LocardX Database Migration 013: Authentication & Role-Based Access Control Enhancements
-- Adds display_name and metadata_json columns, and performance indexes for session validation and role lookup.

-- Add display_name to users table if not present
ALTER TABLE users ADD COLUMN display_name TEXT;

-- Add metadata_json to users table if not present
ALTER TABLE users ADD COLUMN metadata_json TEXT NOT NULL DEFAULT '{}';

-- Performance indexes for frequent session validation and user lookup
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);
CREATE INDEX IF NOT EXISTS idx_users_enabled ON users(enabled);
