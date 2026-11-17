import { create } from 'zustand';

export type AppTheme = 'dark' | 'light';

interface AppState {
  theme: AppTheme;
  sidebarCollapsed: boolean;
  toggleTheme: () => void;
  setTheme: (theme: AppTheme) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
}

const getInitialTheme = (): AppTheme => {
  if (typeof window === 'undefined') return 'dark';
  const saved = localStorage.getItem('locardx_theme');
  if (saved === 'dark' || saved === 'light') return saved;
  return 'dark';
};

export const useAppStore = create<AppState>((set) => ({
  theme: getInitialTheme(),
  sidebarCollapsed: typeof window !== 'undefined' && localStorage.getItem('locardx_sidebar_collapsed') === 'true',
  toggleTheme: () =>
    set((state) => {
      const nextTheme: AppTheme = state.theme === 'dark' ? 'light' : 'dark';
      localStorage.setItem('locardx_theme', nextTheme);
      if (typeof document !== 'undefined') {
        if (nextTheme === 'dark') {
          document.documentElement.classList.add('dark');
        } else {
          document.documentElement.classList.remove('dark');
        }
      }
      return { theme: nextTheme };
    }),
  setTheme: (theme) => {
    localStorage.setItem('locardx_theme', theme);
    if (typeof document !== 'undefined') {
      if (theme === 'dark') {
        document.documentElement.classList.add('dark');
      } else {
        document.documentElement.classList.remove('dark');
      }
    }
    set({ theme });
  },
  toggleSidebar: () =>
    set((state) => {
      const next = !state.sidebarCollapsed;
      localStorage.setItem('locardx_sidebar_collapsed', String(next));
      return { sidebarCollapsed: next };
    }),
  setSidebarCollapsed: (collapsed) => {
    localStorage.setItem('locardx_sidebar_collapsed', String(collapsed));
    set({ sidebarCollapsed: collapsed });
  },
}));
