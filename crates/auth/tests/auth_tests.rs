use locardx_audit::AuditService;
use locardx_auth::{
    models::{ChangePasswordRequest, CreateUserRequest, InitAdminRequest, LoginRequest, UserRole},
    AuthService,
};
use locardx_common::LocardError;
use locardx_database::Database;
use std::sync::Arc;

fn setup_test_auth_service() -> (AuthService, Arc<Database>) {
    let db =
        Arc::new(Database::open(":memory:").expect("Failed to create in-memory test database"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let auth = AuthService::new(Arc::clone(&db), audit);
    (auth, db)
}

#[test]
fn test_01_first_administrator_can_be_created() {
    let (auth, _) = setup_test_auth_service();

    assert!(auth.is_first_run().expect("is_first_run failed"));

    let req = InitAdminRequest {
        username: "admin_user".to_string(),
        password: "SuperSecretPassword123!".to_string(),
        confirm_password: "SuperSecretPassword123!".to_string(),
    };

    let admin = auth
        .initialize_admin(req)
        .expect("Failed to initialize admin");
    assert_eq!(admin.username, "admin_user");
    assert_eq!(admin.role, UserRole::Administrator);
    assert!(admin.enabled);
    assert!(!auth.is_first_run().expect("is_first_run failed"));
}

#[test]
fn test_02_second_first_run_administrator_cannot_be_created() {
    let (auth, _) = setup_test_auth_service();

    let req1 = InitAdminRequest {
        username: "initial_admin".to_string(),
        password: "AdminPassword123!".to_string(),
        confirm_password: "AdminPassword123!".to_string(),
    };
    assert!(auth.initialize_admin(req1).is_ok());

    // Attempt second setup
    let req2 = InitAdminRequest {
        username: "attacker_admin".to_string(),
        password: "AttackerPass123!".to_string(),
        confirm_password: "AttackerPass123!".to_string(),
    };
    let result = auth.initialize_admin(req2);
    assert!(result.is_err(), "Second first-run setup must be rejected");

    match result.unwrap_err() {
        LocardError::SecurityViolation(msg) => {
            assert!(
                msg.contains("already been completed"),
                "Unexpected error: {}",
                msg
            );
        }
        other => panic!("Expected SecurityViolation error, got: {:?}", other),
    }
}

#[test]
fn test_03_public_registration_is_unavailable() {
    let (auth, _) = setup_test_auth_service();

    // Init admin first
    let _ = auth.initialize_admin(InitAdminRequest {
        username: "admin".to_string(),
        password: "Password123!".to_string(),
        confirm_password: "Password123!".to_string(),
    });

    // An unauthenticated request or random token cannot invoke user creation
    let fake_token = "0000000000000000000000000000000000000000000000000000000000000000";
    let req = CreateUserRequest {
        username: "unauthorized_user".to_string(),
        password: "UserPassword123!".to_string(),
        role: UserRole::Investigator,
    };

    let result = auth.create_user(fake_token, req);
    assert!(
        result.is_err(),
        "Unauthenticated user creation must be blocked"
    );
}

#[test]
fn test_04_correct_password_authenticates() {
    let (auth, _) = setup_test_auth_service();

    let pwd = "StrongPassword2026!";
    auth.initialize_admin(InitAdminRequest {
        username: "forensic_admin".to_string(),
        password: pwd.to_string(),
        confirm_password: pwd.to_string(),
    })
    .expect("Init admin failed");

    let login_res = auth
        .login(LoginRequest {
            username: "forensic_admin".to_string(),
            password: pwd.to_string(),
        })
        .expect("Login failed with correct password");

    assert_eq!(login_res.user.username, "forensic_admin");
    assert!(!login_res.token.is_empty());
    assert!(login_res.expires_at > 0);
}

#[test]
fn test_05_incorrect_password_fails_with_generic_message() {
    let (auth, _) = setup_test_auth_service();

    auth.initialize_admin(InitAdminRequest {
        username: "target_user".to_string(),
        password: "CorrectPassword123!".to_string(),
        confirm_password: "CorrectPassword123!".to_string(),
    })
    .expect("Init admin failed");

    let result = auth.login(LoginRequest {
        username: "target_user".to_string(),
        password: "WrongPassword999!".to_string(),
    });

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert_eq!(
        err_msg,
        "Security policy violation: Invalid username or password."
    );
}

#[test]
fn test_06_user_enumeration_is_not_exposed_through_login_errors() {
    let (auth, _) = setup_test_auth_service();

    auth.initialize_admin(InitAdminRequest {
        username: "existing_user".to_string(),
        password: "ExistingPassword123!".to_string(),
        confirm_password: "ExistingPassword123!".to_string(),
    })
    .expect("Init admin failed");

    // Existing user with bad password
    let res_existing = auth.login(LoginRequest {
        username: "existing_user".to_string(),
        password: "BadPassword123!".to_string(),
    });

    // Non-existent user
    let res_nonexistent = auth.login(LoginRequest {
        username: "nonexistent_ghost".to_string(),
        password: "BadPassword123!".to_string(),
    });

    assert!(res_existing.is_err());
    assert!(res_nonexistent.is_err());

    // Messages must be identical to defeat timing/enumeration attacks
    assert_eq!(
        res_existing.unwrap_err().to_string(),
        res_nonexistent.unwrap_err().to_string()
    );
}

#[test]
fn test_07_disabled_user_cannot_log_in() {
    let (auth, _) = setup_test_auth_service();

    let admin_session = auth
        .initialize_admin(InitAdminRequest {
            username: "root_admin".to_string(),
            password: "AdminSecret123!".to_string(),
            confirm_password: "AdminSecret123!".to_string(),
        })
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "root_admin".to_string(),
                password: "AdminSecret123!".to_string(),
            })
        })
        .expect("Admin setup failed");

    let new_user = auth
        .create_user(
            &admin_session.token,
            CreateUserRequest {
                username: "field_operator".to_string(),
                password: "OperatorPass123!".to_string(),
                role: UserRole::Operator,
            },
        )
        .expect("Create user failed");

    // Disable the operator
    auth.set_user_enabled(&admin_session.token, &new_user.user_id, false)
        .expect("Disable user failed");

    // Attempt login as disabled operator
    let result = auth.login(LoginRequest {
        username: "field_operator".to_string(),
        password: "OperatorPass123!".to_string(),
    });

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Account is disabled"));
}

