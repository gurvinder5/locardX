use locardx_audit::AuditService;
use locardx_auth::{
    models::{
        ChangeUserRoleRequest, CreateUserRequest, InitAdminRequest, LoginRequest, Permission,
        UserRole,
    },
    AuthService, AuthorizationEngine,
};
use locardx_database::Database;
use std::sync::Arc;

fn setup_test_auth_service() -> (AuthService, Arc<Database>, Arc<AuditService>) {
    let db =
        Arc::new(Database::open(":memory:").expect("Failed to create in-memory test database"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let auth = AuthService::new(Arc::clone(&db), Arc::clone(&audit));
    (auth, db, audit)
}

// =========================================================================
// 1. AUTHENTICATION TESTS (Points 1 to 10)
// =========================================================================

#[test]
fn test_01_authentication_initializes_successfully() {
    let (auth, _, _) = setup_test_auth_service();
    assert!(auth.is_first_run().expect("is_first_run query failed"));
}

#[test]
fn test_02_administrator_setup_requires_valid_credentials() {
    let (auth, _, _) = setup_test_auth_service();

    // Rejects weak password (<10 chars, missing special)
    let weak_req = InitAdminRequest {
        username: "admin".to_string(),
        password: "short".to_string(),
        confirm_password: "short".to_string(),
        display_name: None,
    };
    assert!(auth.initialize_admin(weak_req).is_err());

    // Rejects password confirmation mismatch
    let mismatch_req = InitAdminRequest {
        username: "admin".to_string(),
        password: "StrongPassword123!".to_string(),
        confirm_password: "MismatchPassword123!".to_string(),
        display_name: None,
    };
    assert!(auth.initialize_admin(mismatch_req).is_err());

    // Valid setup succeeds
    let valid_req = InitAdminRequest::new("sysadmin", "StrongPassword123!");
    let admin = auth.initialize_admin(valid_req).expect("Setup failed");
    assert_eq!(admin.username, "sysadmin");
    assert_eq!(admin.role, UserRole::Administrator);
    assert!(!auth.is_first_run().expect("First run check failed"));
}

#[test]
fn test_03_plaintext_password_is_never_stored() {
    let (auth, db, _) = setup_test_auth_service();
    let raw_pwd = "SecretPlaintextPassword123!";

    auth.initialize_admin(InitAdminRequest::new("crypto_admin", raw_pwd))
        .expect("Init failed");

    let stored_hash: String = db
        .with_conn(|conn| {
            let mut stmt =
                conn.prepare("SELECT password_hash FROM users WHERE username = 'crypto_admin'")?;
            stmt.query_row([], |r| r.get(0))
        })
        .expect("DB query failed");

    assert_ne!(stored_hash, raw_pwd);
    assert!(stored_hash.starts_with("$argon2id$"));
}

#[test]
fn test_04_password_hash_verification_succeeds() {
    let (auth, _, _) = setup_test_auth_service();
    let pwd = "SuperSecretPassword123!";

    auth.initialize_admin(InitAdminRequest::new("examiner_lead", pwd))
        .expect("Init failed");

    let login_res = auth
        .login(LoginRequest {
            username: "examiner_lead".to_string(),
            password: pwd.to_string(),
        })
        .expect("Login failed with correct password");

    assert_eq!(login_res.user.username, "examiner_lead");
    assert!(!login_res.token.is_empty());
    assert!(login_res.expires_at > 0);
}

#[test]
fn test_05_incorrect_password_fails_with_generic_message() {
    let (auth, _, _) = setup_test_auth_service();
    auth.initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .expect("Init failed");

    let result = auth.login(LoginRequest {
        username: "admin".to_string(),
        password: "WrongPassword123!".to_string(),
    });

    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert_eq!(
        msg,
        "Security policy violation: Invalid username or password."
    );
}

#[test]
fn test_06_unknown_user_returns_generic_failure() {
    let (auth, _, _) = setup_test_auth_service();
    auth.initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .expect("Init failed");

    let res_known = auth.login(LoginRequest {
        username: "admin".to_string(),
        password: "WrongPassword123!".to_string(),
    });

    let res_unknown = auth.login(LoginRequest {
        username: "ghost_nonexistent_user".to_string(),
        password: "WrongPassword123!".to_string(),
    });

    assert_eq!(
        res_known.unwrap_err().to_string(),
        res_unknown.unwrap_err().to_string()
    );
}

#[test]
fn test_07_disabled_user_cannot_login() {
    let (auth, _, _) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Admin setup failed");

    let operator = auth
        .create_user(
            &admin_session.token,
            CreateUserRequest::new("op1", "OperatorPass123!", UserRole::Operator),
        )
        .expect("Create user failed");

    auth.set_user_enabled(&admin_session.token, &operator.user_id, false)
        .expect("Disable user failed");

    let login_attempt = auth.login(LoginRequest {
        username: "op1".to_string(),
        password: "OperatorPass123!".to_string(),
    });

    assert!(login_attempt.is_err());
    let err = login_attempt.unwrap_err().to_string();
    assert!(err.contains("Account is disabled"));
}

#[test]
fn test_08_logout_invalidates_session() {
    let (auth, _, _) = setup_test_auth_service();
    let session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Login failed");

    assert!(auth.get_current_user(&session.token).is_ok());

    auth.logout(&session.token).expect("Logout failed");

    assert!(auth.get_current_user(&session.token).is_err());
}

#[test]
fn test_09_expired_session_is_rejected() {
    let (auth, db, _) = setup_test_auth_service();
    let session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Login failed");

    // Artificially expire the session
    let past = 1000;
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE sessions SET expires_at = ?1 WHERE token = ?2",
            rusqlite::params![past, session.token],
        )?;
        Ok(())
    })
    .expect("DB update failed");

    let result = auth.get_current_user(&session.token);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("expired"));
}

