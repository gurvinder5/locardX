import React, { useEffect, useState } from 'react';
import {
  LayoutDashboard,
  HardDrive,
  Users,
  FileText,
  Search,
  ClipboardList,
  LogOut,
  Shield,
  ShieldCheck,
  ShieldAlert,
  Activity,
  Loader2,
  Cpu,
  Download,
  Briefcase,
  ChevronLeft,
  ChevronRight,
  Menu,
} from 'lucide-react';
import { getAppInfo, AppInfo } from '../services/tauri';
import { useAuthStore } from '../stores/authStore';
import { useCaseStore } from '../stores/caseStore';
import { ActiveCaseRequiredModal } from '../components/case/ActiveCaseRequiredModal';
import FirstRunSetupPage from '../pages/FirstRunSetupPage';
import LoginPage from '../pages/LoginPage';
import UserManagementPage from '../pages/UserManagementPage';
import DashboardPage from '../pages/DashboardPage';
import DeviceExplorerPage from '../pages/DeviceExplorerPage';
import { IntegrityVerificationPage } from '../pages/IntegrityVerificationPage';
import { OperationsPage } from '../pages/OperationsPage';
import { SafetyInterlocksPage } from '../pages/SafetyInterlocksPage';
import { SanitizationPlannerPage } from '../pages/SanitizationPlannerPage';
import { FileEraserPage } from '../pages/FileEraserPage';
import { DriveEraserPage } from '../pages/DriveEraserPage';
import { ForensicAcquisitionPage } from '../pages/ForensicAcquisitionPage';
import { ForensicRecoveryPage } from '../pages/ForensicRecoveryPage';
import { CaseManagementPage } from '../pages/CaseManagementPage';
import { AuditTrailPage } from '../pages/AuditTrailPage';
import { useRecoveryStore } from '../stores/recoveryStore';


type NavTab =
  | 'dashboard'
  | 'cases'
  | 'device-explorer'
  | 'operations'
  | 'integrity'
  | 'safety-interlocks'
  | 'sanitization-planner'
  | 'acquisition'
  | 'drive-eraser'
  | 'file-eraser'
  | 'recovery'
  | 'audit-logs'
  | 'user-management';

interface NavItem {
  id: NavTab;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  iconColor?: string;
  adminOnly?: boolean;
}

interface NavGroup {
  groupName: string;
  items: NavItem[];
}

const NAV_GROUPS: NavGroup[] = [
  {
    groupName: 'Workspace',
    items: [
      { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard, iconColor: 'text-sky-600' },
      { id: 'cases', label: 'Case Management', icon: Briefcase, iconColor: 'text-indigo-600' },
    ],
  },
  {
    groupName: 'Forensic Operations',
    items: [
      { id: 'acquisition', label: 'Forensic Acquisition', icon: Download, iconColor: 'text-blue-600' },
      { id: 'recovery', label: 'Forensics & Recovery', icon: Search, iconColor: 'text-emerald-600' },
      { id: 'device-explorer', label: 'Device Explorer', icon: HardDrive, iconColor: 'text-slate-600' },
    ],
  },
  {
    groupName: 'Data Sanitization',
    items: [
      { id: 'drive-eraser', label: 'Drive Eraser', icon: Cpu, iconColor: 'text-rose-600' },
      { id: 'file-eraser', label: 'File Eraser', icon: FileText, iconColor: 'text-amber-600' },
      { id: 'sanitization-planner', label: 'Sanitization Planner', icon: Cpu, iconColor: 'text-violet-600' },
    ],
  },
  {
    groupName: 'Governance & Security',
    items: [
      { id: 'operations', label: 'Operations Ledger', icon: Activity, iconColor: 'text-indigo-600' },
      { id: 'integrity', label: 'Integrity Verification', icon: ShieldCheck, iconColor: 'text-teal-600' },
      { id: 'safety-interlocks', label: 'Safety & Interlocks', icon: ShieldAlert, iconColor: 'text-amber-600' },
      { id: 'audit-logs', label: 'Audit Trail', icon: ClipboardList, iconColor: 'text-slate-600' },
      { id: 'user-management', label: 'User Management', icon: Users, iconColor: 'text-purple-600', adminOnly: true },
    ],
  },
];