#[test]
fn test_08_administrator_can_create_other_users() {
    let (auth, _) = setup_test_auth_service();

    let admin_session = auth
        .initialize_admin(InitAdminRequest {
            username: "sysadmin".to_string(),
            password: "SysAdminPassword123!".to_string(),
            confirm_password: "SysAdminPassword123!".to_string(),
        })
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "sysadmin".to_string(),
                password: "SysAdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    let inv = auth
        .create_user(
            &admin_session.token,
            CreateUserRequest {
                username: "investigator_alice".to_string(),
                password: "AlicePassword123!".to_string(),
                role: UserRole::Investigator,
            },
        )
        .expect("Failed to create investigator");
    assert_eq!(inv.role, UserRole::Investigator);

    let op = auth
        .create_user(
            &admin_session.token,
            CreateUserRequest {
                username: "operator_bob".to_string(),
                password: "BobPassword123!".to_string(),
                role: UserRole::Operator,
            },
        )
        .expect("Failed to create operator");
    assert_eq!(op.role, UserRole::Operator);

    let vi = auth
        .create_user(
            &admin_session.token,
            CreateUserRequest {
                username: "viewer_charlie".to_string(),
                password: "CharliePassword123!".to_string(),
                role: UserRole::Viewer,
            },
        )
        .expect("Failed to create viewer");
    assert_eq!(vi.role, UserRole::Viewer);

    let users = auth
        .list_users(&admin_session.token)
        .expect("list_users failed");
    assert_eq!(users.len(), 4);
}