#[test]
fn test_10_session_validation_succeeds_for_valid_session() {
    let (auth, _, _) = setup_test_auth_service();
    let session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Login failed");

    let val = auth
        .validate_session(&session.token)
        .expect("validate_session call failed");

    assert!(val.is_valid);
    assert!(val.user.is_some());
    assert!(val.time_remaining_seconds.unwrap_or(0) > 0);
    assert!(!val.permissions.is_empty());
}

// =========================================================================
// 2. REGISTRATION & ADMINISTRATION TESTS (Points 11 to 17)
// =========================================================================

#[test]
fn test_11_public_registration_is_rejected() {
    let (auth, _, _) = setup_test_auth_service();
    let _ = auth.initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"));

    // Attempt second setup
    let req2 = InitAdminRequest::new("intruder", "IntruderPassword123!");
    let res2 = auth.initialize_admin(req2);
    assert!(res2.is_err(), "Second first-run setup must be rejected");

    // Attempt unauthenticated user creation
    let res3 = auth.create_user(
        "invalid_fake_token_12345",
        CreateUserRequest::new("unauth", "UnauthPassword123!", UserRole::Viewer),
    );
    assert!(res3.is_err(), "Public user creation must be rejected");
}

#[test]
fn test_12_authorized_administrator_can_create_a_user() {
    let (auth, _, _) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    let user = auth
        .create_user(
            &admin_session.token,
            CreateUserRequest::new("investigator1", "InvPassword123!", UserRole::Investigator),
        )
        .expect("Create user failed");

    assert_eq!(user.username, "investigator1");
    assert_eq!(user.role, UserRole::Investigator);
}

#[test]
fn test_13_unauthorized_user_cannot_create_a_user() {
    let (auth, _, _) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    auth.create_user(
        &admin_session.token,
        CreateUserRequest::new("inv1", "InvPassword123!", UserRole::Investigator),
    )
    .expect("Create user failed");

    let inv_session = auth
        .login(LoginRequest {
            username: "inv1".to_string(),
            password: "InvPassword123!".to_string(),
        })
        .expect("Login failed");

    // Investigator tries to create another user
    let res = auth.create_user(
        &inv_session.token,
        CreateUserRequest::new("peer", "PeerPassword123!", UserRole::Operator),
    );
    assert!(res.is_err(), "Non-admin cannot create users");
}

#[test]
fn test_14_duplicate_username_is_rejected() {
    let (auth, _, _) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    auth.create_user(
        &admin_session.token,
        CreateUserRequest::new("alice", "AlicePassword123!", UserRole::Investigator),
    )
    .expect("Create user failed");

    let dup = auth.create_user(
        &admin_session.token,
        CreateUserRequest::new("alice", "AnotherPassword123!", UserRole::Operator),
    );
    assert!(dup.is_err(), "Duplicate username must be rejected");
}

