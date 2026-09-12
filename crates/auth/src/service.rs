use crate::authorization::AuthorizationEngine;
use crate::models::{
    AuthSessionResponse, ChangePasswordRequest, CreateUserRequest, InitAdminRequest, LoginRequest,
    PublicUser, Session, User, UserRole,
};
use crate::password::{hash_password, verify_password};
use crate::session::{calculate_expiration, generate_session_token, is_expired};
use chrono::Utc;
use locardx_audit::AuditService;
use locardx_common::LocardError;
use locardx_database::Database;
use rusqlite::params;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// Core authentication and user management service.
/// Enforces closed registration, secure hashing, and role authorization.
pub struct AuthService {
    db: Arc<Database>,
    audit: Arc<AuditService>,
}

impl AuthService {
    pub fn new(db: Arc<Database>, audit: Arc<AuditService>) -> Self {
        Self { db, audit }
    }

    /// Checks if the application is in first-run state (no Administrator exists).
    pub fn is_first_run(&self) -> Result<bool, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT COUNT(*) FROM users WHERE role = 'Administrator' AND enabled = 1",
            )?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            Ok(count == 0)
        })
    }

    /// Initializes the first Administrator account during setup.
    /// SECURITY INVARIANT: Permanently locked once an Administrator exists.
    pub fn initialize_admin(&self, req: InitAdminRequest) -> Result<PublicUser, LocardError> {
        if !self.is_first_run()? {
            warn!("Rejected unauthorized attempt to invoke first-run administrator initialization");
            return Err(LocardError::SecurityViolation(
                "First-run setup has already been completed. Closed registration is active."
                    .to_string(),
            ));
        }

        let username = req.username.trim();
        validate_username(username)?;

        if !req.confirm_password.is_empty() && req.password != req.confirm_password {
            return Err(LocardError::SecurityViolation(
                "Password and confirmation password do not match.".to_string(),
            ));
        }

        let pwd_hash = hash_password(&req.password)?;
        let user_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let user = User {
            user_id: user_id.clone(),
            username: username.to_string(),
            password_hash: pwd_hash,
            role: UserRole::Administrator,
            enabled: true,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_login_at: None,
        };

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO users (user_id, username, password_hash, role, enabled, created_at, updated_at, last_login_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    user.user_id,
                    user.username,
                    user.password_hash,
                    user.role.to_string(),
                    if user.enabled { 1 } else { 0 },
                    user.created_at,
                    user.updated_at,
                    user.last_login_at
                ],
            )?;
            Ok(())
        })?;

        self.audit.log_event(
            "FIRST_ADMIN_CREATED",
            &format!(
                "Initial Administrator account '{}' created successfully",
                username
            ),
        )?;

        info!(username = %username, "Initial Administrator created. Setup closed.");
        Ok(PublicUser::from(&user))
    }

    /// Authenticates a user and issues a desktop session token.
    /// Defends against user enumeration by returning generic error messages on failure.
    pub fn login(&self, req: LoginRequest) -> Result<AuthSessionResponse, LocardError> {
        let username = req.username.trim();
        if username.is_empty() || req.password.is_empty() {
            return Err(LocardError::SecurityViolation(
                "Invalid username or password.".to_string(),
            ));
        }

        let now_epoch = Utc::now().timestamp();

        // 1. Check brute-force lockout status
        let is_locked = self.db.with_conn(|conn| {
            let mut stmt =
                conn.prepare("SELECT locked_until FROM login_attempts WHERE username = ?1")?;
            let mut rows = stmt.query(params![username])?;
            if let Some(row) = rows.next()? {
                let locked_until: i64 = row.get(0)?;
                Ok(locked_until > now_epoch)
            } else {
                Ok(false)
            }
        })?;

        if is_locked {
            warn!(username = %username, "Login attempt blocked: account is temporarily locked");
            return Err(LocardError::SecurityViolation(
                "Account is temporarily locked due to repeated failed login attempts. Please try again later."
                    .to_string(),
            ));
        }

        // 2. Query user record
        let maybe_user: Option<User> = self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT user_id, username, password_hash, role, enabled, created_at, updated_at, last_login_at
                 FROM users WHERE username = ?1",
            )?;
            let mut rows = stmt.query(params![username])?;
            if let Some(row) = rows.next()? {
                let role_str: String = row.get(3)?;
                let role = UserRole::from_str(&role_str).unwrap_or(UserRole::Viewer);
                let enabled_num: i64 = row.get(4)?;

                Ok(Some(User {
                    user_id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    role,
                    enabled: enabled_num != 0,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    last_login_at: row.get(7)?,
                }))
            } else {
                Ok(None)
            }
        })?;

        // 3. Verify credentials
        let authenticated = match &maybe_user {
            Some(user) => verify_password(&req.password, &user.password_hash)?,
            None => {
                // Constant-time mitigation against timing attacks
                let dummy_hash = "$argon2id$v=19$m=19456,t=2,p=1$eHh4eHh4eHh4eHh4eHh4$eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4";
                let _ = verify_password(&req.password, dummy_hash);
                false
            }
        };

        if !authenticated {
            // Register failed attempt and potential lockout
            self.record_failed_attempt(username, now_epoch)?;
            self.audit.log_event(
                "LOGIN_FAILURE",
                &format!("Failed login attempt for username '{}'", username),
            )?;
            return Err(LocardError::SecurityViolation(
                "Invalid username or password.".to_string(),
            ));
        }

        let user = maybe_user.unwrap();

        // 4. Verify account enabled status
        if !user.enabled {
            self.audit.log_event(
                "LOGIN_FAILURE",
                &format!("Login rejected: account '{}' is disabled", user.username),
            )?;
            return Err(LocardError::SecurityViolation(
                "Account is disabled. Please contact an administrator.".to_string(),
            ));
        }

        // 5. Clear failed attempts on success
        self.clear_failed_attempts(username)?;

        // 6. Generate session
        let token = generate_session_token();
        let expires_at = calculate_expiration();
        let now_str = Utc::now().to_rfc3339();

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO sessions (token, user_id, role, created_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    token,
                    user.user_id,
                    user.role.to_string(),
                    now_epoch,
                    expires_at
                ],
            )?;

            conn.execute(
                "UPDATE users SET last_login_at = ?1, updated_at = ?1 WHERE user_id = ?2",
                params![now_str, user.user_id],
            )?;

            Ok(())
        })?;

        self.audit.log_event(
            "LOGIN_SUCCESS",
            &format!("User '{}' authenticated successfully", user.username),
        )?;

        let mut pub_user = PublicUser::from(&user);
        pub_user.last_login_at = Some(now_str);

        Ok(AuthSessionResponse {
            token,
            user: pub_user,
            expires_at,
        })
    }

    /// Logs out an active session and invalidates the session token.
    pub fn logout(&self, token: &str) -> Result<(), LocardError> {
        let user_id: Option<String> = self.db.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT user_id FROM sessions WHERE token = ?1")?;
            let mut rows = stmt.query(params![token])?;
            if let Some(row) = rows.next()? {
                Ok(Some(row.get(0)?))
            } else {
                Ok(None)
            }
        })?;

        if let Some(uid) = user_id {
            self.db.with_conn(|conn| {
                conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])?;
                Ok(())
            })?;

            self.audit.log_event(
                "LOGOUT",
                &format!("Session for user_id '{}' terminated", uid),
            )?;
        }

        Ok(())
    }

    /// Resolves the authenticated user for an active session.
    pub fn get_current_user(&self, token: &str) -> Result<PublicUser, LocardError> {
        let session: Option<Session> = self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT token, user_id, role, created_at, expires_at FROM sessions WHERE token = ?1",
            )?;
            let mut rows = stmt.query(params![token])?;
            if let Some(row) = rows.next()? {
                let role_str: String = row.get(2)?;
                let role = UserRole::from_str(&role_str).unwrap_or(UserRole::Viewer);
                Ok(Some(Session {
                    token: row.get(0)?,
                    user_id: row.get(1)?,
                    role,
                    created_at: row.get(3)?,
                    expires_at: row.get(4)?,
                }))
            } else {
                Ok(None)
            }
        })?;

        let session = match session {
            Some(s) => s,
            None => {
                return Err(LocardError::SecurityViolation(
                    "Session is invalid or has ended. Please log in.".to_string(),
                ));
            }
        };

        if is_expired(session.expires_at) {
            let _ = self.db.with_conn(|conn| {
                conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])?;
                Ok(())
            });
            self.audit.log_event(
                "SESSION_EXPIRED",
                &format!("Session expired for user_id '{}'", session.user_id),
            )?;
            return Err(LocardError::SecurityViolation(
                "Session has expired. Please log in again.".to_string(),
            ));
        }

        let user: Option<User> = self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT user_id, username, password_hash, role, enabled, created_at, updated_at, last_login_at
                 FROM users WHERE user_id = ?1",
            )?;
            let mut rows = stmt.query(params![session.user_id])?;
            if let Some(row) = rows.next()? {
                let role_str: String = row.get(3)?;
                let role = UserRole::from_str(&role_str).unwrap_or(UserRole::Viewer);
                let enabled_num: i64 = row.get(4)?;

                Ok(Some(User {
                    user_id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    role,
                    enabled: enabled_num != 0,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    last_login_at: row.get(7)?,
                }))
            } else {
                Ok(None)
            }
        })?;

        match user {
            Some(u) if u.enabled => Ok(PublicUser::from(&u)),
            Some(_) => Err(LocardError::SecurityViolation(
                "Account is disabled.".to_string(),
            )),
            None => Err(LocardError::SecurityViolation(
                "User account no longer exists.".to_string(),
            )),
        }
    }

    /// Administrator command to create a new user account.
    /// Closed registration: only authenticated administrators can invoke this.
    pub fn create_user(
        &self,
        token: &str,
        req: CreateUserRequest,
    ) -> Result<PublicUser, LocardError> {
        let caller = self.get_current_user(token)?;
        AuthorizationEngine::require_admin(caller.role)?;

        let username = req.username.trim();
        validate_username(username)?;

        // Check uniqueness
        let exists: bool = self.db.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM users WHERE username = ?1")?;
            let count: i64 = stmt.query_row(params![username], |row| row.get(0))?;
            Ok(count > 0)
        })?;

        if exists {
            return Err(LocardError::SecurityViolation(format!(
                "Username '{}' is already in use.",
                username
            )));
        }

        let pwd_hash = hash_password(&req.password)?;
        let user_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let user = User {
            user_id: user_id.clone(),
            username: username.to_string(),
            password_hash: pwd_hash,
            role: req.role,
            enabled: true,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_login_at: None,
        };

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO users (user_id, username, password_hash, role, enabled, created_at, updated_at, last_login_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    user.user_id,
                    user.username,
                    user.password_hash,
                    user.role.to_string(),
                    if user.enabled { 1 } else { 0 },
                    user.created_at,
                    user.updated_at,
                    user.last_login_at
                ],
            )?;
            Ok(())
        })?;

        self.audit.log_event(
            "USER_CREATED",
            &format!(
                "Administrator '{}' created user '{}' with role '{}'",
                caller.username, user.username, user.role
            ),
        )?;

        info!(
            creator = %caller.username,
            created_user = %user.username,
            role = %user.role,
            "User account created by Administrator"
        );

        Ok(PublicUser::from(&user))
    }

    /// Administrator command to enable or disable an existing user account.
    pub fn set_user_enabled(
        &self,
        token: &str,
        user_id: &str,
        enabled: bool,
    ) -> Result<(), LocardError> {
        let caller = self.get_current_user(token)?;
        AuthorizationEngine::require_admin(caller.role)?;

        // Guard: Cannot disable the last active administrator
        if !enabled {
            let (target_is_admin, admin_count): (bool, i64) = self.db.with_conn(|conn| {
                let mut stmt = conn.prepare("SELECT role FROM users WHERE user_id = ?1")?;
                let mut rows = stmt.query(params![user_id])?;
                let target_role: Option<String> = if let Some(row) = rows.next()? {
                    Some(row.get(0)?)
                } else {
                    None
                };

                let is_admin = target_role.as_deref() == Some("Administrator");
                let mut count_stmt = conn.prepare(
                    "SELECT COUNT(*) FROM users WHERE role = 'Administrator' AND enabled = 1",
                )?;
                let count: i64 = count_stmt.query_row([], |r| r.get(0))?;

                Ok((is_admin, count))
            })?;

            if target_is_admin && admin_count <= 1 {
                return Err(LocardError::SecurityViolation(
                    "Cannot disable the sole active Administrator account.".to_string(),
                ));
            }
        }

        let now = Utc::now().to_rfc3339();
        let rows_affected = self.db.with_conn(|conn| {
            let affected = conn.execute(
                "UPDATE users SET enabled = ?1, updated_at = ?2 WHERE user_id = ?3",
                params![if enabled { 1 } else { 0 }, now, user_id],
            )?;

            // Invalidate active sessions immediately if account is disabled
            if !enabled {
                conn.execute("DELETE FROM sessions WHERE user_id = ?1", params![user_id])?;
            }

            Ok(affected)
        })?;

        if rows_affected == 0 {
            return Err(LocardError::SecurityViolation(
                "User account not found.".to_string(),
            ));
        }

        let event_type = if enabled {
            "USER_ENABLED"
        } else {
            "USER_DISABLED"
        };
        self.audit.log_event(
            event_type,
            &format!(
                "Administrator '{}' changed user_id '{}' enabled status to {}",
                caller.username, user_id, enabled
            ),
        )?;

        Ok(())
    }

    /// Administrator command to list all user accounts (sanitized, no password hashes).
    pub fn list_users(&self, token: &str) -> Result<Vec<PublicUser>, LocardError> {
        let caller = self.get_current_user(token)?;
        AuthorizationEngine::require_admin(caller.role)?;

        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT user_id, username, role, enabled, created_at, updated_at, last_login_at
                 FROM users ORDER BY created_at ASC",
            )?;

            let rows = stmt.query_map([], |row| {
                let role_str: String = row.get(2)?;
                let role = UserRole::from_str(&role_str).unwrap_or(UserRole::Viewer);
                let enabled_num: i64 = row.get(3)?;

                Ok(PublicUser {
                    user_id: row.get(0)?,
                    username: row.get(1)?,
                    role,
                    enabled: enabled_num != 0,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    last_login_at: row.get(6)?,
                })
            })?;

            let mut users = Vec::new();
            for user in rows {
                users.push(user?);
            }
            Ok(users)
        })
    }

    /// Updates password for the authenticated user session.
    pub fn change_password(
        &self,
        token: &str,
        req: ChangePasswordRequest,
    ) -> Result<(), LocardError> {
        let caller = self.get_current_user(token)?;

        // Verify current password
        let current_hash: String = self.db.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT password_hash FROM users WHERE user_id = ?1")?;
            stmt.query_row(params![caller.user_id], |row| row.get(0))
        })?;

        if !verify_password(&req.old_password, &current_hash)? {
            return Err(LocardError::SecurityViolation(
                "Current password verification failed.".to_string(),
            ));
        }

        let new_hash = hash_password(&req.new_password)?;
        let now = Utc::now().to_rfc3339();

        self.db.with_conn(|conn| {
            conn.execute(
                "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE user_id = ?3",
                params![new_hash, now, caller.user_id],
            )?;
            Ok(())
        })?;

        self.audit.log_event(
            "PASSWORD_CHANGED",
            &format!("Password changed for user '{}'", caller.username),
        )?;

        info!(username = %caller.username, "User password updated successfully");
        Ok(())
    }

    // Helper: records failed login attempt and calculates lockout
    fn record_failed_attempt(&self, username: &str, now: i64) -> Result<(), LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt =
                conn.prepare("SELECT failed_count FROM login_attempts WHERE username = ?1")?;
            let count: i64 = match stmt.query_row(params![username], |r| r.get(0)) {
                Ok(c) => c,
                Err(rusqlite::Error::QueryReturnedNoRows) => 0,
                Err(e) => return Err(e),
            };

            let new_count = count + 1;
            // Lock account for 5 minutes after 5 consecutive failures
            let locked_until = if new_count >= 5 { now + 300 } else { 0 };

            conn.execute(
                "INSERT INTO login_attempts (username, failed_count, locked_until)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(username) DO UPDATE SET
                    failed_count = excluded.failed_count,
                    locked_until = excluded.locked_until",
                params![username, new_count, locked_until],
            )?;

            Ok(())
        })
    }

    // Helper: clears failed attempts upon successful login
    fn clear_failed_attempts(&self, username: &str) -> Result<(), LocardError> {
        self.db.with_conn(|conn| {
            conn.execute(
                "DELETE FROM login_attempts WHERE username = ?1",
                params![username],
            )?;
            Ok(())
        })
    }
}

fn validate_username(username: &str) -> Result<(), LocardError> {
    if username.len() < 3 || username.len() > 32 {
        return Err(LocardError::SecurityViolation(
            "Username must be between 3 and 32 characters long.".to_string(),
        ));
    }

    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(LocardError::SecurityViolation(
            "Username may only contain letters, digits, underscores, and hyphens.".to_string(),
        ));
    }

    Ok(())
}