export const App: React.FC = () => {
  const {
    currentUser,
    isFirstRun,
    isLoading: authLoading,
    checkFirstRun,
    logout,
  } = useAuthStore();

  const { activeCase, loadActiveCase } = useCaseStore();
  const [activeTab, setActiveTab] = useState<NavTab>('device-explorer');
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [infoLoading, setInfoLoading] = useState<boolean>(true);
  const [showCaseSelectorModal, setShowCaseSelectorModal] = useState(false);
  const [blockedNavTarget, setBlockedNavTarget] = useState<NavTab | null>(null);
  const { isScanning: recoveryScanning } = useRecoveryStore();
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(() => {
    return localStorage.getItem('locardx_sidebar_collapsed') === 'true';
  });

  const toggleSidebar = () => {
    setSidebarCollapsed((prev) => {
      const next = !prev;
      localStorage.setItem('locardx_sidebar_collapsed', String(next));
      return next;
    });
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        toggleSidebar();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  useEffect(() => {
    checkFirstRun();
    loadActiveCase();
  }, [checkFirstRun, loadActiveCase]);

  useEffect(() => {
    let isMounted = true;
    async function loadAppMetadata() {
      try {
        setInfoLoading(true);
        const info = await getAppInfo();
        if (isMounted) {
          setAppInfo(info);
        }
      } catch (err) {
        console.error('Failed to query backend application metadata:', err);
      } finally {
        if (isMounted) {
          setInfoLoading(false);
        }
      }
    }
    loadAppMetadata();
    return () => {
      isMounted = false;
    };
  }, []);

  // Loading Screen (Light Workstation Theme)
  if (authLoading && isFirstRun === null) {
    return (
      <div className="min-h-screen bg-slate-50 flex flex-col items-center justify-center text-slate-700 font-sans">
        <Loader2 className="w-8 h-8 text-slate-600 animate-spin mb-3" />
        <p className="text-xs uppercase tracking-wider text-slate-500 font-semibold">
          Initializing LocardX Forensic Workstation...
        </p>
      </div>
    );
  }

  // First Run Setup
  if (isFirstRun === true) {
    return <FirstRunSetupPage />;
  }

  // Unauthenticated: Show Login Page
  if (!currentUser) {
    return <LoginPage />;
  }

  const isAdmin = currentUser.role === 'Administrator';

  const handleTabClick = (tab: NavTab) => {
    if (tab === 'acquisition' || tab === 'recovery') {
      if (!activeCase || activeCase.status === 'completed' || activeCase.status === 'archived') {
        setBlockedNavTarget(tab);
        setShowCaseSelectorModal(true);
        return;
      }
    }
    setActiveTab(tab);
  };

  return (
    <div className="h-screen bg-slate-50 text-slate-900 flex flex-col font-sans antialiased overflow-hidden">
      {/* Workstation Header */}
      <header className="bg-white border-b border-slate-200 px-5 py-2 flex items-center justify-between shrink-0 z-40 shadow-2xs">
        <div className="flex items-center space-x-2.5">
          <button
            onClick={toggleSidebar}
            className="p-1 rounded-md text-slate-500 hover:text-slate-800 hover:bg-slate-100 transition-colors"
            title={sidebarCollapsed ? 'Expand sidebar (Ctrl+B)' : 'Collapse sidebar (Ctrl+B)'}
          >
            <Menu className="w-4 h-4" />
          </button>
          <div className="w-2 h-2 rounded-full bg-emerald-500" />
          <h1 className="text-base font-bold tracking-tight text-slate-900">
            LOCARD<span className="text-sky-600">X</span>
          </h1>
          <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-slate-100 text-slate-600 border border-slate-200">
            v{appInfo?.version || '0.1.0'}
          </span>
          <span className="text-[11px] text-slate-400 hidden sm:inline">|</span>
          <span className="text-xs text-slate-500 font-medium hidden sm:inline">
            Forensic & Sanitization Workstation
          </span>
        </div>

        {/* Active Case Indicator */}
        <div className="flex items-center">
          {activeCase ? (
            <button
              onClick={() => setShowCaseSelectorModal(true)}
              className="flex items-center gap-2 px-3 py-1 rounded-md bg-sky-50 text-sky-800 border border-sky-200 hover:bg-sky-100 transition-colors shadow-2xs group"
              title="Click to switch or view active forensic case"
            >
              <Briefcase className="w-3.5 h-3.5 text-sky-600 group-hover:scale-105 transition-transform" />
              <div className="flex items-center gap-1.5 text-xs">
                <span className="font-semibold text-slate-500 uppercase text-[10px] tracking-wider">Active Case:</span>
                <span className="font-mono font-bold text-sky-900">{activeCase.case_reference}</span>
                <span className="text-slate-400">·</span>
                <span className="max-w-[200px] truncate text-slate-700 font-medium">{activeCase.title}</span>
                <span className={`text-[9px] uppercase px-1.5 py-0.5 rounded font-bold border ${
                  activeCase.status === 'open' || activeCase.status === 'in_progress'
                    ? 'bg-emerald-50 text-emerald-700 border-emerald-200'
                    : 'bg-rose-50 text-rose-700 border-rose-200'
                }`}>
                  {activeCase.status}
                </span>
              </div>
            </button>
          ) : (
            <button
              onClick={() => setShowCaseSelectorModal(true)}
              className="flex items-center gap-2 px-3 py-1 rounded-md bg-amber-50 text-amber-800 border border-amber-300 hover:bg-amber-100 transition-colors shadow-2xs animate-pulse"
              title="No active case selected. Forensic operations will be blocked. Click to select or create a case."
            >
              <Briefcase className="w-3.5 h-3.5 text-amber-600" />
              <div className="flex items-center gap-1.5 text-xs">
                <span className="font-semibold text-slate-500 uppercase text-[10px] tracking-wider">Active Case:</span>
                <span className="font-mono font-bold text-amber-900">None</span>
                <span className="text-[10px] font-semibold text-amber-700 bg-amber-200/60 px-1.5 py-0.5 rounded">
                  Forensics Blocked
                </span>
              </div>
            </button>
          )}
        </div>

        {/* User Info & Sign Out */}
        <div className="flex items-center space-x-3 text-xs">
          <div className="flex items-center space-x-2">
            <span className="text-slate-500">Operator:</span>
            <span className="font-semibold text-slate-800">{currentUser.username}</span>
            <span
              className={`px-2 py-0.5 rounded text-[10px] font-mono uppercase font-semibold border ${
                currentUser.role === 'Administrator'
                  ? 'bg-purple-50 text-purple-700 border-purple-200'
                  : currentUser.role === 'Investigator'
                  ? 'bg-blue-50 text-blue-700 border-blue-200'
                  : currentUser.role === 'Operator'
                  ? 'bg-amber-50 text-amber-700 border-amber-200'
                  : 'bg-slate-100 text-slate-600 border-slate-200'
              }`}
            >
              {currentUser.role}
            </span>
          </div>

          <div className="h-4 w-px bg-slate-200" />

          <button
            onClick={() => logout()}
            className="text-slate-500 hover:text-rose-600 flex items-center gap-1 transition-colors px-2 py-1 rounded hover:bg-slate-100"
            title="Sign out of current workstation session"
          >
            <LogOut className="w-3.5 h-3.5" />
            <span className="text-xs">Sign Out</span>
          </button>
        </div>
      </header>

      {/* Main Workspace: Left Sidebar + Central Viewport */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left Vertical Sidebar */}
        <aside
          className={`bg-white border-r border-slate-200 flex flex-col shrink-0 transition-all duration-200 select-none ${
            sidebarCollapsed ? 'w-14' : 'w-56'
          }`}
        >
          <div className="flex-1 overflow-y-auto py-3 px-2 space-y-4">
            {NAV_GROUPS.map((group) => {
              const visibleItems = group.items.filter((item) => !item.adminOnly || isAdmin);
              if (visibleItems.length === 0) return null;

              return (
                <div key={group.groupName} className="space-y-0.5">
                  {!sidebarCollapsed ? (
                    <div className="px-2.5 py-1 text-[10px] font-bold uppercase tracking-wider text-slate-400">
                      {group.groupName}
                    </div>
                  ) : (
                    <div className="h-px bg-slate-100 my-1.5 mx-1" />
                  )}
                  {visibleItems.map((item) => {
                    const Icon = item.icon;
                    const isActive = activeTab === item.id;

                    return (
                      <button
                        key={item.id}
                        onClick={() => handleTabClick(item.id)}
                        className={`w-full text-left rounded-md flex items-center transition-all ${
                          sidebarCollapsed
                            ? 'justify-center p-2'
                            : 'px-2.5 py-1.5 gap-2.5 text-xs'
                        } ${
                          isActive
                            ? 'bg-slate-900 text-white font-semibold shadow-2xs'
                            : 'text-slate-600 hover:text-slate-900 hover:bg-slate-100/80 font-medium'
                        }`}
                        title={sidebarCollapsed ? item.label : undefined}
                      >
                        <Icon className={`w-4 h-4 shrink-0 ${isActive ? 'text-white' : item.iconColor || 'text-slate-500'}`} />
                        {!sidebarCollapsed && (
                          <span className="truncate flex-1">{item.label}</span>
                        )}
                        {item.id === 'recovery' && recoveryScanning && (
                          <span className="relative flex h-2 w-2 shrink-0">
                            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                            <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
                          </span>
                        )}
                      </button>
                    );
                  })}
                </div>
              );
            })}
          </div>

          {/* Sidebar Bottom Collapse Control */}
          <div className="p-2 border-t border-slate-100 bg-slate-50/60 flex items-center justify-between">
            <button
              onClick={toggleSidebar}
              className={`flex items-center gap-2 p-1.5 rounded text-xs text-slate-500 hover:text-slate-900 hover:bg-slate-200/60 transition-colors w-full ${
                sidebarCollapsed ? 'justify-center' : ''
              }`}
              title={sidebarCollapsed ? 'Expand sidebar (Ctrl+B)' : 'Collapse sidebar (Ctrl+B)'}
            >
              {sidebarCollapsed ? (
                <ChevronRight className="w-4 h-4" />
              ) : (
                <>
                  <ChevronLeft className="w-4 h-4" />
                  <span className="text-[11px] font-medium">Collapse</span>
                  <span className="text-[10px] text-slate-400 ml-auto font-mono">Ctrl+B</span>
                </>
              )}
            </button>
          </div>
        </aside>

        {/* Central Viewport */}
        <main className="flex-1 overflow-y-auto p-6 bg-slate-50">
          <div className="max-w-7xl mx-auto w-full">
            {activeTab === 'cases' && <CaseManagementPage />}
            {activeTab === 'device-explorer' && <DeviceExplorerPage />}
            {activeTab === 'operations' && <OperationsPage />}
            {activeTab === 'integrity' && <IntegrityVerificationPage />}
            {activeTab === 'safety-interlocks' && <SafetyInterlocksPage />}
            {activeTab === 'sanitization-planner' && <SanitizationPlannerPage />}

            {activeTab === 'dashboard' && (
              <DashboardPage
                appInfo={appInfo}
                loading={infoLoading}
                onNavigate={(tab) => handleTabClick(tab as NavTab)}
              />
            )}

            {activeTab === 'user-management' && <UserManagementPage />}

            {activeTab === 'drive-eraser' && <DriveEraserPage />}

            {activeTab === 'file-eraser' && <FileEraserPage />}

            {activeTab === 'acquisition' && <ForensicAcquisitionPage />}

            <div className={activeTab === 'recovery' ? '' : 'hidden'}>
              <ForensicRecoveryPage />
            </div>

            {activeTab === 'audit-logs' && <AuditTrailPage />}
          </div>
        </main>
      </div>

      {/* Active Case Required Modal */}
      <ActiveCaseRequiredModal
        isOpen={showCaseSelectorModal}
        onClose={() => {
          setShowCaseSelectorModal(false);
          setBlockedNavTarget(null);
        }}
        operationName={
          blockedNavTarget === 'acquisition'
            ? 'Forensic Disk Acquisition'
            : blockedNavTarget === 'recovery'
            ? 'Forensic File Recovery'
            : 'Forensic Operation'
        }
        onCaseActivated={() => {
          setShowCaseSelectorModal(false);
          if (blockedNavTarget) {
            setActiveTab(blockedNavTarget);
            setBlockedNavTarget(null);
          }
        }}
      />

      {/* Footer Status Bar */}
      <footer className="border-t border-slate-200 px-6 py-2 text-[11px] text-slate-500 flex justify-between items-center bg-white shadow-xs">
        <div className="flex items-center space-x-3">
          <span className="flex items-center gap-1.5 text-slate-600">
            <Shield className="w-3.5 h-3.5 text-emerald-600" />
            <span>Active Session: {currentUser.username} ({currentUser.role})</span>
          </span>
          <span className="text-slate-300">&bull;</span>
          <span className="text-slate-500">Read-Only Device Discovery Active</span>
        </div>
        <div className="flex items-center space-x-3 font-mono text-[10px]">
          <span className="text-slate-600">Audit Chain: Verified</span>
          <span className="text-slate-300">&bull;</span>
          <span className="text-slate-400">Every contact leaves a trace</span>
        </div>
      </footer>
    </div>
  );
};

export default App;
