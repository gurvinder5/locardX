export type UserRole = 'Administrator' | 'Investigator' | 'Operator' | 'Viewer';

export interface PublicUser {
  user_id: string;
  username: string;
  role: UserRole;
  enabled: boolean;
  created_at: string;
  last_login: string | null;
}

export interface AuthSessionResponse {
  token: string;
  user: PublicUser;
  expires_at: string;
}

export interface InitAdminRequest {
  username: string;
  password: string;
  confirm_password?: string;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface CreateUserRequest {
  username: string;
  password: string;
  role: UserRole;
}

export interface SetUserEnabledRequest {
  user_id: string;
  enabled: boolean;
}

export interface ChangePasswordRequest {
  current_password: string;
  new_password: string;
}