#[test]
fn test_15_user_can_be_disabled() {
    let (auth, _, _) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    let user = auth
        .create_user(
            &admin_session.token,
            CreateUserRequest::new("bob", "BobPassword123!", UserRole::Operator),
        )
        .expect("Create user failed");

    auth.set_user_enabled(&admin_session.token, &user.user_id, false)
        .expect("Disable failed");

    let fetched = auth
        .get_user(&admin_session.token, &user.user_id)
        .expect("Fetch failed");
    assert!(!fetched.enabled);
}

#[test]
fn test_16_disabled_account_cannot_authenticate() {
    let (auth, _, _) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    let user = auth
        .create_user(
            &admin_session.token,
            CreateUserRequest::new("carol", "CarolPassword123!", UserRole::Viewer),
        )
        .expect("Create user failed");

    auth.set_user_enabled(&admin_session.token, &user.user_id, false)
        .expect("Disable failed");

    let res = auth.login(LoginRequest {
        username: "carol".to_string(),
        password: "CarolPassword123!".to_string(),
    });
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("Account is disabled"));
}

#[test]
fn test_17_role_changes_require_authorization() {
    let (auth, _, _) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    let user = auth
        .create_user(
            &admin_session.token,
            CreateUserRequest::new("dave", "DavePassword123!", UserRole::Operator),
        )
        .expect("Create user failed");

    // Admin promotes dave to Investigator
    let updated = auth
        .change_user_role(
            &admin_session.token,
            ChangeUserRoleRequest {
                user_id: user.user_id.clone(),
                new_role: UserRole::Investigator,
            },
        )
        .expect("Role change failed");
    assert_eq!(updated.role, UserRole::Investigator);

    // Sole admin demotion is blocked
    let demote_admin = auth.change_user_role(
        &admin_session.token,
        ChangeUserRoleRequest {
            user_id: admin_session.user.user_id,
            new_role: UserRole::Operator,
        },
    );
    assert!(demote_admin.is_err(), "Demoting sole admin must be blocked");
}

// =========================================================================
// 3. AUTHORIZATION TESTS (Points 18 to 26)
// =========================================================================

#[test]
fn test_18_unauthenticated_user_cannot_access_protected_commands() {
    let (auth, _, _) = setup_test_auth_service();
    let fake_token = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

    let res = auth.authorize_permission(fake_token, Permission::CaseCreate);
    assert!(res.is_err(), "Unauthenticated request must fail");
}

#[test]
fn test_19_unauthorized_user_cannot_access_protected_commands() {
    let (auth, _, _) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    auth.create_user(
        &admin_session.token,
        CreateUserRequest::new("viewer_user", "ViewerPassword123!", UserRole::Viewer),
    )
    .expect("Create user failed");

    let viewer_session = auth
        .login(LoginRequest {
            username: "viewer_user".to_string(),
            password: "ViewerPassword123!".to_string(),
        })
        .expect("Login failed");

    // Viewer tries to create case
    let res = auth.authorize_permission(&viewer_session.token, Permission::CaseCreate);
    assert!(
        res.is_err(),
        "Viewer must not possess CaseCreate permission"
    );
}

#[test]
fn test_20_authorized_user_can_perform_permitted_action() {
    let (auth, _, _) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    auth.create_user(
        &admin_session.token,
        CreateUserRequest::new("lead_inv", "InvPassword123!", UserRole::Investigator),
    )
    .expect("Create user failed");

    let inv_session = auth
        .login(LoginRequest {
            username: "lead_inv".to_string(),
            password: "InvPassword123!".to_string(),
        })
        .expect("Login failed");

    let user = auth
        .authorize_permission(&inv_session.token, Permission::CaseCreate)
        .expect("Authorized user must succeed");
    assert_eq!(user.username, "lead_inv");
}

