import React, { useState, useMemo } from 'react';
import { ShieldCheck, Lock, User, AlertCircle, CheckCircle2, XCircle } from 'lucide-react';
import {
  Input,
  Button,
  Divider,
} from '@heroui/react';
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
    <div className="min-h-screen bg-slate-100 dark:bg-[#0B0F14] flex flex-col justify-center items-center p-4 font-sans text-slate-800 dark:text-slate-200 antialiased">
      <div className="max-w-md w-full bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border rounded-xs shadow-2xs p-6 space-y-5">
        {/* Header */}
        <div className="flex flex-col items-center text-center pb-2">
          <div className="w-10 h-10 rounded-xs bg-slate-100 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border flex items-center justify-center mb-3 text-slate-800 dark:text-slate-200">
            <ShieldCheck className="w-5 h-5" />
          </div>
          <h1 className="text-xl font-bold tracking-tight text-slate-900 dark:text-white">
            LOCARDX
          </h1>
          <p className="text-[11px] uppercase tracking-wider text-slate-500 dark:text-slate-400 font-semibold mt-0.5">
            Initial Workstation Provisioning
          </p>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-2 text-center leading-relaxed max-w-xs">
            Closed registration requires creating the root Administrator account. First-run setup is permanently locked once completed.
          </p>
        </div>

        <div className="space-y-4">
          {displayError && (
            <div className="p-3 rounded-xs bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900 text-rose-800 dark:text-rose-200 text-xs flex items-start gap-2">
              <AlertCircle className="w-4 h-4 text-rose-600 dark:text-rose-400 shrink-0 mt-0.5" />
              <span>{displayError}</span>
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-3.5">
            <Input
              size="sm"
              variant="bordered"
              label="Administrator Username"
              placeholder="admin"
              value={username}
              onValueChange={setUsername}
              isRequired
              autoComplete="username"
              isDisabled={isLoading}
              startContent={<User className="w-4 h-4 text-slate-400" />}
            />

            <Input
              size="sm"
              variant="bordered"
              type="password"
              label="Master Password"
              placeholder="••••••••••••"
              value={password}
              onValueChange={setPassword}
              isRequired
              autoComplete="new-password"
              isDisabled={isLoading}
              startContent={<Lock className="w-4 h-4 text-slate-400" />}
            />

            <Input
              size="sm"
              variant="bordered"
              type="password"
              label="Confirm Master Password"
              placeholder="••••••••••••"
              value={confirmPassword}
              onValueChange={setConfirmPassword}
              isRequired
              autoComplete="new-password"
              isDisabled={isLoading}
              startContent={<Lock className="w-4 h-4 text-slate-400" />}
            />

            {/* Password Strength Checklist */}
            <div className="bg-slate-50 dark:bg-[#1B2530] border border-slate-200 dark:border-[#26313D] rounded-xs p-3 space-y-1.5 text-xs">
              <span className="font-semibold text-slate-600 dark:text-slate-400 block mb-1 text-[10px] uppercase tracking-wider">
                Argon2id Complexity Checklist:
              </span>
              <div className="grid grid-cols-1 gap-1 text-[11px] text-slate-500 dark:text-slate-400">
                <div className="flex items-center space-x-1.5">
                  {passwordValidation.minLength ? (
                    <CheckCircle2 className="w-3.5 h-3.5 text-slate-800 dark:text-white" />
                  ) : (
                    <XCircle className="w-3.5 h-3.5 text-slate-300 dark:text-slate-600" />
                  )}
                  <span className={passwordValidation.minLength ? 'text-slate-900 dark:text-white font-medium' : ''}>
                    At least 10 characters
                  </span>
                </div>
                <div className="flex items-center space-x-1.5">
                  {passwordValidation.hasUpper ? (
                    <CheckCircle2 className="w-3.5 h-3.5 text-slate-800 dark:text-white" />
                  ) : (
                    <XCircle className="w-3.5 h-3.5 text-slate-300 dark:text-slate-600" />
                  )}
                  <span className={passwordValidation.hasUpper ? 'text-slate-900 dark:text-white font-medium' : ''}>
                    Uppercase letter (A-Z)
                  </span>
                </div>
                <div className="flex items-center space-x-1.5">
                  {passwordValidation.hasLower ? (
                    <CheckCircle2 className="w-3.5 h-3.5 text-slate-800 dark:text-white" />
                  ) : (
                    <XCircle className="w-3.5 h-3.5 text-slate-300 dark:text-slate-600" />
                  )}
                  <span className={passwordValidation.hasLower ? 'text-slate-900 dark:text-white font-medium' : ''}>
                    Lowercase letter (a-z)
                  </span>
                </div>
                <div className="flex items-center space-x-1.5">
                  {passwordValidation.hasDigit ? (
                    <CheckCircle2 className="w-3.5 h-3.5 text-slate-800 dark:text-white" />
                  ) : (
                    <XCircle className="w-3.5 h-3.5 text-slate-300 dark:text-slate-600" />
                  )}
                  <span className={passwordValidation.hasDigit ? 'text-slate-900 dark:text-white font-medium' : ''}>
                    Numerical digit (0-9)
                  </span>
                </div>
                <div className="flex items-center space-x-1.5">
                  {passwordValidation.hasSpecial ? (
                    <CheckCircle2 className="w-3.5 h-3.5 text-slate-800 dark:text-white" />
                  ) : (
                    <XCircle className="w-3.5 h-3.5 text-slate-300 dark:text-slate-600" />
                  )}
                  <span className={passwordValidation.hasSpecial ? 'text-slate-900 dark:text-white font-medium' : ''}>
                    Special symbol character
                  </span>
                </div>
              </div>
            </div>

            <Button
              type="submit"
              color="primary"
              size="md"
              isLoading={isLoading}
              isDisabled={isLoading || !isPasswordValid || !username}
              className="w-full mt-2 bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-semibold rounded-xs"
            >
              {isLoading ? 'Initializing...' : 'Initialize Administrator & Launch'}
            </Button>
          </form>
        </div>

        <Divider className="my-1 bg-slate-100 dark:bg-[#26313D]" />

        <div className="pt-2 text-center flex flex-col space-y-1">
          <p className="text-[10px] text-slate-400 dark:text-slate-500">
            Backend integrity interlock: First-run setup permanently locks upon creation.
          </p>
          <Button
            size="sm"
            variant="light"
            onPress={() => checkFirstRun()}
            className="text-xs text-slate-700 dark:text-slate-300 font-medium"
          >
            Administrator already provisioned? Switch to Sign In &rarr;
          </Button>
        </div>
      </div>
    </div>
  );
};

export default FirstRunSetupPage;