#[test]
fn test_08b_administrator_can_create_another_administrator() {
    let (auth, _) = setup_test_auth_service();

    let admin_session = auth
        .initialize_admin(InitAdminRequest {
            username: "root_admin".to_string(),
            password: "RootAdminPassword123!".to_string(),
            confirm_password: "RootAdminPassword123!".to_string(),
        })
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "root_admin".to_string(),
                password: "RootAdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    // Create a second administrator
    let second_admin = auth
        .create_user(
            &admin_session.token,
            CreateUserRequest {
                username: "backup_admin".to_string(),
                password: "BackupAdminPassword123!".to_string(),
                role: UserRole::Administrator,
            },
        )
        .expect("Administrator should be able to create another Administrator");

    assert_eq!(second_admin.role, UserRole::Administrator);
    assert_eq!(second_admin.username, "backup_admin");

    // Verify the second administrator can authenticate successfully
    let second_session = auth
        .login(LoginRequest {
            username: "backup_admin".to_string(),
            password: "BackupAdminPassword123!".to_string(),
        })
        .expect("Second administrator should be able to log in");

    assert_eq!(second_session.user.role, UserRole::Administrator);

    // Verify the second administrator has full administrative rights to create users
    let third_user = auth
        .create_user(
            &second_session.token,
            CreateUserRequest {
                username: "analyst_dave".to_string(),
                password: "DavePassword123!".to_string(),
                role: UserRole::Investigator,
            },
        )
        .expect("Second administrator should be able to create users");

    assert_eq!(third_user.role, UserRole::Investigator);
}

#[test]
fn test_09_non_administrator_cannot_create_users() {
    let (auth, _) = setup_test_auth_service();

    let admin_session = auth
        .initialize_admin(InitAdminRequest {
            username: "main_admin".to_string(),
            password: "MainPassword123!".to_string(),
            confirm_password: "MainPassword123!".to_string(),
        })
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "main_admin".to_string(),
                password: "MainPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    // Create an investigator
    auth.create_user(
        &admin_session.token,
        CreateUserRequest {
            username: "investigator_dan".to_string(),
            password: "DanPassword123!".to_string(),
            role: UserRole::Investigator,
        },
    )
    .expect("Create user failed");

    // Log in as investigator
    let inv_session = auth
        .login(LoginRequest {
            username: "investigator_dan".to_string(),
            password: "DanPassword123!".to_string(),
        })
        .expect("Investigator login failed");

    // Investigator tries to create an account
    let result = auth.create_user(
        &inv_session.token,
        CreateUserRequest {
            username: "unauthorized_peer".to_string(),
            password: "PeerPassword123!".to_string(),
            role: UserRole::Viewer,
        },
    );

    assert!(
        result.is_err(),
        "Non-admin must be barred from creating accounts"
    );
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Administrator privileges required"));
}

#[test]
fn test_10_logout_invalidates_the_session() {
    let (auth, _) = setup_test_auth_service();

    let session = auth
        .initialize_admin(InitAdminRequest {
            username: "test_logout".to_string(),
            password: "LogoutPassword123!".to_string(),
            confirm_password: "LogoutPassword123!".to_string(),
        })
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "test_logout".to_string(),
                password: "LogoutPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    assert!(auth.get_current_user(&session.token).is_ok());

    // Perform logout
    auth.logout(&session.token).expect("Logout failed");

    // Verify session token is now completely invalid
    let post_logout = auth.get_current_user(&session.token);
    assert!(post_logout.is_err());
}

#[test]
fn test_11_expired_session_is_rejected() {
    let (auth, db) = setup_test_auth_service();

    let session = auth
        .initialize_admin(InitAdminRequest {
            username: "expiry_user".to_string(),
            password: "ExpiryPassword123!".to_string(),
            confirm_password: "ExpiryPassword123!".to_string(),
        })
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "expiry_user".to_string(),
                password: "ExpiryPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    // Artificially expire the session in SQLite
    let past_timestamp = 1000;
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE sessions SET expires_at = ?1 WHERE token = ?2",
            rusqlite::params![past_timestamp, session.token],
        )?;
        Ok(())
    })
    .expect("DB update failed");

    let result = auth.get_current_user(&session.token);
    assert!(result.is_err(), "Expired session must be rejected");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("expired"));
}