#[test]
fn test_21_erasure_authorization_is_enforced() {
    assert!(AuthorizationEngine::has_permission(
        UserRole::Administrator,
        Permission::ErasureExecute
    ));
    assert!(AuthorizationEngine::has_permission(
        UserRole::Operator,
        Permission::ErasureExecute
    ));
    assert!(!AuthorizationEngine::has_permission(
        UserRole::Investigator,
        Permission::ErasureExecute
    ));
    assert!(!AuthorizationEngine::has_permission(
        UserRole::Viewer,
        Permission::ErasureExecute
    ));
}

#[test]
fn test_22_acquisition_authorization_is_enforced() {
    assert!(AuthorizationEngine::has_permission(
        UserRole::Administrator,
        Permission::AcquisitionStart
    ));
    assert!(AuthorizationEngine::has_permission(
        UserRole::Investigator,
        Permission::AcquisitionStart
    ));
    assert!(AuthorizationEngine::has_permission(
        UserRole::Operator,
        Permission::AcquisitionStart
    ));
    assert!(!AuthorizationEngine::has_permission(
        UserRole::Viewer,
        Permission::AcquisitionStart
    ));
}

#[test]
fn test_23_recovery_authorization_is_enforced() {
    assert!(AuthorizationEngine::has_permission(
        UserRole::Administrator,
        Permission::RecoveryStart
    ));
    assert!(AuthorizationEngine::has_permission(
        UserRole::Investigator,
        Permission::RecoveryStart
    ));
    assert!(AuthorizationEngine::has_permission(
        UserRole::Operator,
        Permission::RecoveryStart
    ));
    assert!(!AuthorizationEngine::has_permission(
        UserRole::Viewer,
        Permission::RecoveryStart
    ));
}

#[test]
fn test_24_case_authorization_is_enforced() {
    assert!(AuthorizationEngine::has_permission(
        UserRole::Administrator,
        Permission::CaseCreate
    ));
    assert!(AuthorizationEngine::has_permission(
        UserRole::Investigator,
        Permission::CaseCreate
    ));
    assert!(!AuthorizationEngine::has_permission(
        UserRole::Operator,
        Permission::CaseCreate
    ));
    assert!(!AuthorizationEngine::has_permission(
        UserRole::Viewer,
        Permission::CaseCreate
    ));
}

#[test]
fn test_25_user_management_authorization_is_enforced() {
    assert!(AuthorizationEngine::has_permission(
        UserRole::Administrator,
        Permission::UserCreate
    ));
    assert!(!AuthorizationEngine::has_permission(
        UserRole::Investigator,
        Permission::UserCreate
    ));
    assert!(!AuthorizationEngine::has_permission(
        UserRole::Operator,
        Permission::UserCreate
    ));
    assert!(!AuthorizationEngine::has_permission(
        UserRole::Viewer,
        Permission::UserCreate
    ));
}

#[test]
fn test_26_audit_access_authorization_is_enforced() {
    assert!(AuthorizationEngine::has_permission(
        UserRole::Administrator,
        Permission::AuditVerify
    ));
    assert!(AuthorizationEngine::has_permission(
        UserRole::Investigator,
        Permission::AuditVerify
    ));
    assert!(!AuthorizationEngine::has_permission(
        UserRole::Operator,
        Permission::AuditVerify
    ));
    assert!(!AuthorizationEngine::has_permission(
        UserRole::Viewer,
        Permission::AuditVerify
    ));

    // All roles can view audit
    assert!(AuthorizationEngine::has_permission(
        UserRole::Viewer,
        Permission::AuditView
    ));
}

// =========================================================================
// 4. AUDIT INTEGRATION TESTS (Points 27 to 33)
// =========================================================================

#[test]
fn test_27_successful_login_generates_appropriate_audit_event() {
    let (auth, _, audit) = setup_test_auth_service();
    auth.initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .expect("Init failed");

    auth.login(LoginRequest {
        username: "admin".to_string(),
        password: "AdminPassword123!".to_string(),
    })
    .expect("Login failed");

    let events = audit.list_events(10).expect("List audit failed");
    assert!(events.iter().any(|e| e.event_type == "LOGIN_SUCCESS"));
}

#[test]
fn test_28_failed_login_generates_appropriate_audit_event() {
    let (auth, _, audit) = setup_test_auth_service();
    auth.initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .expect("Init failed");

    let _ = auth.login(LoginRequest {
        username: "admin".to_string(),
        password: "WrongPassword123!".to_string(),
    });

    let events = audit.list_events(10).expect("List audit failed");
    assert!(events.iter().any(|e| e.event_type == "LOGIN_FAILURE"));
}

