import { useAuthStore } from '../stores/authStore';

export const useAuth = () => {
  const store = useAuthStore();
  return {
    user: store.currentUser,
    token: store.sessionToken,
    isAuthenticated: !!store.currentUser && !!store.sessionToken,
    isAdmin: store.currentUser?.role === 'Administrator',
    isFirstRun: store.isFirstRun,
    isLoading: store.isLoading,
    error: store.error,
    login: store.login,
    logout: store.logout,
    initAdmin: store.initAdmin,
    checkFirstRun: store.checkFirstRun,
    clearError: store.clearError,
  };
};

export default useAuth;