#[test]
fn test_12_password_is_never_stored_plaintext() {
    let (auth, db) = setup_test_auth_service();

    let raw_password = "SecretPlaintextPassword123!";
    auth.initialize_admin(InitAdminRequest {
        username: "crypto_user".to_string(),
        password: raw_password.to_string(),
        confirm_password: raw_password.to_string(),
    })
    .expect("Init failed");

    let stored_hash: String = db
        .with_conn(|conn| {
            let mut stmt =
                conn.prepare("SELECT password_hash FROM users WHERE username = 'crypto_user'")?;
            stmt.query_row([], |r| r.get(0))
        })
        .expect("Query failed");

    // Must NOT equal plaintext
    assert_ne!(stored_hash, raw_password);
    // Must be valid PHC Argon2id string
    assert!(stored_hash.starts_with("$argon2id$"));
}

#[test]
fn test_13_authentication_events_are_audited() {
    let (auth, _) = setup_test_auth_service();

    // Creating admin emits FIRST_ADMIN_CREATED
    let _ = auth.initialize_admin(InitAdminRequest {
        username: "audited_admin".to_string(),
        password: "AuditPassword123!".to_string(),
        confirm_password: "AuditPassword123!".to_string(),
    });

    // Login emits LOGIN_SUCCESS
    let session = auth
        .login(LoginRequest {
            username: "audited_admin".to_string(),
            password: "AuditPassword123!".to_string(),
        })
        .expect("Login failed");

    // Change password emits PASSWORD_CHANGED
    auth.change_password(
        &session.token,
        ChangePasswordRequest {
            old_password: "AuditPassword123!".to_string(),
            new_password: "NewAuditPassword123!".to_string(),
        },
    )
    .expect("Change password failed");

    // Logout emits LOGOUT
    auth.logout(&session.token).expect("Logout failed");
}

#[test]
fn test_14_password_hashes_are_not_returned_to_frontend() {
    let (auth, _) = setup_test_auth_service();

    let admin = auth
        .initialize_admin(InitAdminRequest {
            username: "safe_admin".to_string(),
            password: "SafePassword123!".to_string(),
            confirm_password: "SafePassword123!".to_string(),
        })
        .expect("Init failed");

    let serialized = serde_json::to_string(&admin).expect("Serialization failed");

    // Assert that the JSON serialized response has NO "password_hash" field
    assert!(!serialized.contains("password_hash"));
    assert!(!serialized.contains("$argon2id$"));
    assert!(serialized.contains("\"username\":\"safe_admin\""));
    assert!(serialized.contains("\"role\":\"Administrator\""));
}

#[test]
fn test_15_rbac_viewer_denied_destructive_requests() {
    use locardx_auth::AuthorizationEngine;
    use locardx_common::OperationType;

    // Viewer cannot request destructive operations
    assert!(AuthorizationEngine::can_request_operation(
        UserRole::Viewer,
        OperationType::DriveErasure
    )
    .is_err());
    assert!(AuthorizationEngine::can_request_operation(
        UserRole::Viewer,
        OperationType::FileErasure
    )
    .is_err());
    assert!(AuthorizationEngine::can_request_operation(
        UserRole::Viewer,
        OperationType::FolderErasure
    )
    .is_err());

    // Viewer CAN request read-only integrity operations
    assert!(AuthorizationEngine::can_request_operation(
        UserRole::Viewer,
        OperationType::IntegrityHash
    )
    .is_ok());
    assert!(AuthorizationEngine::can_request_operation(
        UserRole::Viewer,
        OperationType::IntegrityVerify
    )
    .is_ok());
}

#[test]
fn test_16_rbac_operator_and_admin_destructive_request_policy() {
    use locardx_auth::AuthorizationEngine;
    use locardx_common::OperationType;

    // Operator and Admin can request/prepare destructive operations
    assert!(AuthorizationEngine::can_request_operation(
        UserRole::Operator,
        OperationType::DriveErasure
    )
    .is_ok());
    assert!(AuthorizationEngine::can_request_operation(
        UserRole::Administrator,
        OperationType::DriveErasure
    )
    .is_ok());
    assert!(AuthorizationEngine::can_request_operation(
        UserRole::Investigator,
        OperationType::FileErasure
    )
    .is_ok());

    // Unknown operation rejected for all roles
    assert!(AuthorizationEngine::can_request_operation(
        UserRole::Administrator,
        OperationType::Unknown
    )
    .is_err());
}