#[test]
fn test_29_logout_generates_audit_event() {
    let (auth, _, audit) = setup_test_auth_service();
    let session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    auth.logout(&session.token).expect("Logout failed");

    let events = audit.list_events(10).expect("List audit failed");
    assert!(events.iter().any(|e| e.event_type == "LOGOUT"));
}

#[test]
fn test_30_authorization_denial_generates_audit_event() {
    let (auth, _, audit) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    auth.create_user(
        &admin_session.token,
        CreateUserRequest::new("viewer1", "ViewerPassword123!", UserRole::Viewer),
    )
    .expect("Create user failed");

    let viewer_session = auth
        .login(LoginRequest {
            username: "viewer1".to_string(),
            password: "ViewerPassword123!".to_string(),
        })
        .expect("Login failed");

    let _ = auth.authorize_permission(&viewer_session.token, Permission::UserCreate);

    let events = audit.list_events(10).expect("List audit failed");
    assert!(events
        .iter()
        .any(|e| e.event_type == "AUTHORIZATION_DENIED"));
}

#[test]
fn test_31_user_creation_generates_audit_event() {
    let (auth, _, audit) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    auth.create_user(
        &admin_session.token,
        CreateUserRequest::new("newbie", "NewbiePassword123!", UserRole::Operator),
    )
    .expect("Create user failed");

    let events = audit.list_events(10).expect("List audit failed");
    assert!(events.iter().any(|e| e.event_type == "USER_CREATED"));
}

#[test]
fn test_32_role_change_generates_audit_event() {
    let (auth, _, audit) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    let user = auth
        .create_user(
            &admin_session.token,
            CreateUserRequest::new("promotee", "PromoteePassword123!", UserRole::Operator),
        )
        .expect("Create user failed");

    auth.change_user_role(
        &admin_session.token,
        ChangeUserRoleRequest {
            user_id: user.user_id,
            new_role: UserRole::Investigator,
        },
    )
    .expect("Change role failed");

    let events = audit.list_events(10).expect("List audit failed");
    assert!(events.iter().any(|e| e.event_type == "USER_ROLE_CHANGED"));
}

#[test]
fn test_33_audit_chain_remains_valid() {
    let (auth, _, audit) = setup_test_auth_service();
    let session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    let user = auth
        .create_user(
            &session.token,
            CreateUserRequest::new("user_audited", "UserPassword123!", UserRole::Operator),
        )
        .expect("Create user failed");

    let _ = auth.change_user_role(
        &session.token,
        ChangeUserRoleRequest {
            user_id: user.user_id,
            new_role: UserRole::Investigator,
        },
    );

    let ver = audit.verify_chain().expect("Chain verify failed");
    assert!(ver.is_valid);
    assert!(ver.total_events >= 3);
}

// =========================================================================
// 5. SECURITY INVARIANT TESTS (Points 34 to 40)
// =========================================================================

#[test]
fn test_34_passwords_never_appear_in_logs() {
    let (auth, _, audit) = setup_test_auth_service();
    let secret = "SuperConfidentialPass999!";

    auth.initialize_admin(InitAdminRequest::new("secret_admin", secret))
        .expect("Init failed");

    let events = audit.list_events(100).expect("List failed");
    for event in events {
        assert!(!event.details.contains(secret));
        assert!(!event.details.contains("$argon2id$"));
    }
}

#[test]
fn test_35_passwords_never_appear_in_reports() {
    let (auth, _, _) = setup_test_auth_service();
    let user = auth
        .initialize_admin(InitAdminRequest::new("admin", "Password123!"))
        .expect("Init failed");

    let serialized = serde_json::to_string(&user).expect("Serialize failed");
    assert!(!serialized.contains("password_hash"));
    assert!(!serialized.contains("$argon2id$"));
}

