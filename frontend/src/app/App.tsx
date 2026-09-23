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
    <div className="min-h-screen bg-slate-50 text-slate-900 flex flex-col font-sans antialiased">
      {/* Workstation Header */}
      <header className="bg-white border-b border-slate-200 px-6 py-2.5 flex items-center justify-between sticky top-0 z-40 shadow-2xs">
        <div className="flex items-center space-x-3">
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
                <span className="text-slate-400">â€”</span>
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

      {/* Navigation Subheader */}
      <nav className="bg-white border-b border-slate-200 px-6">
        <div className="flex space-x-1 overflow-x-auto py-1">
          <button
            onClick={() => handleTabClick('cases')}
            className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-2 transition-colors ${
              activeTab === 'cases'
                ? 'bg-slate-100 text-slate-900 font-semibold shadow-2xs border border-slate-200'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
            }`}
          >
            <Briefcase className="w-3.5 h-3.5 text-indigo-600" />
            <span>Cases</span>
          </button>

          <button
            onClick={() => handleTabClick('device-explorer')}
            className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-2 transition-colors ${
              activeTab === 'device-explorer'
                ? 'bg-slate-100 text-slate-900 font-semibold shadow-2xs border border-slate-200'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
            }`}
          >
            <HardDrive className="w-3.5 h-3.5 text-slate-700" />
            <span>Device Explorer</span>
          </button>

          <button
            onClick={() => handleTabClick('operations')}
            className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-2 transition-colors ${
              activeTab === 'operations'
                ? 'bg-slate-100 text-slate-900 font-semibold shadow-2xs border border-slate-200'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
            }`}
          >
            <Activity className="w-3.5 h-3.5 text-indigo-600" />
            <span>Operations</span>
          </button>

          <button
            onClick={() => handleTabClick('integrity')}
            className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-2 transition-colors ${
              activeTab === 'integrity'
                ? 'bg-slate-100 text-slate-900 font-semibold shadow-2xs border border-slate-200'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
            }`}
          >
            <ShieldCheck className="w-3.5 h-3.5 text-emerald-600" />
            <span>Integrity Verification</span>
          </button>

          <button
            onClick={() => handleTabClick('safety-interlocks')}
            className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-2 transition-colors ${
              activeTab === 'safety-interlocks'
                ? 'bg-slate-100 text-slate-900 font-semibold shadow-2xs border border-slate-200'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
            }`}
          >
            <ShieldAlert className="w-3.5 h-3.5 text-amber-700" />
            <span>Safety & Interlocks</span>
          </button>

          <button
            onClick={() => handleTabClick('sanitization-planner')}
            className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-2 transition-colors ${
              activeTab === 'sanitization-planner'
                ? 'bg-slate-100 text-slate-900 font-semibold shadow-2xs border border-slate-200'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
            }`}
          >
            <Cpu className="w-3.5 h-3.5 text-indigo-700" />
            <span>Sanitization Planner</span>
          </button>

          <button
            onClick={() => handleTabClick('dashboard')}
            className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-2 transition-colors ${
              activeTab === 'dashboard'
                ? 'bg-slate-100 text-slate-900 font-semibold shadow-2xs border border-slate-200'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
            }`}
          >
            <LayoutDashboard className="w-3.5 h-3.5 text-slate-600" />
            <span>Dashboard</span>
          </button>

          <button
            onClick={() => handleTabClick('drive-eraser')}
            className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-2 transition-colors ${
              activeTab === 'drive-eraser'
                ? 'bg-slate-100 text-slate-900 font-semibold shadow-2xs border border-slate-200'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
            }`}
          >
            <Cpu className="w-3.5 h-3.5 text-slate-600" />
            <span>Drive Eraser</span>
          </button>

          <button
            onClick={() => handleTabClick('file-eraser')}
            className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-2 transition-colors ${
              activeTab === 'file-eraser'
                ? 'bg-slate-100 text-slate-900 font-semibold shadow-2xs border border-slate-200'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
            }`}
          >
            <FileText className="w-3.5 h-3.5 text-slate-600" />
            <span>File Eraser</span>
          </button>

          <button
            onClick={() => handleTabClick('acquisition')}
            className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-2 transition-colors ${
              activeTab === 'acquisition'
                ? 'bg-slate-100 text-slate-900 font-semibold shadow-2xs border border-slate-200'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
            }`}
          >
            <Download className="w-3.5 h-3.5 text-blue-600" />
            <span>Forensic Acquisition</span>
          </button>

          <button
            onClick={() => handleTabClick('recovery')}
            className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-2 transition-colors ${
              activeTab === 'recovery'
                ? 'bg-slate-100 text-slate-900 font-semibold shadow-2xs border border-slate-200'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
            }`}
          >
            <Search className="w-3.5 h-3.5 text-slate-600" />
            <span>Forensics & Recovery</span>
            {recoveryScanning && (
              <span className="relative flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
              </span>
            )}
          </button>

          <button
            onClick={() => handleTabClick('audit-logs')}
            className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-2 transition-colors ${
              activeTab === 'audit-logs'
                ? 'bg-slate-100 text-slate-900 font-semibold shadow-2xs border border-slate-200'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
            }`}
          >
            <ClipboardList className="w-3.5 h-3.5 text-slate-600" />
            <span>Audit Trail</span>
          </button>

          {isAdmin && (
            <button
              onClick={() => handleTabClick('user-management')}
              className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-2 transition-colors ${
                activeTab === 'user-management'
                  ? 'bg-slate-100 text-slate-900 font-semibold shadow-2xs border border-slate-200'
                  : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
              }`}
            >
              <Users className="w-3.5 h-3.5 text-purple-700" />
              <span>User Management</span>
            </button>
          )}
        </div>
      </nav>

      {/* Main Content Viewport */}
      <main className="flex-1 p-6 max-w-7xl mx-auto w-full">
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
      </main>

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
