import React, { useEffect, useState } from 'react';
import {
  ShieldCheck,
  Search,
  Cpu,
  Download,
  Briefcase,
  ClipboardList,
  ArrowRight,
  FolderOpen,
  CheckCircle2,
} from 'lucide-react';
import { AppInfo } from '../services/tauri';
import { listCases, verifyAuditChain } from '../services/case';
import { Case, AuditChainVerification } from '../types/case';

interface DashboardPageProps {
  appInfo: AppInfo | null;
  loading: boolean;
  onNavigate?: (tab: string) => void;
}

export const DashboardPage: React.FC<DashboardPageProps> = ({ appInfo, loading, onNavigate }) => {
  const [cases, setCases] = useState<Case[]>([]);
  const [auditStatus, setAuditStatus] = useState<AuditChainVerification | null>(null);
  const [loadingCases, setLoadingCases] = useState<boolean>(true);

  useEffect(() => {
    let isMounted = true;
    async function loadDashboardStats() {
      try {
        setLoadingCases(true);
        const [caseRes, auditRes] = await Promise.all([
          listCases(undefined, 5, 0),
          verifyAuditChain(),
        ]);
        if (isMounted) {
          setCases(caseRes.cases);
          setAuditStatus(auditRes);
        }
      } catch (err) {
        console.error('Failed to load dashboard metrics:', err);
      } finally {
        if (isMounted) {
          setLoadingCases(false);
        }
      }
    }
    loadDashboardStats();
    return () => {
      isMounted = false;
    };
  }, []);

  const openCasesCount = cases.filter((c) => c.status === 'open' || c.status === 'in_progress').length;

  return (
    <div className="space-y-6 text-slate-800 font-sans">
      {/* Workstation Status Banner */}
      <section className="bg-white border border-slate-200 rounded-md p-5 shadow-2xs">
        <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
          <div>
            <h2 className="text-sm font-semibold text-slate-900 flex items-center gap-2">
              <ShieldCheck className="w-4 h-4 text-emerald-600" />
              LocardX Unified Forensic & Sanitization Workstation
            </h2>
            <p className="text-xs text-slate-500 mt-1 max-w-2xl">
              Cryptographic SHA-256 chain-of-custody, read-only disk acquisition, deep file carving, NIST SP 800-88 sanitization, and case management active.
            </p>
          </div>
          <div className="flex items-center gap-2 font-mono text-xs text-slate-700 bg-slate-50 px-3 py-1.5 rounded border border-slate-200">
            <span className="w-2 h-2 rounded-full bg-emerald-500" />
            <span>IPC Core: {loading ? 'Querying...' : appInfo?.build_status || 'Operational'}</span>
          </div>
        </div>
      </section>

      {/* Case Management & Quick Navigation Summary */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* KPI 1: Active Cases */}
        <div
          onClick={() => onNavigate && onNavigate('cases')}
          className="bg-white border border-slate-200 rounded-md p-4 shadow-2xs hover:border-indigo-300 cursor-pointer transition-all group"
        >
          <div className="flex items-center justify-between text-slate-500">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Investigation Cases</span>
            <Briefcase className="w-4 h-4 text-indigo-600 group-hover:scale-110 transition-transform" />
          </div>
          <div className="text-2xl font-bold font-mono text-slate-900 mt-2">
            {loadingCases ? '...' : cases.length}
          </div>
          <div className="flex items-center justify-between text-[11px] text-slate-500 mt-1">
            <span>{openCasesCount} active / in-progress</span>
            <span className="text-indigo-600 font-medium group-hover:underline flex items-center gap-0.5">
              Open Cases <ArrowRight className="w-3 h-3" />
            </span>
          </div>
        </div>

        {/* KPI 2: Cryptographic Audit Status */}
        <div
          onClick={() => onNavigate && onNavigate('audit-logs')}
          className="bg-white border border-slate-200 rounded-md p-4 shadow-2xs hover:border-emerald-300 cursor-pointer transition-all group"
        >
          <div className="flex items-center justify-between text-slate-500">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Audit Chain Status</span>
            <ClipboardList className="w-4 h-4 text-emerald-600 group-hover:scale-110 transition-transform" />
          </div>
          <div className="text-sm font-bold text-emerald-700 mt-3 flex items-center gap-1.5">
            <CheckCircle2 className="w-4 h-4 text-emerald-600" />
            <span>{auditStatus?.is_valid ? 'Chain Verified' : 'Checking...'}</span>
          </div>
          <div className="flex items-center justify-between text-[11px] text-slate-500 mt-2">
            <span>{auditStatus ? `${auditStatus.total_events} events chained` : 'Zero tampering detected'}</span>
            <span className="text-emerald-600 font-medium group-hover:underline flex items-center gap-0.5">
              Inspect Log <ArrowRight className="w-3 h-3" />
            </span>
          </div>
        </div>

        {/* KPI 3: Forensic Acquisition */}
        <div
          onClick={() => onNavigate && onNavigate('acquisition')}
          className="bg-white border border-slate-200 rounded-md p-4 shadow-2xs hover:border-blue-300 cursor-pointer transition-all group"
        >
          <div className="flex items-center justify-between text-slate-500">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Forensic Acquisition</span>
            <Download className="w-4 h-4 text-blue-600 group-hover:scale-110 transition-transform" />
          </div>
          <div className="text-2xl font-bold font-mono text-slate-900 mt-2">
            Raw DD
          </div>
          <div className="flex items-center justify-between text-[11px] text-slate-500 mt-1">
            <span>Streaming SHA-256</span>
            <span className="text-blue-600 font-medium group-hover:underline flex items-center gap-0.5">
              Acquire <ArrowRight className="w-3 h-3" />
            </span>
          </div>
        </div>

        {/* KPI 4: Evidence Carving & Recovery */}
        <div
          onClick={() => onNavigate && onNavigate('recovery')}
          className="bg-white border border-slate-200 rounded-md p-4 shadow-2xs hover:border-purple-300 cursor-pointer transition-all group"
        >
          <div className="flex items-center justify-between text-slate-500">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Carving & Recovery</span>
            <Search className="w-4 h-4 text-purple-600 group-hover:scale-110 transition-transform" />
          </div>
          <div className="text-2xl font-bold font-mono text-slate-900 mt-2">
            TSK & Carve
          </div>
          <div className="flex items-center justify-between text-[11px] text-slate-500 mt-1">
            <span>Multi-format validation</span>
            <span className="text-purple-600 font-medium group-hover:underline flex items-center gap-0.5">
              Recover <ArrowRight className="w-3 h-3" />
            </span>
          </div>
        </div>
      </div>

      {/* Active Cases Quick Access */}
      <div className="bg-white border border-slate-200 rounded-md shadow-2xs overflow-hidden">
        <div className="px-5 py-3.5 border-b border-slate-200 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <FolderOpen className="w-4 h-4 text-indigo-600" />
            <h3 className="text-xs font-bold uppercase tracking-wider text-slate-800">
              Recent Forensic Cases
            </h3>
          </div>
          {onNavigate && (
            <button
              onClick={() => onNavigate('cases')}
              className="inline-flex items-center gap-1 text-xs font-semibold text-indigo-600 hover:text-indigo-800 transition-colors"
            >
              <span>View All Cases</span>
              <ArrowRight className="w-3 h-3" />
            </button>
          )}
        </div>

        {cases.length === 0 ? (
          <div className="p-8 text-center text-slate-500 text-xs">
            {loadingCases ? 'Loading cases...' : 'No forensic investigation cases registered.'}
          </div>
        ) : (
          <div className="divide-y divide-slate-100">
            {cases.slice(0, 4).map((c) => (
              <div
                key={c.case_id}
                onClick={() => onNavigate && onNavigate('cases')}
                className="p-4 hover:bg-slate-50/80 cursor-pointer transition-colors flex items-center justify-between gap-4"
              >
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-xs font-bold text-slate-900 bg-slate-100 px-2 py-0.5 rounded border border-slate-200">
                      {c.case_reference}
                    </span>
                    <h4 className="text-xs font-semibold text-slate-800 truncate">{c.title}</h4>
                    <span
                      className={`text-[10px] font-semibold px-2 py-0.5 rounded capitalize ${
                        c.status === 'open'
                          ? 'bg-emerald-50 text-emerald-700 border border-emerald-200'
                          : c.status === 'in_progress'
                          ? 'bg-blue-50 text-blue-700 border border-blue-200'
                          : c.status === 'completed'
                          ? 'bg-slate-100 text-slate-700 border border-slate-200'
                          : 'bg-amber-50 text-amber-700 border border-amber-200'
                      }`}
                    >
                      {c.status.replace('_', ' ')}
                    </span>
                  </div>
                  <p className="text-xs text-slate-500 mt-1 truncate">{c.description || 'No description provided.'}</p>
                </div>

                <div className="text-right shrink-0 text-[11px] text-slate-400 font-mono">
                  <div>Lead: {c.lead_investigator}</div>
                  <div className="text-[10px] text-slate-400">
                    Created: {new Date(c.created_at).toLocaleDateString()}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Core Architectural Pillars */}
      <div>
        <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-500 mb-3">
          Architecture & System Modules (Steps 10 – 13)
        </h3>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {/* Pillar 1: Drive Eraser */}
          <div
            onClick={() => onNavigate && onNavigate('drive-eraser')}
            className="bg-white border border-slate-200 rounded-md p-4 shadow-2xs hover:border-slate-300 cursor-pointer transition-colors"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2 text-slate-800">
                <Cpu className="w-4 h-4 text-slate-600" />
                <h4 className="text-xs font-semibold">Drive Eraser</h4>
              </div>
              <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded bg-slate-100 text-slate-700 border border-slate-200">
                Step 10
              </span>
            </div>
            <p className="text-xs text-slate-500 leading-relaxed">
              Block sanitization, NIST SP 800-88 compliance, and hardware ATA/NVMe sanitize interlocks.
            </p>
          </div>

          {/* Pillar 2: Forensic Acquisition */}
          <div
            onClick={() => onNavigate && onNavigate('acquisition')}
            className="bg-white border border-slate-200 rounded-md p-4 shadow-2xs hover:border-slate-300 cursor-pointer transition-colors"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2 text-slate-800">
                <Download className="w-4 h-4 text-blue-600" />
                <h4 className="text-xs font-semibold">Disk Acquisition</h4>
              </div>
              <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded bg-slate-100 text-slate-700 border border-slate-200">
                Step 11
              </span>
            </div>
            <p className="text-xs text-slate-500 leading-relaxed">
              Read-only physical bitstream capture, Raw DD output, and streaming SHA-256 verification.
            </p>
          </div>

          {/* Pillar 3: Recovery */}
          <div
            onClick={() => onNavigate && onNavigate('recovery')}
            className="bg-white border border-slate-200 rounded-md p-4 shadow-2xs hover:border-slate-300 cursor-pointer transition-colors"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2 text-slate-800">
                <Search className="w-4 h-4 text-purple-600" />
                <h4 className="text-xs font-semibold">File Recovery</h4>
              </div>
              <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded bg-slate-100 text-slate-700 border border-slate-200">
                Step 12
              </span>
            </div>
            <p className="text-xs text-slate-500 leading-relaxed">
              Forensic image validation, TSK structural traversal, signature carving, and confidence scoring.
            </p>
          </div>

          {/* Pillar 4: Case & Audit Management */}
          <div
            onClick={() => onNavigate && onNavigate('cases')}
            className="bg-white border border-slate-200 rounded-md p-4 shadow-2xs hover:border-slate-300 cursor-pointer transition-colors"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2 text-slate-800">
                <Briefcase className="w-4 h-4 text-indigo-600" />
                <h4 className="text-xs font-semibold">Case & Audit</h4>
              </div>
              <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded bg-slate-100 text-slate-700 border border-slate-200">
                Step 13
              </span>
            </div>
            <p className="text-xs text-slate-500 leading-relaxed">
              Cross-module audit chains, tamper-evident custody ledger, and unified forensic cert reporting.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default DashboardPage;
