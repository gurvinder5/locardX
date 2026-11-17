import React, { useState } from 'react';
import { Lock, User, AlertCircle, Shield } from 'lucide-react';
import {
  Input,
  Button,
  Chip,
  Divider,
} from '@heroui/react';
import { useAuthStore } from '../stores/authStore';

export const LoginPage: React.FC = () => {
  const { login, isLoading, error, clearError } = useAuthStore();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    clearError();

    if (!username.trim() || !password) {
      return;
    }

    try {
      await login({
        username: username.trim(),
        password,
      });
    } catch {
      // Error message is stored in authStore
    }
  };

  return (
    <div className="min-h-screen bg-slate-100 dark:bg-[#0B0F14] flex flex-col justify-center items-center p-4 font-sans text-slate-800 dark:text-slate-200 antialiased">
      <div className="max-w-sm w-full bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border rounded-xs shadow-2xs p-6 space-y-4">
        {/* Header */}
        <div className="flex flex-col items-center text-center pb-2">
          <div className="w-10 h-10 rounded-xs bg-slate-100 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border flex items-center justify-center mb-3 text-slate-800 dark:text-slate-200">
            <Shield className="w-5 h-5" />
          </div>
          <h1 className="text-xl font-bold tracking-tight text-slate-900 dark:text-white">
            LOCARDX
          </h1>
          <p className="text-[11px] uppercase tracking-wider text-slate-500 dark:text-slate-400 font-semibold mt-0.5">
            Forensic Workstation Access
          </p>
          <Chip
            size="sm"
            variant="flat"
            className="mt-2 text-[10px] h-5 font-mono bg-slate-100 dark:bg-workstation-surface text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-workstation-border rounded-xs"
          >
            Closed Authentication
          </Chip>
        </div>

        <div>
          {error && (
            <div className="mb-4 p-3 rounded-xs bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900 text-rose-800 dark:text-rose-200 text-xs flex items-start gap-2">
              <AlertCircle className="w-4 h-4 text-rose-600 dark:text-rose-400 shrink-0 mt-0.5" />
              <span>{error}</span>
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-4">
            <Input
              size="sm"
              variant="bordered"
              label="Username"
              placeholder="Operator username"
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
              label="Password"
              placeholder="••••••••••••"
              value={password}
              onValueChange={setPassword}
              isRequired
              autoComplete="current-password"
              isDisabled={isLoading}
              startContent={<Lock className="w-4 h-4 text-slate-400" />}
            />

            <Button
              type="submit"
              color="primary"
              size="md"
              isLoading={isLoading}
              isDisabled={isLoading || !username || !password}
              className="w-full mt-2 bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-semibold rounded-xs"
            >
              Sign In to Workstation
            </Button>
          </form>
        </div>

        <Divider className="my-1 bg-slate-100 dark:bg-[#26313D]" />

        <div className="pt-2 text-center">
          <p className="text-[10px] text-slate-400 dark:text-slate-500 leading-normal">
            Closed registration. Contact an Administrator for credential provisioning.
          </p>
        </div>
      </div>
    </div>
  );
};

export default LoginPage;
