import React, { useEffect, useState, useMemo } from 'react';
import {
  Users,
  UserPlus,
  ShieldAlert,
  Search,
  CheckCircle2,
  XCircle,
  AlertCircle,
  ToggleLeft,
  ToggleRight,
  X,
} from 'lucide-react';
import { useAuthStore, extractErrorMessage } from '../stores/authStore';
import { PublicUser, UserRole } from '../types/auth';
import * as authService from '../services/auth';

export const UserManagementPage: React.FC = () => {
  const { currentUser, sessionToken } = useAuthStore();
  const [users, setUsers] = useState<PublicUser[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);

  // New User Modal state
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [newUsername, setNewUsername] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [newRole, setNewRole] = useState<UserRole>('Investigator');
  const [modalError, setModalError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Filter state
  const [searchTerm, setSearchTerm] = useState('');

  const passwordValidation = useMemo(() => {
    return {
      minLength: newPassword.length >= 10,
      hasUpper: /[A-Z]/.test(newPassword),
      hasLower: /[a-z]/.test(newPassword),
      hasDigit: /[0-9]/.test(newPassword),
      hasSpecial: /[^A-Za-z0-9]/.test(newPassword),
    };
  }, [newPassword]);

  const isPasswordValid =
    passwordValidation.minLength &&
    passwordValidation.hasUpper &&
    passwordValidation.hasLower &&
    passwordValidation.hasDigit &&
    passwordValidation.hasSpecial;

  const loadUsers = async () => {
    if (!sessionToken) return;
    try {
      setLoading(true);
      setError(null);
      const userList = await authService.listUsers(sessionToken);
      setUsers(userList);
    } catch (err: unknown) {
      const msg = extractErrorMessage(err, 'Failed to retrieve users.');
      setError(msg);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadUsers();
  }, [sessionToken]);

  const handleToggleEnabled = async (user: PublicUser) => {
    if (!sessionToken) return;
    try {
      setError(null);
      setActionSuccess(null);
      await authService.setUserEnabled(sessionToken, {
        user_id: user.user_id,
        enabled: !user.enabled,
      });
      setActionSuccess(
        `User ${user.username} successfully ${user.enabled ? 'disabled' : 'enabled'}.`
      );
      await loadUsers();
    } catch (err: unknown) {
      const msg = extractErrorMessage(err, 'Failed to modify user status.');
      setError(msg);
    }
  };

  const handleCreateUser = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!sessionToken) return;
    setModalError(null);

    if (!newUsername.trim()) {
      setModalError('Username is required.');
      return;
    }
    if (!isPasswordValid) {
      setModalError('Password does not satisfy complexity requirements.');
      return;
    }

    try {
      setIsSubmitting(true);
      await authService.createUser(sessionToken, {
        username: newUsername.trim(),
        password: newPassword,
        role: newRole,
      });
      setIsModalOpen(false);
      const createdUser = newUsername.trim();
      const createdRole = newRole;
      setNewUsername('');
      setNewPassword('');
      setNewRole('Investigator');
      setActionSuccess(`Account '${createdUser}' (${createdRole}) created successfully.`);
      await loadUsers();
    } catch (err: unknown) {
      const msg = extractErrorMessage(err, 'Failed to create user account.');
      setModalError(msg);
    } finally {
      setIsSubmitting(false);
    }
  };

  if (currentUser?.role !== 'Administrator') {
    return (
      <div className="p-8 max-w-2xl mx-auto text-center">
        <div className="bg-white border border-rose-200 rounded-md p-6 text-rose-800 shadow-2xs">
          <ShieldAlert className="w-10 h-10 mx-auto mb-2 text-rose-600" />
          <h2 className="text-base font-bold text-slate-900">Access Restricted</h2>
          <p className="text-xs mt-1 text-slate-500">
            User management requires Administrator authority. Your active role is {currentUser?.role}.
          </p>
        </div>
      </div>
    );
  }

  const filteredUsers = users.filter((u) =>
    u.username.toLowerCase().includes(searchTerm.toLowerCase())
  );

  return (
    <div className="space-y-5 text-slate-800 font-sans">
      {/* Title & Actions */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 border-b border-slate-200 pb-4">
        <div>
          <h2 className="text-xl font-bold tracking-tight text-slate-900 flex items-center gap-2">
            <Users className="w-5 h-5 text-slate-700" />
            User Management & Access Control
          </h2>
          <p className="text-xs text-slate-500 mt-0.5">
            Enforce closed operator provisioning, RBAC roles, and account operational status.
          </p>
        </div>

        <button
          onClick={() => {
            setModalError(null);
            setIsModalOpen(true);
          }}
          className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-semibold text-white bg-slate-900 hover:bg-slate-800 rounded-md shadow-2xs transition-colors"
        >
          <UserPlus className="w-3.5 h-3.5" />
          Create New User
        </button>
      </div>

      {/* Notifications */}
      {actionSuccess && (
        <div className="p-3 bg-emerald-50 border border-emerald-200 rounded-md text-emerald-800 text-xs flex items-center justify-between">
          <div className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-emerald-600" />
            <span>{actionSuccess}</span>
          </div>
          <button
            onClick={() => setActionSuccess(null)}
            className="text-emerald-600 hover:text-emerald-800"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      )}

      {error && (
        <div className="p-3 bg-rose-50 border border-rose-200 rounded-md text-rose-800 text-xs flex items-center justify-between">
          <div className="flex items-center gap-2">
            <AlertCircle className="w-4 h-4 text-rose-600" />
            <span>{error}</span>
          </div>
          <button
            onClick={() => setError(null)}
            className="text-rose-600 hover:text-rose-800"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      )}

      {/* Search & Filter Bar */}
      <div className="bg-white border border-slate-200 rounded-md p-3 flex items-center justify-between gap-4 shadow-2xs">
        <div className="relative flex-1 max-w-xs">
          <Search className="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-2.5" />
          <input
            type="text"
            placeholder="Search accounts..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full pl-8 pr-3 py-1 bg-slate-50 border border-slate-200 rounded text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:border-slate-400 focus:bg-white transition-colors"
          />
        </div>
        <div className="text-xs text-slate-500 font-mono">
          Total Registered Users: <span className="text-slate-900 font-bold">{users.length}</span>
        </div>
      </div>

      {/* Users Table */}
      <div className="bg-white border border-slate-200 rounded-md overflow-hidden shadow-2xs">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs text-slate-700">
            <thead className="bg-slate-50 text-slate-500 border-b border-slate-200 uppercase tracking-wider font-semibold">
              <tr>
                <th className="px-4 py-2.5">Username</th>
                <th className="px-4 py-2.5">Role</th>
                <th className="px-4 py-2.5">Account Status</th>
                <th className="px-4 py-2.5">Created</th>
                <th className="px-4 py-2.5">Last Active</th>
                <th className="px-4 py-2.5 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100">
              {loading ? (
                <tr>
                  <td colSpan={6} className="px-4 py-6 text-center text-slate-400">
                    Loading accounts...
                  </td>
                </tr>
              ) : filteredUsers.length === 0 ? (
                <tr>
                  <td colSpan={6} className="px-4 py-6 text-center text-slate-400">
                    No matching users found.
                  </td>
                </tr>
              ) : (
                filteredUsers.map((u) => {
                  const isCurrent = u.username === currentUser.username;
                  return (
                    <tr
                      key={u.user_id}
                      className="hover:bg-slate-50/60 transition-colors"
                    >
                      <td className="px-4 py-3 font-medium text-slate-900 flex items-center gap-2">
                        <span>{u.username}</span>
                        {isCurrent && (
                          <span className="text-[10px] px-1.5 py-0.2 rounded bg-sky-50 text-sky-700 border border-sky-200 font-mono font-semibold">
                            ACTIVE
                          </span>
                        )}
                      </td>
                      <td className="px-4 py-3">
                        <span
                          className={`px-2 py-0.5 rounded text-[11px] font-mono border ${
                            u.role === 'Administrator'
                              ? 'bg-purple-50 text-purple-700 border-purple-200'
                              : u.role === 'Investigator'
                              ? 'bg-blue-50 text-blue-700 border-blue-200'
                              : u.role === 'Operator'
                              ? 'bg-amber-50 text-amber-700 border-amber-200'
                              : 'bg-slate-100 text-slate-700 border-slate-200'
                          }`}
                        >
                          {u.role}
                        </span>
                      </td>
                      <td className="px-4 py-3">
                        <span
                          className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-medium border ${
                            u.enabled
                              ? 'bg-emerald-50 text-emerald-700 border-emerald-200'
                              : 'bg-rose-50 text-rose-700 border-rose-200'
                          }`}
                        >
                          <span
                            className={`w-1.5 h-1.5 rounded-full ${
                              u.enabled ? 'bg-emerald-500' : 'bg-rose-500'
                            }`}
                          />
                          {u.enabled ? 'Active' : 'Disabled'}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-slate-500 font-mono text-[11px]">
                        {new Date(u.created_at).toLocaleDateString()}
                      </td>
                      <td className="px-4 py-3 text-slate-500 font-mono text-[11px]">
                        {u.last_login ? (
                          new Date(u.last_login).toLocaleString()
                        ) : (
                          <span className="text-slate-400">Never</span>
                        )}
                      </td>
                      <td className="px-4 py-3 text-right">
                        <button
                          onClick={() => handleToggleEnabled(u)}
                          title={u.enabled ? 'Disable Account' : 'Enable Account'}
                          className={`inline-flex items-center gap-1 px-2.5 py-1 rounded text-xs transition-colors ${
                            u.enabled
                              ? 'text-rose-600 hover:bg-rose-50 border border-rose-200'
                              : 'text-emerald-600 hover:bg-emerald-50 border border-emerald-200'
                          }`}
                        >
                          {u.enabled ? (
                            <>
                              <ToggleLeft className="w-3.5 h-3.5" />
                              <span>Disable</span>
                            </>
                          ) : (
                            <>
                              <ToggleRight className="w-3.5 h-3.5" />
                              <span>Enable</span>
                            </>
                          )}
                        </button>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Create User Modal */}
      {isModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/40 backdrop-blur-2xs">
          <div className="bg-white border border-slate-200 rounded-lg p-6 max-w-md w-full shadow-lg space-y-4">
            <div className="flex items-center justify-between border-b border-slate-200 pb-3">
              <h3 className="text-sm font-semibold text-slate-900 flex items-center gap-2">
                <UserPlus className="w-4 h-4 text-slate-700" />
                Register New Workstation User
              </h3>
              <button
                onClick={() => setIsModalOpen(false)}
                className="text-slate-400 hover:text-slate-600"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {modalError && (
              <div className="p-2.5 rounded bg-rose-50 border border-rose-200 text-rose-800 text-xs flex items-center gap-2">
                <AlertCircle className="w-4 h-4 text-rose-600 flex-shrink-0" />
                <span>{modalError}</span>
              </div>
            )}

            <form onSubmit={handleCreateUser} className="space-y-3.5">
              <div>
                <label className="block text-xs font-medium text-slate-700 mb-1">
                  Username
                </label>
                <input
                  type="text"
                  value={newUsername}
                  onChange={(e) => setNewUsername(e.target.value)}
                  required
                  className="w-full px-3 py-1.5 bg-slate-50 border border-slate-300 rounded text-xs text-slate-900 focus:outline-none focus:border-slate-500 focus:bg-white"
                  placeholder="e.g. investigator_smith"
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-slate-700 mb-1">
                  Role Assignment
                </label>
                <select
                  value={newRole}
                  onChange={(e) => setNewRole(e.target.value as UserRole)}
                  className="w-full px-3 py-1.5 bg-slate-50 border border-slate-300 rounded text-xs text-slate-900 focus:outline-none focus:border-slate-500 focus:bg-white"
                >
                  <option value="Administrator">Administrator (Full System & User Control)</option>
                  <option value="Investigator">Investigator (Forensics, Carving & Reports)</option>
                  <option value="Operator">Operator (Sanitization Operations)</option>
                  <option value="Viewer">Viewer (Read-Only Inspection)</option>
                </select>
              </div>

              <div>
                <label className="block text-xs font-medium text-slate-700 mb-1">
                  Initial Password
                </label>
                <input
                  type="password"
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  required
                  className="w-full px-3 py-1.5 bg-slate-50 border border-slate-300 rounded text-xs text-slate-900 focus:outline-none focus:border-slate-500 focus:bg-white"
                  placeholder="••••••••••••"
                />
              </div>

              {/* Checklist */}
              <div className="bg-slate-50 border border-slate-200 rounded p-2.5 space-y-1 text-[11px] text-slate-500">
                <div className="flex items-center gap-1.5">
                  {passwordValidation.minLength ? (
                    <CheckCircle2 className="w-3 h-3 text-emerald-600" />
                  ) : (
                    <XCircle className="w-3 h-3 text-slate-300" />
                  )}
                  <span>10+ characters</span>
                </div>
                <div className="flex items-center gap-1.5">
                  {passwordValidation.hasUpper && passwordValidation.hasLower ? (
                    <CheckCircle2 className="w-3 h-3 text-emerald-600" />
                  ) : (
                    <XCircle className="w-3 h-3 text-slate-300" />
                  )}
                  <span>Uppercase and lowercase letters</span>
                </div>
                <div className="flex items-center gap-1.5">
                  {passwordValidation.hasDigit && passwordValidation.hasSpecial ? (
                    <CheckCircle2 className="w-3 h-3 text-emerald-600" />
                  ) : (
                    <XCircle className="w-3 h-3 text-slate-300" />
                  )}
                  <span>Number and special symbol</span>
                </div>
              </div>

              <div className="flex justify-end gap-2 pt-2 border-t border-slate-200">
                <button
                  type="button"
                  onClick={() => setIsModalOpen(false)}
                  className="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 text-slate-700 text-xs rounded transition-colors"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={isSubmitting || !isPasswordValid}
                  className="px-3.5 py-1.5 bg-slate-900 hover:bg-slate-800 disabled:bg-slate-200 disabled:text-slate-400 text-white text-xs font-semibold rounded transition-colors"
                >
                  {isSubmitting ? 'Creating...' : 'Create Account'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};

export default UserManagementPage;
