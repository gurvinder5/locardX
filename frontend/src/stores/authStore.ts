import { create } from 'zustand';
import {
  PublicUser,
  LoginRequest,
  InitAdminRequest,
} from '../types/auth';
import * as authService from '../services/auth';

interface AuthState {
  currentUser: PublicUser | null;
  sessionToken: string | null;
  permissions: string[];
  isFirstRun: boolean | null;
  isLoading: boolean;
  error: string | null;

  checkFirstRun: () => Promise<void>;
  initAdmin: (req: InitAdminRequest) => Promise<void>;
  login: (req: LoginRequest) => Promise<void>;
  logout: () => Promise<void>;
  validateCurrentSession: () => Promise<boolean>;
  hasPermission: (permission: string) => boolean;
  clearError: () => void;
}

export function extractErrorMessage(err: unknown, fallback: string): string {
  if (typeof err === 'string') return err;
  if (err && typeof err === 'object') {
    if ('message' in err && typeof (err as { message: unknown }).message === 'string') {
      return (err as { message: string }).message;
    }
  }
  if (err instanceof Error) return err.message;
  return fallback;
}

export const useAuthStore = create<AuthState>((set, get) => ({
  currentUser: null,
  sessionToken: null,
  permissions: [],
  isFirstRun: null,
  isLoading: true,
  error: null,

  clearError: () => set({ error: null }),

  hasPermission: (permission: string) => {
    const { permissions, currentUser } = get();
    if (!currentUser) return false;
    if (currentUser.role === 'Administrator') return true;
    return permissions.includes(permission);
  },

  checkFirstRun: async () => {
    try {
      set({ isLoading: true, error: null });
      const status = await authService.getAuthenticationStatus();
      set({ isFirstRun: status.is_first_run, isLoading: false });
    } catch (err: unknown) {
      const msg = extractErrorMessage(err, 'Failed to verify system status');
      set({ error: msg, isLoading: false });
    }
  },

  validateCurrentSession: async () => {
    const token = get().sessionToken;
    if (!token) return false;
    try {
      const res = await authService.validateSession(token);
      if (res.is_valid && res.user) {
        set({
          currentUser: res.user,
          permissions: res.permissions,
        });
        return true;
      } else {
        set({
          currentUser: null,
          sessionToken: null,
          permissions: [],
        });
        return false;
      }
    } catch {
      set({
        currentUser: null,
        sessionToken: null,
        permissions: [],
      });
      return false;
    }
  },

  initAdmin: async (req: InitAdminRequest) => {
    try {
      set({ isLoading: true, error: null });
      await authService.initializeAdmin(req);
      // Immediately log in with the new admin credentials
      const session = await authService.login({
        username: req.username,
        password: req.password,
      });
      set({
        currentUser: session.user,
        sessionToken: session.token,
        permissions: session.permissions || [],
        isFirstRun: false,
        isLoading: false,
      });
    } catch (err: unknown) {
      const msg = extractErrorMessage(err, 'Failed to initialize administrator');
      if (
        msg.toLowerCase().includes('already been completed') ||
        msg.toLowerCase().includes('closed registration')
      ) {
        set({ isFirstRun: false, error: msg, isLoading: false });
      } else {
        set({ error: msg, isLoading: false });
      }
      throw err;
    }
  },

  login: async (req: LoginRequest) => {
    try {
      set({ isLoading: true, error: null });
      const session = await authService.login(req);
      set({
        currentUser: session.user,
        sessionToken: session.token,
        permissions: session.permissions || [],
        isLoading: false,
      });
    } catch (err: unknown) {
      const msg = extractErrorMessage(err, 'Invalid username or password.');
      set({ error: msg, isLoading: false });
      throw err;
    }
  },

  logout: async () => {
    const token = get().sessionToken;
    if (token) {
      try {
        await authService.logout(token);
      } catch (err) {
        console.warn('Backend logout notification failed:', err);
      }
    }
    set({
      currentUser: null,
      sessionToken: null,
      permissions: [],
      error: null,
    });
  },
}));
