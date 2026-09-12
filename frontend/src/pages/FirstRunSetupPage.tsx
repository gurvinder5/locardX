import React, { useState, useMemo } from 'react';
import { ShieldCheck, Lock, User, AlertCircle, CheckCircle2, XCircle } from 'lucide-react';
import { useAuthStore } from '../stores/authStore';

export const FirstRunSetupPage: React.FC = () => {
  const { initAdmin, isLoading, error, clearError, checkFirstRun } = useAuthStore();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [localError, setLocalError] = useState<string | null>(null);

  const passwordValidation = useMemo(() => {
    return {
      minLength: password.length >= 10,
      hasUpper: /[A-Z]/.test(password),
      hasLower: /[a-z]/.test(password),
      hasDigit: /[0-9]/.test(password),
      hasSpecial: /[^A-Za-z0-9]/.test(password),
    };
  }, [password]);

  const isPasswordValid =
    passwordValidation.minLength &&
    passwordValidation.hasUpper &&
    passwordValidation.hasLower &&
    passwordValidation.hasDigit &&
    passwordValidation.hasSpecial;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLocalError(null);
    clearError();

    if (!username.trim()) {
      setLocalError('Administrator username is required.');
      return;
    }

    if (!isPasswordValid) {
      setLocalError('Password does not satisfy all complexity requirements.');
      return;
    }

    if (password !== confirmPassword) {
      setLocalError('Passwords do not match.');
      return;
    }

    try {
      await initAdmin({
        username: username.trim(),
        password,
        confirm_password: confirmPassword,
      });
    } catch {
      // Error is set in store
    }
  };

  const displayError = localError || error;

  return (
    <div className="min-h-screen bg-slate-100 flex flex-col justify-center items-center p-4 font-sans text-slate-800 antialiased">
      <div className="max-w-md w-full bg-white border border-slate-200 rounded-lg p-7 shadow-xs">
        {/* Header */}
        <div className="flex flex-col items-center text-center mb-6">
          <div className="w-12 h-12 rounded-lg bg-slate-100 border border-slate-200 flex items-center justify-center mb-3 text-slate-700">
            <ShieldCheck className="w-6 h-6 text-slate-800" />
          </div>
          <h1 className="text-xl font-bold tracking-tight text-slate-900">
            LOCARD<span className="text-sky-600">X</span>
          </h1>
          <p className="text-[11px] uppercase tracking-wider text-slate-500 font-semibold mt-0.5">
            Initial Workstation Provisioning
          </p>
          <p className="text-xs text-slate-500 mt-2 text-center leading-relaxed">
            Closed registration requires creating the root Administrator account. First-run setup is permanently locked once completed.
          </p>
        </div>

        {displayError && (
          <div className="mb-4 p-2.5 rounded bg-rose-50 border border-rose-200 text-rose-800 text-xs flex items-start space-x-2">
            <AlertCircle className="w-4 h-4 text-rose-600 flex-shrink-0 mt-0.5" />
            <span>{displayError}</span>
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-3.5">
          {/* Username */}
          <div>
            <label className="block text-xs font-medium text-slate-700 mb-1">
              Administrator Username
            </label>
            <div className="relative">
              <span className="absolute inset-y-0 left-0 pl-2.5 flex items-center text-slate-400 pointer-events-none">
                <User className="w-3.5 h-3.5" />
              </span>
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                required
                autoComplete="username"
                className="w-full pl-8 pr-3 py-1.5 bg-slate-50 border border-slate-300 rounded text-xs text-slate-900 placeholder-slate-400 focus:outline-none focus:border-slate-500 focus:bg-white transition-colors"
                placeholder="admin"
                disabled={isLoading}
              />
            </div>
          </div>

          {/* Password */}
          <div>
            <label className="block text-xs font-medium text-slate-700 mb-1">
              Master Password
            </label>
            <div className="relative">
              <span className="absolute inset-y-0 left-0 pl-2.5 flex items-center text-slate-400 pointer-events-none">
                <Lock className="w-3.5 h-3.5" />
              </span>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
                autoComplete="new-password"
                className="w-full pl-8 pr-3 py-1.5 bg-slate-50 border border-slate-300 rounded text-xs text-slate-900 placeholder-slate-400 focus:outline-none focus:border-slate-500 focus:bg-white transition-colors"
                placeholder="••••••••••••"
                disabled={isLoading}
              />
            </div>
          </div>

          {/* Confirm Password */}
          <div>
            <label className="block text-xs font-medium text-slate-700 mb-1">
              Confirm Master Password
            </label>
            <div className="relative">
              <span className="absolute inset-y-0 left-0 pl-2.5 flex items-center text-slate-400 pointer-events-none">
                <Lock className="w-3.5 h-3.5" />
              </span>
              <input
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                required
                autoComplete="new-password"
                className="w-full pl-8 pr-3 py-1.5 bg-slate-50 border border-slate-300 rounded text-xs text-slate-900 placeholder-slate-400 focus:outline-none focus:border-slate-500 focus:bg-white transition-colors"
                placeholder="••••••••••••"
                disabled={isLoading}
              />
            </div>
          </div>

          {/* Password Strength Checklist */}
          <div className="bg-slate-50 border border-slate-200 rounded p-2.5 space-y-1 text-xs">
            <span className="font-semibold text-slate-600 block mb-1 text-[10px] uppercase tracking-wider">
              Argon2id Complexity Checklist:
            </span>
            <div className="grid grid-cols-1 gap-1 text-[11px] text-slate-500">
              <div className="flex items-center space-x-1.5">
                {passwordValidation.minLength ? (
                  <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600" />
                ) : (
                  <XCircle className="w-3.5 h-3.5 text-slate-300" />
                )}
                <span className={passwordValidation.minLength ? 'text-slate-800 font-medium' : ''}>
                  At least 10 characters
                </span>
              </div>
              <div className="flex items-center space-x-1.5">
                {passwordValidation.hasUpper ? (
                  <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600" />
                ) : (
                  <XCircle className="w-3.5 h-3.5 text-slate-300" />
                )}
                <span className={passwordValidation.hasUpper ? 'text-slate-800 font-medium' : ''}>
                  Uppercase letter (A-Z)
                </span>
              </div>
              <div className="flex items-center space-x-1.5">
                {passwordValidation.hasLower ? (
                  <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600" />
                ) : (
                  <XCircle className="w-3.5 h-3.5 text-slate-300" />
                )}
                <span className={passwordValidation.hasLower ? 'text-slate-800 font-medium' : ''}>
                  Lowercase letter (a-z)
                </span>
              </div>
              <div className="flex items-center space-x-1.5">
                {passwordValidation.hasDigit ? (
                  <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600" />
                ) : (
                  <XCircle className="w-3.5 h-3.5 text-slate-300" />
                )}
                <span className={passwordValidation.hasDigit ? 'text-slate-800 font-medium' : ''}>
                  Numerical digit (0-9)
                </span>
              </div>
              <div className="flex items-center space-x-1.5">
                {passwordValidation.hasSpecial ? (
                  <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600" />
                ) : (
                  <XCircle className="w-3.5 h-3.5 text-slate-300" />
                )}
                <span className={passwordValidation.hasSpecial ? 'text-slate-800 font-medium' : ''}>
                  Special symbol character
                </span>
              </div>
            </div>
          </div>

          <button
            type="submit"
            disabled={isLoading || !isPasswordValid}
            className="w-full mt-2 py-2 px-4 bg-slate-900 hover:bg-slate-800 disabled:bg-slate-200 disabled:text-slate-400 disabled:cursor-not-allowed text-white font-medium rounded text-xs shadow-2xs transition-colors flex justify-center items-center"
          >
            {isLoading ? 'Initializing...' : 'Initialize Administrator & Launch'}
          </button>
        </form>

        <div className="mt-5 border-t border-slate-100 pt-3 text-center space-y-2">
          <p className="text-[10px] text-slate-400">
            Backend integrity interlock: First-run setup permanently locks upon creation.
          </p>
          <div>
            <button
              type="button"
              onClick={() => checkFirstRun()}
              className="text-xs text-sky-600 hover:text-sky-700 font-medium transition-colors"
            >
              Administrator already provisioned? Switch to Sign In →
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default FirstRunSetupPage;