#[test]
fn test_36_frontend_cannot_spoof_authenticated_identity() {
    let (auth, _, _) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    let viewer = auth
        .create_user(
            &admin_session.token,
            CreateUserRequest::new("viewer_spoof", "ViewerPassword123!", UserRole::Viewer),
        )
        .expect("Create user failed");

    let viewer_session = auth
        .login(LoginRequest {
            username: "viewer_spoof".to_string(),
            password: "ViewerPassword123!".to_string(),
        })
        .expect("Login failed");

    // The viewer attempts to authorize as Administrator, but token resolves strictly to Viewer in DB
    let auth_result = auth.authorize_permission(&viewer_session.token, Permission::UserCreate);
    assert!(auth_result.is_err());
    assert_ne!(viewer.role, UserRole::Administrator);
}

#[test]
fn test_37_frontend_cannot_bypass_backend_authorization() {
    let (auth, _, _) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    auth.create_user(
        &admin_session.token,
        CreateUserRequest::new(
            "operator_bypass",
            "OperatorPassword123!",
            UserRole::Operator,
        ),
    )
    .expect("Create user failed");

    let op_session = auth
        .login(LoginRequest {
            username: "operator_bypass".to_string(),
            password: "OperatorPassword123!".to_string(),
        })
        .expect("Login failed");

    // Even if frontend UI showed "Create User", backend rejects it
    let res = auth.create_user(
        &op_session.token,
        CreateUserRequest::new("hacker", "HackerPass123!", UserRole::Administrator),
    );
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("Access denied"));
}

#[test]
fn test_38_session_token_cannot_be_reused_after_logout() {
    let (auth, _, _) = setup_test_auth_service();
    let session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    let token = session.token.clone();
    auth.logout(&token).expect("Logout failed");

    let res = auth.validate_session(&token).expect("Validate failed");
    assert!(!res.is_valid);

    let auth_res = auth.authorize_permission(&token, Permission::CaseView);
    assert!(auth_res.is_err());
}

#[test]
fn test_39_case_access_control_and_isolation() {
    let (auth, _, _) = setup_test_auth_service();
    let admin_session = auth
        .initialize_admin(InitAdminRequest::new("admin", "AdminPassword123!"))
        .and_then(|_| {
            auth.login(LoginRequest {
                username: "admin".to_string(),
                password: "AdminPassword123!".to_string(),
            })
        })
        .expect("Setup failed");

    auth.create_user(
        &admin_session.token,
        CreateUserRequest::new("inv_case", "InvPassword123!", UserRole::Investigator),
    )
    .expect("Create user failed");

    auth.create_user(
        &admin_session.token,
        CreateUserRequest::new("viewer_case", "ViewerPassword123!", UserRole::Viewer),
    )
    .expect("Create user failed");

    let inv_session = auth
        .login(LoginRequest {
            username: "inv_case".to_string(),
            password: "InvPassword123!".to_string(),
        })
        .expect("Login failed");

    let viewer_session = auth
        .login(LoginRequest {
            username: "viewer_case".to_string(),
            password: "ViewerPassword123!".to_string(),
        })
        .expect("Login failed");

    // Investigator can create and modify cases
    assert!(auth
        .authorize_permission(&inv_session.token, Permission::CaseCreate)
        .is_ok());
    assert!(auth
        .authorize_permission(&inv_session.token, Permission::CaseModify)
        .is_ok());
    assert!(auth
        .authorize_permission(&inv_session.token, Permission::CaseClose)
        .is_ok());

    // Viewer cannot mutate cases
    assert!(auth
        .authorize_permission(&viewer_session.token, Permission::CaseCreate)
        .is_err());
    assert!(auth
        .authorize_permission(&viewer_session.token, Permission::CaseModify)
        .is_err());
    assert!(auth
        .authorize_permission(&viewer_session.token, Permission::CaseClose)
        .is_err());
}

#[test]
fn test_40_existing_destructive_erasure_safety_tests_remain_intact() {
    use locardx_security::SecurityEngine;

    // Direct OS boot target C:\ is unconditionally blocked regardless of role
    let c_drive_check = SecurityEngine::validate_target_safety("C:\\");
    assert!(c_drive_check.is_err());
    let err_msg = c_drive_check.unwrap_err().to_string();
    assert!(err_msg.contains("System root partition cannot be modified"));

    let unix_root_check = SecurityEngine::validate_target_safety("/");
    assert!(unix_root_check.is_err());

    let safe_external = SecurityEngine::validate_target_safety("\\\\.\\PhysicalDrive2");
    assert!(safe_external.is_ok());
}
