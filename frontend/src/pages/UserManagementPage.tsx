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
} from 'lucide-react';
import {
  Button,
  Input,
  Chip,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  Table,
  TableHeader,
  TableColumn,
  TableBody,
  TableRow,
  TableCell,
} from '@heroui/react';
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
  const [newDisplayName, setNewDisplayName] = useState('');
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

  const handleRoleChange = async (user: PublicUser, role: UserRole) => {
    if (!sessionToken) return;
    try {
      setError(null);
      setActionSuccess(null);
      await authService.changeUserRole(sessionToken, {
        user_id: user.user_id,
        new_role: role,
      });
      setActionSuccess(`Role for '${user.username}' successfully changed to ${role}.`);
      await loadUsers();
    } catch (err: unknown) {
      const msg = extractErrorMessage(err, 'Failed to change user role.');
      setError(msg);
    }
  };

  const handleUnlockUser = async (user: PublicUser) => {
    if (!sessionToken) return;
    try {
      setError(null);
      setActionSuccess(null);
      await authService.unlockUser(sessionToken, {
        user_id: user.user_id,
      });
      setActionSuccess(`User '${user.username}' has been successfully unlocked.`);
      await loadUsers();
    } catch (err: unknown) {
      const msg = extractErrorMessage(err, 'Failed to unlock user.');
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
        display_name: newDisplayName.trim() || undefined,
      });
      setIsModalOpen(false);
      const createdUser = newUsername.trim();
      const createdRole = newRole;
      setNewUsername('');
      setNewDisplayName('');
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
        <div className="bg-white dark:bg-workstation-panel border border-rose-200 dark:border-rose-900/40 rounded-xs p-6 text-rose-800 dark:text-rose-300">
          <ShieldAlert className="w-10 h-10 mx-auto mb-2 text-rose-600 dark:text-rose-400" />
          <h2 className="text-base font-bold text-slate-900 dark:text-slate-100">Access Restricted</h2>
          <p className="text-xs mt-1 text-slate-500 dark:text-slate-400">
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
    <div className="space-y-5 w-full min-w-0 font-sans">
      {/* Title & Actions */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 border-b border-slate-200 dark:border-workstation-border pb-4">
        <div>
          <h1 className="text-xl font-bold tracking-tight text-slate-900 dark:text-slate-100 flex items-center gap-2">
            <Users className="w-5 h-5 text-slate-700 dark:text-slate-300" />
            User Management & Access Control
          </h1>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
            Enforce closed operator provisioning, RBAC roles, and account operational status.
          </p>
        </div>

        <Button
          size="sm"
          onPress={() => {
            setModalError(null);
            setIsModalOpen(true);
          }}
          startContent={<UserPlus className="w-3.5 h-3.5" />}
          className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 text-xs font-semibold px-3.5 shadow-2xs rounded-xs"
        >
          Create New User
        </Button>
      </div>

      {/* Notifications */}
      {actionSuccess && (
        <div className="p-3 bg-slate-100 dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border rounded-xs text-slate-800 dark:text-slate-200 text-xs flex items-center justify-between">
          <div className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-slate-700 dark:text-slate-300" />
            <span>{actionSuccess}</span>
          </div>
          <Button
            size="sm"
            variant="light"
            onPress={() => setActionSuccess(null)}
            className="text-slate-500 hover:text-slate-800 h-6 px-2 text-xs"
          >
            Dismiss
          </Button>
        </div>
      )}

      {error && (
        <div className="p-3 bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 rounded-xs text-rose-800 dark:text-rose-300 text-xs flex items-center justify-between">
          <div className="flex items-center gap-2">
            <AlertCircle className="w-4 h-4 text-rose-600 dark:text-rose-400" />
            <span>{error}</span>
          </div>
          <Button
            size="sm"
            variant="light"
            onPress={() => setError(null)}
            className="text-rose-600 hover:text-rose-800 h-6 px-2 text-xs"
          >
            Dismiss
          </Button>
        </div>
      )}

      {/* Search & Filter Bar */}
      <div className="bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border rounded-xs p-3 flex items-center justify-between gap-4">
        <div className="w-full max-w-xs">
          <Input
            size="sm"
            placeholder="Search accounts..."
            value={searchTerm}
            onValueChange={setSearchTerm}
            startContent={<Search className="w-3.5 h-3.5 text-slate-400 shrink-0" />}
            classNames={{
              inputWrapper: 'bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded-xs',
            }}
          />
        </div>
        <div className="text-xs text-slate-500 dark:text-slate-400 font-mono">
          Total Registered Users: <span className="text-slate-900 dark:text-slate-100 font-bold">{users.length}</span>
        </div>
      </div>

      {/* Users Table */}
      <div className="bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border rounded-xs overflow-hidden min-w-0">
        <div className="w-full overflow-x-auto scrollbar-thin min-w-0">
          <Table
            aria-label="Workstation Users Table"
            removeWrapper
            classNames={{
              base: 'min-w-[620px]',
              table: 'min-w-[620px]',
              thead: 'border-b border-slate-200 dark:border-workstation-border',
              th: 'bg-slate-50 dark:bg-workstation-surface text-slate-500 dark:text-slate-400 text-[10px] font-bold uppercase tracking-wider h-8 py-0 px-3',
              td: 'py-2.5 px-3 text-xs text-slate-700 dark:text-slate-300 border-b border-slate-100 dark:border-workstation-border',
            }}
          >
          <TableHeader>
            <TableColumn>USERNAME</TableColumn>
            <TableColumn>ROLE</TableColumn>
            <TableColumn>ACCOUNT STATUS</TableColumn>
            <TableColumn>CREATED</TableColumn>
            <TableColumn>LAST ACTIVE</TableColumn>
            <TableColumn className="text-right">ACTIONS</TableColumn>
          </TableHeader>
          <TableBody emptyContent={loading ? 'Loading accounts...' : 'No matching users found.'}>
            {filteredUsers.map((u) => {
              const isCurrent = u.username === currentUser?.username;
              return (
                <TableRow key={u.user_id} className="hover:bg-slate-50/60 dark:hover:bg-workstation-surface/50">
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <span className="font-medium text-slate-900 dark:text-slate-100">{u.username}</span>
                      {isCurrent && (
                        <Chip size="sm" variant="flat" className="font-mono font-bold text-[10px] bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700">
                          ACTIVE
                        </Chip>
                      )}
                    </div>
                    {u.display_name && (
                      <div className="text-[11px] font-normal text-slate-500 dark:text-slate-400">
                        {u.display_name}
                      </div>
                    )}
                  </TableCell>
                  <TableCell>
                    <select
                      value={u.role}
                      onChange={(e) => handleRoleChange(u, e.target.value as UserRole)}
                      disabled={isCurrent && u.role === 'Administrator'}
                      title={isCurrent && u.role === 'Administrator' ? 'Administrator cannot demote self' : 'Change user role'}
                      className="px-2 py-1 rounded text-[11px] font-mono border bg-white dark:bg-workstation-surface border-slate-300 dark:border-workstation-border text-slate-800 dark:text-slate-200 cursor-pointer focus:outline-none disabled:opacity-75 disabled:cursor-not-allowed"
                    >
                      <option value="Administrator">Administrator</option>
                      <option value="Investigator">Investigator</option>
                      <option value="Operator">Operator</option>
                      <option value="Viewer">Viewer</option>
                    </select>
                  </TableCell>
                  <TableCell>
                    <Chip
                      size="sm"
                      variant="flat"
                      className={`text-[10px] font-medium border ${
                        u.enabled
                          ? 'bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 border-slate-300 dark:border-slate-700'
                          : 'bg-rose-50 dark:bg-rose-950/40 text-rose-700 dark:text-rose-400 border-rose-200 dark:border-rose-900/40'
                      }`}
                    >
                      {u.enabled ? 'Active' : 'Disabled'}
                    </Chip>
                  </TableCell>
                  <TableCell className="font-mono text-[11px] text-slate-500 dark:text-slate-400">
                    {new Date(u.created_at).toLocaleDateString()}
                  </TableCell>
                  <TableCell className="font-mono text-[11px] text-slate-500 dark:text-slate-400">
                    {u.last_login || u.last_login_at ? (
                      new Date(u.last_login || u.last_login_at!).toLocaleString()
                    ) : (
                      <span className="text-slate-400">Never</span>
                    )}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-1.5">
                      <Button
                        size="sm"
                        variant="flat"
                        onPress={() => handleUnlockUser(u)}
                        className="text-xs h-7 px-2.5 text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-workstation-border bg-slate-50 dark:bg-workstation-surface"
                      >
                        Unlock
                      </Button>
                      <Button
                        size="sm"
                        variant="flat"
                        color={u.enabled ? 'danger' : 'default'}
                        onPress={() => handleToggleEnabled(u)}
                        isDisabled={isCurrent && u.role === 'Administrator'}
                        startContent={u.enabled ? <ToggleLeft className="w-3.5 h-3.5" /> : <ToggleRight className="w-3.5 h-3.5" />}
                        className="text-xs h-7 px-2.5"
                      >
                        {u.enabled ? 'Disable' : 'Enable'}
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
        </div>
      </div>

      {/* Create User Modal */}
      <Modal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        size="md"
        classNames={{
          base: 'bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border',
          header: 'border-b border-slate-100 dark:border-workstation-border',
          footer: 'border-t border-slate-100 dark:border-workstation-border',
        }}
      >
        <ModalContent>
          <ModalHeader className="flex items-center gap-2">
            <UserPlus className="w-4 h-4 text-slate-700 dark:text-slate-300" />
            <span className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              Register New Workstation User
            </span>
          </ModalHeader>

          <form onSubmit={handleCreateUser}>
            <ModalBody className="p-6 space-y-3.5">
              {modalError && (
                <div className="p-2.5 rounded bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 text-rose-800 dark:text-rose-300 text-xs flex items-center gap-2">
                  <AlertCircle className="w-4 h-4 text-rose-600 flex-shrink-0" />
                  <span>{modalError}</span>
                </div>
              )}

              <div>
                <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1">
                  Full Name / Display Name <span className="text-slate-400 font-normal">(Optional)</span>
                </label>
                <Input
                  size="sm"
                  value={newDisplayName}
                  onValueChange={setNewDisplayName}
                  placeholder="e.g. John Doe"
                  classNames={{
                    inputWrapper: 'bg-slate-50 dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border',
                  }}
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1">
                  Username
                </label>
                <Input
                  size="sm"
                  value={newUsername}
                  onValueChange={setNewUsername}
                  isRequired
                  placeholder="e.g. investigator_smith"
                  classNames={{
                    inputWrapper: 'bg-slate-50 dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border',
                  }}
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1">
                  Role Assignment
                </label>
                <select
                  value={newRole}
                  onChange={(e) => setNewRole(e.target.value as UserRole)}
                  className="w-full px-3 py-1.5 bg-slate-50 dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border rounded text-xs text-slate-900 dark:text-slate-100 focus:outline-none"
                >
                  <option value="Administrator">Administrator (Full System & User Control)</option>
                  <option value="Investigator">Investigator (Forensics, Carving & Reports)</option>
                  <option value="Operator">Operator (Sanitization Operations)</option>
                  <option value="Viewer">Viewer (Read-Only Inspection)</option>
                </select>
              </div>

              <div>
                <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1">
                  Initial Password
                </label>
                <Input
                  size="sm"
                  type="password"
                  value={newPassword}
                  onValueChange={setNewPassword}
                  isRequired
                  placeholder="••••••••••••"
                  classNames={{
                    inputWrapper: 'bg-slate-50 dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border',
                  }}
                />
              </div>

              {/* Checklist */}
              <div className="bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded p-2.5 space-y-1 text-[11px] text-slate-500 dark:text-slate-400">
                <div className="flex items-center gap-1.5">
                  {passwordValidation.minLength ? (
                    <CheckCircle2 className="w-3 h-3 text-slate-700 dark:text-slate-300" />
                  ) : (
                    <XCircle className="w-3 h-3 text-slate-300 dark:text-slate-600" />
                  )}
                  <span>10+ characters</span>
                </div>
                <div className="flex items-center gap-1.5">
                  {passwordValidation.hasUpper && passwordValidation.hasLower ? (
                    <CheckCircle2 className="w-3 h-3 text-slate-700 dark:text-slate-300" />
                  ) : (
                    <XCircle className="w-3 h-3 text-slate-300 dark:text-slate-600" />
                  )}
                  <span>Uppercase and lowercase letters</span>
                </div>
                <div className="flex items-center gap-1.5">
                  {passwordValidation.hasDigit && passwordValidation.hasSpecial ? (
                    <CheckCircle2 className="w-3 h-3 text-slate-700 dark:text-slate-300" />
                  ) : (
                    <XCircle className="w-3 h-3 text-slate-300 dark:text-slate-600" />
                  )}
                  <span>Number and special symbol</span>
                </div>
              </div>
            </ModalBody>

            <ModalFooter>
              <Button
                size="sm"
                variant="flat"
                onPress={() => setIsModalOpen(false)}
                className="text-xs font-medium text-slate-600 dark:text-slate-400"
              >
                Cancel
              </Button>
              <Button
                size="sm"
                type="submit"
                isDisabled={isSubmitting || !isPasswordValid}
                isLoading={isSubmitting}
                className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 text-xs font-semibold px-4 shadow-sm"
              >
                Create Account
              </Button>
            </ModalFooter>
          </form>
        </ModalContent>
      </Modal>
    </div>
  );
};

export default UserManagementPage;
