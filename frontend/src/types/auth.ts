export type UserRole = 'Administrator' | 'Investigator' | 'Operator' | 'Viewer';

export type Permission =
  | 'CaseCreate'
  | 'CaseView'
  | 'CaseModify'
  | 'CaseClose'
  | 'CaseEvidenceAdd'
  | 'CaseCustodyRecord'
  | 'CaseReportGenerate'
  | 'AcquisitionViewSources'
  | 'AcquisitionStart'
  | 'AcquisitionCancel'
  | 'AcquisitionViewArtifacts'
  | 'RecoveryStart'
  | 'RecoveryViewResults'
  | 'RecoveryExportFiles'
  | 'RecoveryGenerateReport'
  | 'ErasureViewCapabilities'
  | 'ErasurePlan'
  | 'ErasureExecute'
  | 'ErasureGenerateReport'
  | 'AuditView'
  | 'AuditVerify'
  | 'UserCreate'
  | 'UserList'
  | 'UserView'
  | 'UserUpdate'
  | 'UserDisable'
  | 'UserChangeRole'
  | 'UserUnlock';

export interface PublicUser {
  user_id: string;
  username: string;
  role: UserRole;
  display_name?: string | null;
  enabled: boolean;
  created_at: string;
  updated_at?: string;
  last_login?: string | null;
  last_login_at?: string | null;
  metadata_json?: string;
}

export interface AuthSessionResponse {
  token: string;
  user: PublicUser;
  expires_at: number | string;
  permissions?: string[];
}

export interface SessionValidationResponse {
  is_valid: boolean;
  user?: PublicUser | null;
  expires_at?: number | null;
  time_remaining_seconds?: number | null;
  permissions: string[];
}

export interface AuthStatusResponse {
  is_first_run: boolean;
  initialized: boolean;
}

export interface InitAdminRequest {
  username: string;
  password: string;
  confirm_password?: string;
  display_name?: string;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface CreateUserRequest {
  username: string;
  password: string;
  role: UserRole;
  display_name?: string;
  metadata_json?: string;
}

export interface UpdateUserRequest {
  user_id: string;
  display_name?: string;
  metadata_json?: string;
}

export interface ChangeUserRoleRequest {
  user_id: string;
  new_role: UserRole;
}

export interface UnlockUserRequest {
  user_id: string;
}

export interface SetUserEnabledRequest {
  user_id: string;
  enabled: boolean;
}

export interface ChangePasswordRequest {
  current_password: string;
  new_password: string;
}
