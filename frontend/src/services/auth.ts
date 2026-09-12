import {
  AuthSessionResponse,
  ChangePasswordRequest,
  CreateUserRequest,
  InitAdminRequest,
  LoginRequest,
  PublicUser,
  SetUserEnabledRequest,
} from '../types/auth';

const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// Fallback in-memory state for standalone browser preview (Vite dev)
let mockFirstRun = true;
let mockUsers: PublicUser[] = [];
let mockSessions = new Map<string, PublicUser>();

export async function isFirstRun(): Promise<boolean> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<boolean>('is_first_run');
  }
  return mockFirstRun;
}

export async function initializeAdmin(payload: InitAdminRequest): Promise<PublicUser> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<PublicUser>('initialize_admin', { payload });
  }

  if (!mockFirstRun) {
    throw new Error('Initial setup already completed. Contact administrator.');
  }

  const user: PublicUser = {
    user_id: 'mock-admin-id',
    username: payload.username.trim(),
    role: 'Administrator',
    enabled: true,
    created_at: new Date().toISOString(),
    last_login: null,
  };
  mockUsers.push(user);
  mockFirstRun = false;
  return user;
}

export async function login(payload: LoginRequest): Promise<AuthSessionResponse> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<AuthSessionResponse>('login', { payload });
  }

  const user = mockUsers.find((u) => u.username === payload.username.trim());
  if (!user || !user.enabled) {
    throw new Error('Invalid username or password.');
  }

  const token = 'mock-session-token-' + Math.random().toString(36).substring(2);
  mockSessions.set(token, user);

  return {
    token,
    user,
    expires_at: new Date(Date.now() + 8 * 3600 * 1000).toISOString(),
  };
}

export async function logout(token: string): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('logout', { token });
    return;
  }
  mockSessions.delete(token);
}

export async function getCurrentUser(token: string): Promise<PublicUser> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<PublicUser>('get_current_user', { token });
  }
  const user = mockSessions.get(token);
  if (!user) {
    throw new Error('Session invalid or expired.');
  }
  return user;
}

export async function createUser(
  token: string,
  payload: CreateUserRequest
): Promise<PublicUser> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<PublicUser>('create_user', { token, payload });
  }

  const current = mockSessions.get(token);
  if (!current || current.role !== 'Administrator') {
    throw new Error('Access denied: Administrator privilege required.');
  }

  const trimmedUsername = payload.username.trim();
  if (mockUsers.some((u) => u.username.toLowerCase() === trimmedUsername.toLowerCase())) {
    throw new Error(`Username '${trimmedUsername}' is already in use.`);
  }

  const newUser: PublicUser = {
    user_id: 'mock-user-' + Math.random().toString(36).substring(2, 9),
    username: trimmedUsername,
    role: payload.role,
    enabled: true,
    created_at: new Date().toISOString(),
    last_login: null,
  };
  mockUsers.push(newUser);
  return newUser;
}

export async function setUserEnabled(
  token: string,
  payload: SetUserEnabledRequest
): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('set_user_enabled', { token, payload });
    return;
  }

  const current = mockSessions.get(token);
  if (!current || current.role !== 'Administrator') {
    throw new Error('Access denied: Administrator privilege required.');
  }

  const user = mockUsers.find((u) => u.user_id === payload.user_id);
  if (user) {
    user.enabled = payload.enabled;
  }
}

export async function listUsers(token: string): Promise<PublicUser[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<PublicUser[]>('list_users', { token });
  }

  const current = mockSessions.get(token);
  if (!current || current.role !== 'Administrator') {
    throw new Error('Access denied: Administrator privilege required.');
  }

  return [...mockUsers];
}

export async function changePassword(
  token: string,
  payload: ChangePasswordRequest
): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('change_password', { token, payload });
    return;
  }

  const current = mockSessions.get(token);
  if (!current) {
    throw new Error('Session invalid or expired.');
  }
}
