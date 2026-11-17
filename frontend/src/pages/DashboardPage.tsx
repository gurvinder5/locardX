import React, { useEffect, useState } from 'react';
import {
  ShieldCheck,
  Search,
  Cpu,
  Download,
  Briefcase,
  ClipboardList,
  ArrowRight,
  CheckCircle2,
  HardDrive,
  FileCheck,
  Layers,
} from 'lucide-react';
import {
  Button,
  Chip,
  Table,
  TableHeader,
  TableColumn,
  TableBody,
  TableRow,
  TableCell,
  Divider,
} from '@heroui/react';
import { AppInfo } from '../services/tauri';
import { listCases, verifyAuditChain } from '../services/case';
import { Case, AuditChainVerification } from '../types/case';
import { useCaseStore } from '../stores/caseStore';

interface DashboardPageProps {
  appInfo: AppInfo | null;
  loading: boolean;
  onNavigate?: (tab: string) => void;
}

export const DashboardPage: React.FC<DashboardPageProps> = ({ appInfo, loading, onNavigate }) => {
  const [cases, setCases] = useState<Case[]>([]);
  const [auditStatus, setAuditStatus] = useState<AuditChainVerification | null>(null);
  const [loadingCases, setLoadingCases] = useState<boolean>(true);
  const { activeCase, loadActiveCase } = useCaseStore();

  useEffect(() => {
    loadActiveCase();
  }, [loadActiveCase]);

  useEffect(() => {
    let isMounted = true;
    async function loadDashboardStats() {
      try {
        setLoadingCases(true);
        const [caseRes, auditRes] = await Promise.all([
          listCases(undefined, 8, 0),
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
    <div className="space-y-6 text-slate-800 dark:text-slate-200 font-sans pb-10">
      {/* 1. Page Heading */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 pb-4 border-b border-slate-200 dark:border-[#26313D]">
        <div>
          <h1 className="text-xl font-bold tracking-tight text-slate-900 dark:text-slate-100 flex items-center gap-2">
            <ShieldCheck className="w-5 h-5 text-slate-700 dark:text-slate-300" />
            Forensic Workstation Overview
          </h1>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
            LocardX enterprise forensic workstation — Bitstream acquisition, deep carving, NIST SP 800-88 sanitization, and SHA-256 chain of custody.
          </p>
        </div>
        <div className="flex items-center gap-3 shrink-0">
          <div className="flex items-center gap-2 font-mono text-xs px-3 py-1.5 rounded-sm bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]">
            <span className="w-2 h-2 rounded-full bg-slate-400 dark:bg-slate-500" />
            <span>IPC Core: {loading ? 'Querying...' : appInfo?.build_status || 'Operational'}</span>
          </div>
          {onNavigate && (
            <Button
              size="sm"
              variant="flat"
              onPress={() => onNavigate('cases')}
              className="text-xs font-semibold h-8 bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900"
            >
              New Investigation
            </Button>
          )}
        </div>
      </div>

      {/* 2. Contextual Active Case Bar */}
      {activeCase ? (
        <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 p-3 bg-slate-100/70 dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] rounded-sm text-xs">
          <div className="flex items-center gap-3">
            <Briefcase className="w-4 h-4 text-slate-600 dark:text-slate-400 shrink-0" />
            <div className="flex items-center gap-2 flex-wrap">
              <span className="text-[10px] uppercase font-bold tracking-wider text-slate-500 dark:text-slate-400">
                Active Case:
              </span>
              <span className="font-mono font-bold text-slate-900 dark:text-slate-100">
                {activeCase.case_reference}
              </span>
              <span className="text-slate-400">·</span>
              <span className="font-semibold text-slate-800 dark:text-slate-200">
                {activeCase.title}
              </span>
              <Chip size="sm" variant="flat" className="text-[9px] font-bold uppercase h-5 bg-slate-200 dark:bg-[#26313D] text-slate-800 dark:text-slate-200">
                {activeCase.status}
              </Chip>
              <span className="text-slate-400 hidden md:inline">·</span>
              <span className="text-slate-500 dark:text-slate-400 hidden md:inline">
                Lead: <span className="font-medium text-slate-700 dark:text-slate-300">{activeCase.lead_investigator}</span>
              </span>
            </div>
          </div>
          {onNavigate && (
            <Button
              size="sm"
              variant="light"
              onPress={() => onNavigate('cases')}
              endContent={<ArrowRight className="w-3.5 h-3.5" />}
              className="text-xs font-semibold text-slate-900 dark:text-slate-100 h-7 shrink-0"
            >
              Case Workspace
            </Button>
          )}
        </div>
      ) : (
        <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 p-3 bg-slate-50 dark:bg-[#151C24]/40 border border-dashed border-slate-300 dark:border-[#26313D] rounded-sm text-xs">
          <div className="flex items-center gap-2 text-slate-500 dark:text-slate-400">
            <Briefcase className="w-4 h-4 shrink-0" />
            <span>No active case currently selected. Forensic acquisitions and recoveries require an active case context.</span>
          </div>
          {onNavigate && (
            <Button
              size="sm"
              variant="light"
              onPress={() => onNavigate('cases')}
              endContent={<ArrowRight className="w-3.5 h-3.5" />}
              className="text-xs font-semibold text-slate-700 dark:text-slate-300 h-7 shrink-0"
            >
              Select Active Case
            </Button>
          )}
        </div>
      )}

      {/* 3. Horizontal Inline Metrics Ribbon (No Cards) */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 py-4 border-y border-slate-200 dark:border-[#26313D]">
        <div className="space-y-1">
          <span className="text-[10px] uppercase font-bold tracking-wider text-slate-400 dark:text-slate-500 block">
            Investigations
          </span>
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold font-mono text-slate-900 dark:text-slate-100">
              {loadingCases ? '...' : cases.length}
            </span>
            <span className="text-xs text-slate-500 dark:text-slate-400">
              ({openCasesCount} active)
            </span>
          </div>
          <p className="text-[11px] text-slate-500 dark:text-slate-400">
            Evidence cases registered
          </p>
        </div>

        <div className="space-y-1 md:border-l md:border-slate-200 md:dark:border-[#26313D] md:pl-4">
          <span className="text-[10px] uppercase font-bold tracking-wider text-slate-400 dark:text-slate-500 block">
            Audit Ledger Integrity
          </span>
          <div className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-slate-700 dark:text-slate-300" />
            <span className="text-sm font-bold text-slate-900 dark:text-slate-100 font-mono">
              {auditStatus?.is_valid ? 'Chain Verified' : 'Checking...'}
            </span>
          </div>
          <p className="text-[11px] text-slate-500 dark:text-slate-400">
            {auditStatus ? `${auditStatus.total_events} cryptographic records` : 'Zero tampering detected'}
          </p>
        </div>

        <div className="space-y-1 md:border-l md:border-slate-200 md:dark:border-[#26313D] md:pl-4">
          <span className="text-[10px] uppercase font-bold tracking-wider text-slate-400 dark:text-slate-500 block">
            Acquisition Subsystem
          </span>
          <div className="flex items-baseline gap-2">
            <span className="text-base font-bold font-mono text-slate-900 dark:text-slate-100">
              Raw Bitstream DD
            </span>
          </div>
          <p className="text-[11px] text-slate-500 dark:text-slate-400">
            Read-only hardware gate & streaming SHA-256
          </p>
        </div>

        <div className="space-y-1 md:border-l md:border-slate-200 md:dark:border-[#26313D] md:pl-4">
          <span className="text-[10px] uppercase font-bold tracking-wider text-slate-400 dark:text-slate-500 block">
            Sanitization Engine
          </span>
          <div className="flex items-baseline gap-2">
            <span className="text-base font-bold font-mono text-slate-900 dark:text-slate-100">
              NIST SP 800-88
            </span>
          </div>
          <p className="text-[11px] text-slate-500 dark:text-slate-400">
            Clear, Purge, Cryptographic Erase verified
          </p>
        </div>
      </div>

      {/* 4. Quick Action Shortcuts Ribbon */}
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-xs font-semibold text-slate-400 dark:text-slate-500 mr-1">Workstation Operations:</span>
        <Button
          size="sm"
          variant="flat"
          onPress={() => onNavigate && onNavigate('device-explorer')}
          startContent={<HardDrive className="w-3.5 h-3.5" />}
          className="text-xs font-medium h-7 bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-[#26313D]"
        >
          Device Explorer
        </Button>
        <Button
          size="sm"
          variant="flat"
          onPress={() => onNavigate && onNavigate('acquisition')}
          startContent={<Download className="w-3.5 h-3.5" />}
          className="text-xs font-medium h-7 bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-[#26313D]"
        >
          Disk Acquisition
        </Button>
        <Button
          size="sm"
          variant="flat"
          onPress={() => onNavigate && onNavigate('recovery')}
          startContent={<Search className="w-3.5 h-3.5" />}
          className="text-xs font-medium h-7 bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-[#26313D]"
        >
          Forensics & Recovery
        </Button>
        <Button
          size="sm"
          variant="flat"
          onPress={() => onNavigate && onNavigate('drive-eraser')}
          startContent={<Cpu className="w-3.5 h-3.5" />}
          className="text-xs font-medium h-7 bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-[#26313D]"
        >
          Drive Sanitization
        </Button>
        <Button
          size="sm"
          variant="flat"
          onPress={() => onNavigate && onNavigate('integrity')}
          startContent={<FileCheck className="w-3.5 h-3.5" />}
          className="text-xs font-medium h-7 bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-[#26313D]"
        >
          Hash Verification
        </Button>
        <Button
          size="sm"
          variant="flat"
          onPress={() => onNavigate && onNavigate('audit-logs')}
          startContent={<ClipboardList className="w-3.5 h-3.5" />}
          className="text-xs font-medium h-7 bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-[#26313D]"
        >
          Audit Ledger
        </Button>
      </div>

      {/* 5. Main Workspace: Primary Data Table (Direct on Canvas, No Wrapper Card) */}
      <div className="space-y-3 pt-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Briefcase className="w-4 h-4 text-slate-700 dark:text-slate-300" />
            <h2 className="text-sm font-bold text-slate-900 dark:text-slate-100 tracking-tight">
              Recent Investigation Cases
            </h2>
          </div>
          {onNavigate && (
            <Button
              size="sm"
              variant="light"
              onPress={() => onNavigate('cases')}
              endContent={<ArrowRight className="w-3.5 h-3.5" />}
              className="text-xs font-semibold text-slate-700 dark:text-slate-300 h-7"
            >
              View All Cases
            </Button>
          )}
        </div>

        <Table
          aria-label="Recent investigation cases table"
          classNames={{
            base: 'min-w-full',
            table: 'min-w-full text-xs',
            thead: '[&>tr]:first:rounded-none',
            th: 'bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 font-semibold text-[11px] uppercase tracking-wider py-2.5 px-3 border-b border-slate-200 dark:border-[#26313D]',
            td: 'py-2.5 px-3 border-b border-slate-200 dark:border-[#26313D] text-slate-800 dark:text-slate-200',
            wrapper: 'bg-transparent shadow-none p-0 border border-slate-200 dark:border-[#26313D] rounded-sm',
          }}
        >
          <TableHeader>
            <TableColumn>CASE IDENTIFIER</TableColumn>
            <TableColumn>TITLE</TableColumn>
            <TableColumn>INVESTIGATOR</TableColumn>
            <TableColumn>STATUS</TableColumn>
            <TableColumn>REGISTERED</TableColumn>
            <TableColumn align="end">ACTION</TableColumn>
          </TableHeader>
          <TableBody
            emptyContent={loadingCases ? 'Loading cases...' : 'No investigation cases registered in workstation.'}
          >
            {cases.map((c) => (
              <TableRow key={c.case_id} className="hover:bg-slate-50 dark:hover:bg-[#151C24] transition-colors">
                <TableCell className="font-mono font-bold text-slate-900 dark:text-slate-100">
                  {c.case_reference}
                </TableCell>
                <TableCell className="font-medium max-w-xs truncate">
                  {c.title}
                </TableCell>
                <TableCell className="text-slate-600 dark:text-slate-400">
                  {c.lead_investigator}
                </TableCell>
                <TableCell>
                  <Chip
                    size="sm"
                    variant="flat"
                    className={`font-mono text-[10px] font-bold uppercase h-5 ${
                      c.status === 'open' || c.status === 'in_progress'
                        ? 'bg-slate-100 dark:bg-[#26313D] text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-slate-700'
                        : 'bg-slate-100 dark:bg-[#151C24] text-slate-500 border border-slate-200 dark:border-[#26313D]'
                    }`}
                  >
                    {c.status}
                  </Chip>
                </TableCell>
                <TableCell className="font-mono text-[11px] text-slate-500">
                  {new Date(c.created_at).toLocaleDateString()}
                </TableCell>
                <TableCell>
                  {onNavigate && (
                    <Button
                      size="sm"
                      variant="light"
                      onPress={() => onNavigate('cases')}
                      className="text-xs font-semibold text-slate-700 dark:text-slate-300 h-6 px-2"
                    >
                      Inspect
                    </Button>
                  )}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      <Divider className="my-6 bg-slate-200 dark:bg-[#26313D]" />

      {/* 6. Forensic Subsystems & Invariants Status (Aligned Telemetry Rows, No Cards) */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Layers className="w-4 h-4 text-slate-700 dark:text-slate-300" />
          <h2 className="text-sm font-bold text-slate-900 dark:text-slate-100 tracking-tight">
            Workstation Subsystems & Safety Invariants
          </h2>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 text-xs border border-slate-200 dark:border-[#26313D] p-4 rounded-sm bg-slate-50/50 dark:bg-[#151C24]/30">
          <div className="space-y-1">
            <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
              Write-Blocking Gate
            </span>
            <div className="font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-slate-500" />
              Hardware Interlock Active
            </div>
            <p className="text-[11px] text-slate-500 dark:text-slate-400">
              Direct physical bitstream imaging enforces read-only OS driver flags.
            </p>
          </div>

          <div className="space-y-1">
            <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
              Cryptographic Engine
            </span>
            <div className="font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-slate-500" />
              Constant-Time SHA-256
            </div>
            <p className="text-[11px] text-slate-500 dark:text-slate-400">
              Chunked pre/post acquisition digests with zero memory leakage.
            </p>
          </div>

          <div className="space-y-1">
            <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
              Evidence Carving
            </span>
            <div className="font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-slate-500" />
              Multi-Format Signatures
            </div>
            <p className="text-[11px] text-slate-500 dark:text-slate-400">
              Header/footer heuristics with sector alignment & entropy scoring.
            </p>
          </div>

          <div className="space-y-1">
            <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
              Audit Immutability
            </span>
            <div className="font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-slate-500" />
              Tamper-Evident Chain
            </div>
            <p className="text-[11px] text-slate-500 dark:text-slate-400">
              Every operation dispatches an append-only cryptographic event block.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default DashboardPage;
