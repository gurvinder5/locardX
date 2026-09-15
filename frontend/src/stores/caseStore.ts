import { create } from 'zustand';
import { Case, CreateCaseRequest } from '../types/case';
import * as caseService from '../services/case';

interface CaseState {
  activeCase: Case | null;
  cases: Case[];
  isLoading: boolean;
  error: string | null;

  loadActiveCase: () => Promise<Case | null>;
  setActiveCase: (caseId: string) => Promise<Case>;
  clearActiveCase: () => Promise<void>;
  loadCases: () => Promise<void>;
  createCase: (req: CreateCaseRequest, sessionToken: string) => Promise<Case>;
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

export const useCaseStore = create<CaseState>((set) => ({
  activeCase: null,
  cases: [],
  isLoading: false,
  error: null,

  clearError: () => set({ error: null }),

  loadActiveCase: async () => {
    try {
      const active = await caseService.getActiveCase();
      set({ activeCase: active });
      return active;
    } catch (err) {
      console.warn('Failed to load active case from backend:', err);
      return null;
    }
  },

  setActiveCase: async (caseId: string) => {
    set({ isLoading: true, error: null });
    try {
      const caseObj = await caseService.setActiveCase(caseId);
      set({ activeCase: caseObj, isLoading: false });
      return caseObj;
    } catch (err: unknown) {
      const msg = extractErrorMessage(err, 'Failed to activate case');
      set({ error: msg, isLoading: false });
      throw err;
    }
  },

  clearActiveCase: async () => {
    set({ isLoading: true, error: null });
    try {
      await caseService.clearActiveCase();
      set({ activeCase: null, isLoading: false });
    } catch (err: unknown) {
      const msg = extractErrorMessage(err, 'Failed to clear active case');
      set({ error: msg, isLoading: false });
      throw err;
    }
  },

  loadCases: async () => {
    try {
      const res = await caseService.listCases(undefined, 100, 0);
      set({ cases: res.cases });
    } catch (err) {
      console.warn('Failed to load cases:', err);
    }
  },

  createCase: async (req: CreateCaseRequest, sessionToken: string) => {
    set({ isLoading: true, error: null });
    try {
      const newCase = await caseService.createCase(sessionToken, req);
      // Also automatically activate this newly created case
      try {
        const activated = await caseService.setActiveCase(newCase.case_id);
        set((state) => ({
          cases: [newCase, ...state.cases],
          activeCase: activated,
          isLoading: false,
        }));
        return activated;
      } catch {
        set((state) => ({
          cases: [newCase, ...state.cases],
          isLoading: false,
        }));
        return newCase;
      }
    } catch (err: unknown) {
      const msg = extractErrorMessage(err, 'Failed to create case');
      set({ error: msg, isLoading: false });
      throw err;
    }
  },
}));
