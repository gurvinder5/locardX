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
  Sun,
  Moon,
} from 'lucide-react';
import {
  Button,
  Chip,
  Tooltip,
  Dropdown,
  DropdownTrigger,
  DropdownMenu,
  DropdownItem,
  Breadcrumbs,
  BreadcrumbItem,
  Divider,
} from '@heroui/react';
import { getAppInfo, AppInfo } from '../services/tauri';
import { useAuthStore } from '../stores/authStore';
import { useCaseStore } from '../stores/caseStore';
import { useAppStore } from '../stores/appStore';
import { useRecoveryStore } from '../stores/recoveryStore';
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

export type NavTab =
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
      { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
      { id: 'cases', label: 'Case Management', icon: Briefcase },
    ],
  },
  {
    groupName: 'Forensic Operations',
    items: [
      { id: 'acquisition', label: 'Forensic Acquisition', icon: Download },
      { id: 'recovery', label: 'Forensics & Recovery', icon: Search },
      { id: 'device-explorer', label: 'Device Explorer', icon: HardDrive },
    ],
  },
  {
    groupName: 'Data Sanitization',
    items: [
      { id: 'drive-eraser', label: 'Drive Eraser', icon: Cpu },
      { id: 'file-eraser', label: 'File Eraser', icon: FileText },
      { id: 'sanitization-planner', label: 'Sanitization Planner', icon: Cpu },
    ],
  },
  {
    groupName: 'Governance & Security',
    items: [
      { id: 'operations', label: 'Operations Ledger', icon: Activity },
      { id: 'integrity', label: 'Integrity Verification', icon: ShieldCheck },
      { id: 'safety-interlocks', label: 'Safety & Interlocks', icon: ShieldAlert },
      { id: 'audit-logs', label: 'Audit Trail', icon: ClipboardList },
      { id: 'user-management', label: 'User Management', icon: Users, adminOnly: true },
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
  const { theme, toggleTheme, sidebarCollapsed, toggleSidebar } = useAppStore();
  const [activeTab, setActiveTab] = useState<NavTab>('dashboard');
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [infoLoading, setInfoLoading] = useState<boolean>(true);
  const [showCaseSelectorModal, setShowCaseSelectorModal] = useState(false);
  const [blockedNavTarget, setBlockedNavTarget] = useState<NavTab | null>(null);
  const { isScanning: recoveryScanning } = useRecoveryStore();
  const mainRef = React.useRef<HTMLElement>(null);

  useEffect(() => {
    if (mainRef.current) {
      mainRef.current.scrollLeft = 0;
      mainRef.current.scrollTop = 0;
    }
  }, [activeTab]);

  useEffect(() => {
    if (theme === 'dark') {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [theme]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        toggleSidebar();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [toggleSidebar]);

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

  // Loading Screen
  if (authLoading && isFirstRun === null) {
    return (
      <div className="min-h-screen bg-slate-50 dark:bg-[#0B0F14] flex flex-col items-center justify-center text-slate-700 dark:text-slate-300 font-sans">
        <Loader2 className="w-8 h-8 text-slate-600 dark:text-slate-400 animate-spin mb-3" />
        <p className="text-xs uppercase tracking-wider text-slate-500 dark:text-slate-400 font-semibold">
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

  // Find active item info for breadcrumb
  let activeGroupName = 'Workspace';
  let activeItemLabel = 'Dashboard';
  for (const group of NAV_GROUPS) {
    const found = group.items.find((item) => item.id === activeTab);
    if (found) {
      activeGroupName = group.groupName;
      activeItemLabel = found.label;
      break;
    }
  }

  return (
    <div className={`${theme} h-screen bg-slate-50 dark:bg-[#0B0F14] text-slate-900 dark:text-slate-100 flex flex-col font-sans antialiased overflow-hidden`}>
      {/* Workstation Top Navigation Bar */}
      <header className="bg-white dark:bg-[#10161D] border-b border-slate-200 dark:border-[#26313D] px-4 py-2 flex items-center justify-between shrink-0 z-40 transition-colors">
        {/* Left: Brand Identity & Breadcrumb */}
        <div className="flex items-center space-x-3">
          <Button
            isIconOnly
            size="sm"
            variant="light"
            onClick={toggleSidebar}
            aria-label={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            className="text-slate-600 dark:text-slate-300 min-w-8 w-8 h-8"
          >
            {sidebarCollapsed ? <ChevronRight className="w-4 h-4" /> : <ChevronLeft className="w-4 h-4" />}
          </Button>

          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-slate-500 dark:bg-slate-400" />
            <span className="text-sm font-bold tracking-tight text-slate-900 dark:text-white">
              LOCARDX
            </span>
            <Chip
              size="sm"
              variant="flat"
              className="text-[10px] h-5 bg-slate-100 dark:bg-[#1B2530] text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]"
            >
              v{appInfo?.version || '0.1.0'}
            </Chip>
          </div>

          <Divider orientation="vertical" className="h-4 bg-slate-200 dark:bg-[#26313D] hidden md:block" />

          {/* Breadcrumbs */}
          <div className="hidden md:block">
            <Breadcrumbs size="sm" variant="light" className="text-xs">
              <BreadcrumbItem className="text-slate-500 dark:text-slate-400 font-medium">
                {activeGroupName}
              </BreadcrumbItem>
              <BreadcrumbItem isCurrent className="text-slate-900 dark:text-white font-semibold">
                {activeItemLabel}
              </BreadcrumbItem>
            </Breadcrumbs>
          </div>
        </div>

        {/* Center: Active Forensic Case Indicator */}
        <div className="flex items-center">
          {activeCase ? (
            <Button
              size="sm"
              variant="bordered"
              onClick={() => setShowCaseSelectorModal(true)}
              className="h-8 px-3 bg-white dark:bg-[#151C24] border-slate-300 dark:border-[#26313D] text-slate-800 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-[#1B2530]"
              startContent={<Briefcase className="w-3.5 h-3.5 text-slate-600 dark:text-slate-300" />}
            >
              <span className="font-semibold text-slate-500 dark:text-slate-400 uppercase text-[10px] tracking-wider">
                Case:
              </span>
              <span className="font-mono font-bold text-slate-900 dark:text-white text-xs">
                {activeCase.case_reference}
              </span>
              <span className="text-slate-400 hidden lg:inline">·</span>
              <span className="max-w-[160px] truncate text-slate-600 dark:text-slate-300 font-medium text-xs hidden lg:inline">
                {activeCase.title}
              </span>
              <Chip
                size="sm"
                variant="flat"
                className={`text-[9px] h-4 uppercase font-bold ${
                  activeCase.status === 'open' || activeCase.status === 'in_progress'
                    ? 'bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-[#26313D]'
                    : 'bg-rose-50 dark:bg-rose-950/40 text-rose-700 dark:text-rose-300 border border-rose-200 dark:border-rose-900'
                }`}
              >
                {activeCase.status}
              </Chip>
            </Button>
          ) : (
            <Button
              size="sm"
              variant="bordered"
              onClick={() => setShowCaseSelectorModal(true)}
              className="h-8 px-3 bg-amber-50 dark:bg-amber-950/20 border-amber-300 dark:border-amber-700 text-amber-900 dark:text-amber-200 hover:bg-amber-100 dark:hover:bg-amber-950/40 animate-pulse"
              startContent={<Briefcase className="w-3.5 h-3.5 text-amber-600 dark:text-amber-400" />}
            >
              <span className="font-semibold text-amber-800 dark:text-amber-300 uppercase text-[10px] tracking-wider">
                Active Case:
              </span>
              <span className="font-mono font-bold text-xs text-amber-950 dark:text-amber-100">None</span>
              <Chip size="sm" variant="flat" className="text-[10px] h-4 font-semibold bg-amber-200/80 dark:bg-amber-900/60 text-amber-900 dark:text-amber-100">
                Forensics Blocked
              </Chip>
            </Button>
          )}
        </div>

        {/* Right: Theme Toggle, Operator Profile, Sign Out */}
        <div className="flex items-center space-x-2">
          {/* Theme Switcher Button */}
          <Tooltip content={theme === 'dark' ? 'Switch to Light Theme' : 'Switch to Dark Theme'}>
            <Button
              isIconOnly
              size="sm"
              variant="light"
              onClick={toggleTheme}
              aria-label="Toggle theme"
              className="text-slate-600 dark:text-slate-300 min-w-8 w-8 h-8"
            >
              {theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
            </Button>
          </Tooltip>

          <Divider orientation="vertical" className="h-4 bg-slate-200 dark:bg-[#26313D]" />

          {/* User Profile & Actions Dropdown */}
          <Dropdown placement="bottom-end">
            <DropdownTrigger>
              <Button
                size="sm"
                variant="light"
                className="h-8 px-2 flex items-center gap-2 text-slate-800 dark:text-slate-200"
              >
                <div className="w-6 h-6 rounded-full bg-slate-200 dark:bg-[#26313D] flex items-center justify-center text-slate-700 dark:text-slate-300 text-xs font-bold">
                  {currentUser.username.charAt(0).toUpperCase()}
                </div>
                <div className="flex flex-col text-left hidden sm:flex">
                  <span className="text-xs font-semibold leading-tight text-slate-900 dark:text-white">
                    {currentUser.username}
                  </span>
                  <span className="text-[10px] text-slate-500 dark:text-slate-400 font-mono leading-tight">
                    {currentUser.role}
                  </span>
                </div>
              </Button>
            </DropdownTrigger>
            <DropdownMenu aria-label="User actions" className="text-xs">
              <DropdownItem key="profile" textValue="Operator Profile" className="opacity-100 cursor-default">
                <div className="font-semibold text-slate-900 dark:text-white">{currentUser.username}</div>
                <div className="text-[11px] text-slate-500 font-mono">{currentUser.role} Account</div>
              </DropdownItem>
              <DropdownItem
                key="logout"
                color="danger"
                className="text-rose-600 dark:text-rose-400"
                startContent={<LogOut className="w-4 h-4" />}
                onPress={() => logout()}
              >
                Sign Out Workstation
              </DropdownItem>
            </DropdownMenu>
          </Dropdown>
        </div>
      </header>

      {/* Main Workspace Layout: Collapsible Sidebar + Content Viewport */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left Sidebar */}
        <aside
          className={`bg-white dark:bg-[#10161D] border-r border-slate-200 dark:border-[#26313D] flex flex-col shrink-0 transition-all duration-200 select-none ${
            sidebarCollapsed ? 'w-16' : 'w-60'
          }`}
        >
          <div className="flex-1 overflow-y-auto py-3 px-2 space-y-4">
            {NAV_GROUPS.map((group) => {
              const visibleItems = group.items.filter((item) => !item.adminOnly || isAdmin);
              if (visibleItems.length === 0) return null;

              return (
                <div key={group.groupName} className="space-y-1">
                  {!sidebarCollapsed ? (
                    <div className="px-2.5 py-1 text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500">
                      {group.groupName}
                    </div>
                  ) : (
                    <div className="h-px bg-slate-100 dark:bg-[#1B2530] my-2 mx-1" />
                  )}
                  {visibleItems.map((item) => {
                    const Icon = item.icon;
                    const isActive = activeTab === item.id;

                    const navBtn = (
                      <button
                        key={item.id}
                        onClick={() => handleTabClick(item.id)}
                        className={`w-full text-left rounded-md flex items-center transition-all cursor-pointer ${
                          sidebarCollapsed
                            ? 'justify-center p-2.5'
                            : 'px-3 py-2 gap-2.5 text-xs'
                        } ${
                          isActive
                            ? 'bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-semibold shadow-xs'
                            : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-[#1B2530] font-medium'
                        }`}
                      >
                        <Icon className={`w-4 h-4 shrink-0 ${isActive ? 'text-white dark:text-slate-900' : 'text-slate-500 dark:text-slate-400'}`} />
                        {!sidebarCollapsed && (
                          <span className="truncate flex-1">{item.label}</span>
                        )}
                        {item.id === 'recovery' && recoveryScanning && (
                          <span className="relative flex h-2 w-2 shrink-0">
                            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-slate-400 opacity-75" />
                            <span className="relative inline-flex rounded-full h-2 w-2 bg-slate-600" />
                          </span>
                        )}
                      </button>
                    );

                    if (sidebarCollapsed) {
                      return (
                        <Tooltip key={item.id} content={item.label} placement="right">
                          <div>{navBtn}</div>
                        </Tooltip>
                      );
                    }
                    return navBtn;
                  })}
                </div>
              );
            })}
          </div>

          {/* Sidebar Footer Controls */}
          <div className="p-2 border-t border-slate-100 dark:border-[#26313D] bg-slate-50/60 dark:bg-[#151C24]">
            <Button
              size="sm"
              variant="light"
              onClick={toggleSidebar}
              className={`w-full text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white flex items-center ${
                sidebarCollapsed ? 'justify-center px-0' : 'justify-between px-2'
              }`}
            >
              {sidebarCollapsed ? (
                <ChevronRight className="w-4 h-4" />
              ) : (
                <>
                  <div className="flex items-center gap-2">
                    <ChevronLeft className="w-4 h-4" />
                    <span className="text-xs font-medium">Collapse</span>
                  </div>
                  <Chip size="sm" variant="flat" className="h-4 text-[9px] font-mono bg-slate-200 dark:bg-[#1B2530] text-slate-600 dark:text-slate-400">
                    Ctrl+B
                  </Chip>
                </>
              )}
            </Button>
          </div>
        </aside>

        {/* Central Viewport */}
        <main ref={mainRef} className="flex-1 overflow-y-auto overflow-x-hidden p-4 sm:p-6 bg-slate-50 dark:bg-[#0B0F14] transition-colors min-w-0">
          <div className="w-full min-w-0">
            {activeTab === 'dashboard' && (
              <DashboardPage
                appInfo={appInfo}
                loading={infoLoading}
                onNavigate={(tab) => handleTabClick(tab as NavTab)}
              />
            )}
            {activeTab === 'cases' && <CaseManagementPage />}
            {activeTab === 'device-explorer' && <DeviceExplorerPage />}
            {activeTab === 'operations' && <OperationsPage />}
            {activeTab === 'integrity' && <IntegrityVerificationPage />}
            {activeTab === 'safety-interlocks' && <SafetyInterlocksPage />}
            {activeTab === 'sanitization-planner' && <SanitizationPlannerPage />}
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

      {/* Workstation Footer Status Bar */}
      <footer className="border-t border-slate-200 dark:border-[#26313D] px-4 py-1.5 text-[11px] text-slate-500 dark:text-slate-400 flex justify-between items-center bg-white dark:bg-[#10161D] transition-colors shadow-2xs">
        <div className="flex items-center space-x-3">
          <span className="flex items-center gap-1.5 text-slate-700 dark:text-slate-300">
            <Shield className="w-3.5 h-3.5 text-slate-500" />
            <span>Active Session: {currentUser.username} ({currentUser.role})</span>
          </span>
          <span className="text-slate-300 dark:text-slate-700">&bull;</span>
          <span>Read-Only Device Discovery Active</span>
        </div>
        <div className="flex items-center space-x-3 font-mono text-[10px]">
          <span className="text-slate-600 dark:text-slate-300">Audit Chain: Verified</span>
          <span className="text-slate-300 dark:text-slate-700">&bull;</span>
          <span className="text-slate-400 dark:text-slate-500">Every contact leaves a trace</span>
        </div>
      </footer>
    </div>
  );
};

export default App;
