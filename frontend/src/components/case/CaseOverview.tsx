import React from 'react';
import {
  Activity,
  HardDrive,
  Search,
  FileCheck,
  ShieldCheck,
  Clock,
  User,
  Calendar,
  Layers,
  FileText,
  RotateCcw,
  Lock,
  CheckCircle2,
  ChevronDown,
} from 'lucide-react';
import {
  Button,
  Chip,
  Dropdown,
  DropdownTrigger,
  DropdownMenu,
  DropdownItem,
} from '@heroui/react';
import { Case, CaseSummary, CaseStatus } from '../../types/case';

interface CaseOverviewProps {
  currentCase: Case;
  summary: CaseSummary | null;
  isCurrentlyActive?: boolean;
  onToggleActive?: () => void;
  onChangeStatus: (status: CaseStatus) => void;
  onReopenCase?: () => void;
  onGenerateReport: () => void;
  reportLoading: boolean;
  reportSuccess?: boolean;
}

export const CaseOverview: React.FC<CaseOverviewProps> = ({
  currentCase,
  summary,
  isCurrentlyActive = false,
  onToggleActive,
  onChangeStatus,
  onReopenCase,
  onGenerateReport,
  reportLoading,
  reportSuccess = false,
}) => {
  const isClosed = currentCase.status === 'completed' || currentCase.status === 'archived';

  return (
    <div className="space-y-5">
      {/* Case Header Banner */}
      <div className="bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] rounded-xs p-5">
          <div className="flex flex-col lg:flex-row justify-between items-start lg:items-center gap-4 pb-4 border-b border-slate-100 dark:border-[#26313D]">
            <div>
              <div className="flex items-center gap-2.5 flex-wrap">
                <Chip
                  size="sm"
                  variant="flat"
                  className="font-mono text-xs font-bold bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700"
                >
                  {currentCase.case_reference}
                </Chip>
                <h1 className="text-base font-bold text-slate-900 dark:text-white">{currentCase.title}</h1>

                {isCurrentlyActive ? (
                  <Chip
                    size="sm"
                    variant="flat"
                    className="text-[11px] font-bold bg-slate-200 dark:bg-[#26313D] text-slate-900 dark:text-white border border-slate-300 dark:border-slate-600"
                  >
                    Active App Case
                  </Chip>
                ) : onToggleActive ? (
                  <Button
                    size="sm"
                    variant="bordered"
                    onPress={onToggleActive}
                    isDisabled={isClosed}
                    className="h-6 text-[11px] px-2 border-slate-300 dark:border-[#26313D] text-slate-700 dark:text-slate-300"
                  >
                    Set as Active Case
                  </Button>
                ) : null}
              </div>
              <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 max-w-3xl leading-relaxed">
                {currentCase.description || 'No description provided.'}
              </p>
            </div>

            {/* Status & Report Controls */}
            <div className="flex items-center gap-2.5 flex-wrap">
              {/* Case Status Dropdown */}
              <Dropdown placement="bottom-end">
                <DropdownTrigger>
                  <Button
                    size="sm"
                    variant="bordered"
                    endContent={<ChevronDown className="w-3.5 h-3.5 text-slate-400" />}
                    className="h-8 text-xs font-semibold border-slate-300 dark:border-[#26313D] bg-white dark:bg-[#151C24] text-slate-800 dark:text-slate-200"
                  >
                    <span className="text-slate-400 text-[11px] font-normal mr-0.5">Status:</span>
                    <span className="uppercase tracking-wider">{currentCase.status.replace('_', ' ')}</span>
                  </Button>
                </DropdownTrigger>
                <DropdownMenu
                  aria-label="Change Investigation Status"
                  selectedKeys={[currentCase.status]}
                  selectionMode="single"
                  onAction={(key) => onChangeStatus(key as CaseStatus)}
                  classNames={{
                    base: 'bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] shadow-lg rounded-xs',
                  }}
                >
                  <DropdownItem key="open" textValue="Open">
                    <div className="flex flex-col py-0.5">
                      <span className="font-semibold text-xs text-slate-900 dark:text-white">OPEN</span>
                      <span className="text-[10px] text-slate-400">Active evidentiary investigation underway</span>
                    </div>
                  </DropdownItem>
                  <DropdownItem key="in_progress" textValue="In Progress">
                    <div className="flex flex-col py-0.5">
                      <span className="font-semibold text-xs text-slate-900 dark:text-white">IN PROGRESS</span>
                      <span className="text-[10px] text-slate-400">Forensic procedures & imaging active</span>
                    </div>
                  </DropdownItem>
                  <DropdownItem key="completed" textValue="Completed">
                    <div className="flex flex-col py-0.5">
                      <span className="font-semibold text-xs text-slate-900 dark:text-white">COMPLETED</span>
                      <span className="text-[10px] text-slate-400">Analysis completed; ready for sign-off</span>
                    </div>
                  </DropdownItem>
                  <DropdownItem key="archived" textValue="Archived">
                    <div className="flex flex-col py-0.5">
                      <span className="font-semibold text-xs text-amber-700 dark:text-amber-400">ARCHIVED</span>
                      <span className="text-[10px] text-slate-400">Read-only closed evidentiary archive</span>
                    </div>
                  </DropdownItem>
                </DropdownMenu>
              </Dropdown>

              {isClosed && onReopenCase && (
                <Button
                  size="sm"
                  color="warning"
                  onPress={onReopenCase}
                  startContent={<RotateCcw className="w-3.5 h-3.5" />}
                  className="bg-amber-600 text-white font-semibold h-8 text-xs"
                >
                  Reopen Case
                </Button>
              )}

              <Button
                size="sm"
                color="primary"
                onPress={onGenerateReport}
                isLoading={reportLoading}
                isDisabled={reportLoading || reportSuccess}
                startContent={
                  reportSuccess ? (
                    <CheckCircle2 className="w-3.5 h-3.5 text-slate-300 animate-pulse" />
                  ) : !reportLoading ? (
                    <FileCheck className="w-3.5 h-3.5" />
                  ) : undefined
                }
                className="bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-semibold h-8 shadow-xs text-xs"
              >
                {reportLoading
                  ? 'Generating Report...'
                  : reportSuccess
                  ? 'Report Generated!'
                  : 'Generate Case Report'}
              </Button>
            </div>
          </div>

          {/* Closed Case Invariant Notice */}
          {isClosed && (
            <div className="mt-3 p-3 bg-amber-50/70 dark:bg-amber-950/20 border border-amber-200 dark:border-amber-800 rounded-lg text-xs text-amber-900 dark:text-amber-200 flex items-start gap-2">
              <Lock className="w-4 h-4 text-amber-600 shrink-0 mt-0.5" />
              <div>
                <strong className="font-semibold">Case Preservation Policy Active:</strong> This case is{' '}
                <span className="font-mono">{currentCase.status}</span>. In accordance with digital
                forensic chain-of-custody standards, all evidence, reports, and operation records are
                preserved immutably. Starting new operations against this case is blocked until an
                authorized Investigator reopens it.
              </div>
            </div>
          )}

          {/* Case Metadata Badges */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 pt-3 text-xs text-slate-600 dark:text-slate-400">
            <div className="flex items-center gap-2">
              <User className="w-3.5 h-3.5 text-slate-400" />
              <span>
                Investigator: <strong className="text-slate-900 dark:text-white">{currentCase.lead_investigator}</strong>
              </span>
            </div>
            <div className="flex items-center gap-2">
              <Calendar className="w-3.5 h-3.5 text-slate-400" />
              <span>
                Created: <strong className="text-slate-900 dark:text-white">{new Date(currentCase.created_at).toLocaleDateString()}</strong>
              </span>
            </div>
            <div className="flex items-center gap-2">
              <Clock className="w-3.5 h-3.5 text-slate-400" />
              <span>
                Updated: <strong className="text-slate-900 dark:text-white">{new Date(currentCase.updated_at).toLocaleTimeString()}</strong>
              </span>
            </div>
            <div className="flex items-center gap-2">
              <ShieldCheck className="w-3.5 h-3.5 text-slate-700 dark:text-slate-300" />
              <span>
                Audit Chain:{' '}
                <strong className="text-slate-900 dark:text-white font-mono">
                  {summary?.audit_chain_status || 'Verified'}
                </strong>
              </span>
            </div>
          </div>
      </div>

      {/* Inline KPI Metric Strip */}
      <div className="border border-slate-200 dark:border-[#26313D] rounded-xs bg-white dark:bg-[#151C24] divide-y sm:divide-y-0 sm:divide-x divide-slate-200 dark:divide-[#26313D] grid grid-cols-2 md:grid-cols-3 xl:grid-cols-5 min-w-0">
        {/* Operations */}
        <div className="p-4">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 mb-1">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Operations</span>
            <Activity className="w-3.5 h-3.5 text-slate-500" />
          </div>
          <div className="text-xl font-bold text-slate-900 dark:text-white">{summary?.operation_count ?? 0}</div>
          <span className="text-[10px] text-slate-400">Linked procedures</span>
        </div>

        {/* Evidence */}
        <div className="p-4">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 mb-1">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Evidence Items</span>
            <HardDrive className="w-3.5 h-3.5 text-slate-500" />
          </div>
          <div className="text-xl font-bold text-slate-900 dark:text-white">{summary?.evidence_count ?? 0}</div>
          <span className="text-[10px] text-slate-400">Media, DD images</span>
        </div>

        {/* Recovered Files */}
        <div className="p-4">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 mb-1">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Carved Files</span>
            <Search className="w-3.5 h-3.5 text-slate-500" />
          </div>
          <div className="text-xl font-bold text-slate-900 dark:text-white">{summary?.recovered_file_count ?? 0}</div>
          <span className="text-[10px] text-slate-400">Reconstructed</span>
        </div>

        {/* Sanitization */}
        <div className="p-4">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 mb-1">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Sanitizations</span>
            <Layers className="w-3.5 h-3.5 text-slate-500" />
          </div>
          <div className="text-xl font-bold text-slate-900 dark:text-white">{summary?.erasure_count ?? 0}</div>
          <span className="text-[10px] text-slate-400">Drive & file erasures</span>
        </div>

        {/* Reports */}
        <div className="p-4">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 mb-1">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Reports</span>
            <FileText className="w-3.5 h-3.5 text-slate-500" />
          </div>
          <div className="text-xl font-bold text-slate-900 dark:text-white">{summary?.report_count ?? 0}</div>
          <span className="text-[10px] text-slate-400">Cryptographically signed</span>
        </div>
      </div>
    </div>
  );
};
