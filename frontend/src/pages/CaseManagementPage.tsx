import React, { useState, useRef } from 'react';
import {
  Briefcase,
  Activity,
  HardDrive,
  ShieldCheck,
  FileText,
  Plus,
  Search,
  Layers,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  PanelLeftOpen,
} from 'lucide-react';
import {
  Button,
  Chip,
} from '@heroui/react';
import { useCases } from '../hooks/useCases';
import { useCaseStore } from '../stores/caseStore';
import { CaseList } from '../components/case/CaseList';
import { CaseOverview } from '../components/case/CaseOverview';
import { CaseOperationsView } from '../components/case/CaseOperationsView';
import { CaseEvidenceView } from '../components/case/CaseEvidenceView';
import { CaseAcquisitionsView } from '../components/case/CaseAcquisitionsView';
import { CaseRecoveriesView } from '../components/case/CaseRecoveriesView';
import { CaseErasuresView } from '../components/case/CaseErasuresView';
import { CaseCustodyTimeline } from '../components/case/CaseCustodyTimeline';
import { CaseReportsView } from '../components/case/CaseReportsView';
import { CreateCaseModal } from '../components/case/CreateCaseModal';
import { CaseStatus } from '../types/case';

export const CaseManagementPage: React.FC = () => {
  const {
    cases,
    activeCase,
    summary,
    operations,
    evidence,
    timeline,
    reports,
    acquisitions,
    recoveries,
    erasures,
    activeReport,
    loading,
    selectCase,
    createNewCase,
    changeCaseStatus,
    reopenActiveCase,
    linkOperation,
    linkEvidence,
    addCustodyRecord,
    produceCaseReport,
    viewReport,
    verifyReport,
  } = useCases();

  const { activeCase: globalActiveCase, setActiveCase: setGlobalActiveCase } = useCaseStore();

  const [activeSubTab, setActiveSubTab] = useState<string>('operations');
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [reportLoading, setReportLoading] = useState(false);
  const [reportSuccess, setReportSuccess] = useState(false);
  const [isCaseListCollapsed, setIsCaseListCollapsed] = useState(false);

  const tabRailRef = useRef<HTMLDivElement>(null);

  const scrollTabRail = (direction: 'left' | 'right') => {
    if (tabRailRef.current) {
      const scrollAmount = 220;
      tabRailRef.current.scrollBy({
        left: direction === 'left' ? -scrollAmount : scrollAmount,
        behavior: 'smooth',
      });
    }
  };

  const handleTabRailWheel = (e: React.WheelEvent<HTMLDivElement>) => {
    if (tabRailRef.current && (e.deltaY !== 0 || e.deltaX !== 0)) {
      e.stopPropagation();
      tabRailRef.current.scrollLeft += e.deltaY !== 0 ? e.deltaY : e.deltaX;
    }
  };

  const subTabs = [
    { id: 'operations', label: 'Operations', count: operations.length, icon: Activity },
    { id: 'evidence', label: 'Evidence', count: evidence.length, icon: HardDrive },
    { id: 'acquisitions', label: 'Acquisitions', count: acquisitions.length, icon: HardDrive },
    { id: 'recoveries', label: 'Recoveries', count: recoveries.length, icon: Search },
    { id: 'erasures', label: 'Erasures', count: erasures.length, icon: Layers },
    { id: 'custody', label: 'Custody', count: timeline.length, icon: ShieldCheck },
    { id: 'reports', label: 'Reports', count: reports.length, icon: FileText },
  ];

  const handleGenerateReport = async () => {
    if (!activeCase) return;
    try {
      setReportLoading(true);
      setReportSuccess(false);
      await produceCaseReport(activeCase.case_id);
      setReportSuccess(true);
      setActiveSubTab('reports');
      setTimeout(() => {
        setReportSuccess(false);
      }, 4000);
    } catch (err) {
      console.error('Failed to generate report:', err);
    } finally {
      setReportLoading(false);
    }
  };

  const isSelectedCaseActive = activeCase?.case_id === globalActiveCase?.case_id;

  return (
    <div className="space-y-6 w-full min-w-0">
      <div className="flex flex-col lg:flex-row gap-6 min-w-0 items-start">
        {/* Left Column: Case Browser / Minimized Rail */}
        {isCaseListCollapsed ? (
          <div className="hidden lg:flex flex-col items-center py-3 px-1 bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] rounded-xs w-11 shrink-0 h-[760px] transition-all">
            <Button
              isIconOnly
              size="sm"
              variant="light"
              onPress={() => setIsCaseListCollapsed(false)}
              className="text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white h-8 w-8 min-w-8"
              title="Expand Investigations & Cases"
            >
              <PanelLeftOpen className="w-4 h-4" />
            </Button>
            <div className="my-2.5 w-6 border-b border-slate-200 dark:border-[#26313D]" />
            <div
              className="flex-1 flex flex-col items-center gap-3 cursor-pointer py-2 group"
              onClick={() => setIsCaseListCollapsed(false)}
              title="Click to expand Investigations & Cases"
            >
              <Briefcase className="w-4 h-4 text-slate-500 group-hover:text-slate-800 dark:group-hover:text-slate-200" />
              <span className="text-[10px] font-mono font-bold text-slate-600 dark:text-slate-400 px-1 py-0.5 rounded bg-slate-100 dark:bg-[#1B2530] border border-slate-200 dark:border-[#26313D]">
                {cases.length}
              </span>
              <span
                style={{ writingMode: 'vertical-rl' }}
                className="text-[10px] font-bold tracking-widest text-slate-400 group-hover:text-slate-700 dark:group-hover:text-slate-200 uppercase select-none mt-2 rotate-180"
              >
                Cases
              </span>
            </div>
          </div>
        ) : (
          <div className="w-full lg:w-[350px] xl:w-[380px] shrink-0 min-w-0 transition-all">
            <CaseList
              cases={cases}
              activeCaseId={activeCase?.case_id || null}
              onSelectCase={selectCase}
              onOpenCreateModal={() => setIsCreateModalOpen(true)}
              loading={loading}
              onToggleCollapse={() => setIsCaseListCollapsed(true)}
            />
          </div>
        )}

        {/* Right Column: Active Case Workspace (Fills 100% width when collapsed) */}
        <div className="flex-1 min-w-0 space-y-6 w-full">
          {activeCase ? (
            <>
              {/* Overview & KPI */}
              <CaseOverview
                currentCase={activeCase}
                summary={summary}
                isCurrentlyActive={isSelectedCaseActive}
                onToggleActive={async () => {
                  if (activeCase) {
                    await setGlobalActiveCase(activeCase.case_id);
                  }
                }}
                onChangeStatus={(st: CaseStatus) => changeCaseStatus(activeCase.case_id, st)}
                onReopenCase={async () => {
                  await reopenActiveCase(activeCase.case_id);
                }}
                onGenerateReport={handleGenerateReport}
                reportLoading={reportLoading}
                reportSuccess={reportSuccess}
              />

              {/* Report Generation Success Confirmation Alert */}
              {reportSuccess && (
                <div className="bg-slate-100 dark:bg-[#1B2530] border border-slate-300 dark:border-[#26313D] text-slate-900 dark:text-white text-xs px-4 py-3 rounded-lg flex items-center justify-between shadow-2xs transition-all">
                  <div className="flex items-center gap-2.5">
                    <CheckCircle2 className="w-4 h-4 text-slate-700 dark:text-slate-300 shrink-0" />
                    <div>
                      <span className="font-bold">Case Report Generated Successfully!</span>
                      <p className="text-[11px] text-slate-600 dark:text-slate-400 mt-0.5">
                        A tamper-evident, cryptographically signed investigation report has been compiled and saved to the case archive below.
                      </p>
                    </div>
                  </div>
                  <Chip size="sm" variant="flat" className="text-[10px] uppercase font-bold tracking-wider bg-slate-200 dark:bg-[#26313D] text-slate-800 dark:text-slate-200 shrink-0">
                    Signed & Verified
                  </Chip>
                </div>
              )}

              {/* Sub-tab Horizontal Sliding Navigation Rail */}
              <div className="relative border border-slate-200 dark:border-[#26313D] bg-slate-100/70 dark:bg-[#10161D] rounded-xs p-1 flex items-center min-w-0">
                <button
                  type="button"
                  onClick={() => scrollTabRail('left')}
                  className="hidden sm:flex items-center justify-center w-6 h-7 text-slate-500 hover:text-slate-900 dark:hover:text-white bg-slate-200/50 dark:bg-[#151C24] hover:bg-slate-200 dark:hover:bg-[#1B2530] rounded-xs transition-colors shrink-0 mr-1 cursor-pointer"
                  title="Scroll tabs left"
                  aria-label="Scroll tabs left"
                >
                  <ChevronLeft className="w-3.5 h-3.5" />
                </button>

                <div
                  ref={tabRailRef}
                  onWheel={handleTabRailWheel}
                  style={{
                    scrollbarWidth: 'none',
                    msOverflowStyle: 'none',
                  }}
                  className="flex items-center gap-1.5 overflow-x-auto overflow-y-hidden overscroll-x-contain scrollbar-none no-scrollbar scroll-smooth min-w-0 w-full py-0.5 px-0.5 select-none"
                >
                  {subTabs.map((tab) => {
                    const isActive = activeSubTab === tab.id;
                    const Icon = tab.icon;
                    return (
                      <button
                        key={tab.id}
                        type="button"
                        onClick={(e) => {
                          setActiveSubTab(tab.id);
                          e.currentTarget.scrollIntoView({
                            behavior: 'smooth',
                            block: 'nearest',
                            inline: 'center',
                          });
                        }}
                        className={`inline-flex items-center gap-2 px-3 py-1.5 rounded-xs text-xs transition-all shrink-0 cursor-pointer ${
                          isActive
                            ? 'bg-white dark:bg-[#1B2530] text-slate-900 dark:text-slate-100 font-bold border border-slate-300 dark:border-[#26313D] shadow-2xs'
                            : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 hover:bg-slate-200/60 dark:hover:bg-[#151C24] border border-transparent'
                        }`}
                      >
                        <Icon className={`w-3.5 h-3.5 shrink-0 ${isActive ? 'text-slate-900 dark:text-slate-100' : 'text-slate-500 dark:text-slate-400'}`} />
                        <span>{tab.label}</span>
                        <span
                          className={`text-[10px] font-mono px-1.5 py-0.2 rounded-xs ${
                            isActive
                              ? 'bg-slate-100 dark:bg-[#151C24] text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-[#26313D] font-bold'
                              : 'bg-slate-200/60 dark:bg-[#151C24] text-slate-500 dark:text-slate-400'
                          }`}
                        >
                          {tab.count}
                        </span>
                      </button>
                    );
                  })}
                </div>

                <button
                  type="button"
                  onClick={() => scrollTabRail('right')}
                  className="hidden sm:flex items-center justify-center w-6 h-7 text-slate-500 hover:text-slate-900 dark:hover:text-white bg-slate-200/50 dark:bg-[#151C24] hover:bg-slate-200 dark:hover:bg-[#1B2530] rounded-xs transition-colors shrink-0 ml-1 cursor-pointer"
                  title="Scroll tabs right"
                  aria-label="Scroll tabs right"
                >
                  <ChevronRight className="w-3.5 h-3.5" />
                </button>
              </div>

              {/* Sub-tab Content Views */}
              {activeSubTab === 'operations' && (
                <CaseOperationsView
                  operations={operations}
                  onLinkOperation={async (opId, opType, notes) => {
                    await linkOperation(activeCase.case_id, opId, opType, notes);
                  }}
                />
              )}

              {activeSubTab === 'evidence' && (
                <CaseEvidenceView
                  evidence={evidence}
                  onAddEvidence={async (req) => {
                    await linkEvidence(activeCase.case_id, req);
                  }}
                />
              )}

              {activeSubTab === 'acquisitions' && (
                <CaseAcquisitionsView acquisitions={acquisitions} />
              )}

              {activeSubTab === 'recoveries' && (
                <CaseRecoveriesView recoveries={recoveries} />
              )}

              {activeSubTab === 'erasures' && (
                <CaseErasuresView erasures={erasures} />
              )}

              {activeSubTab === 'custody' && (
                <CaseCustodyTimeline
                  timeline={timeline}
                  onRecordCustody={async (req) => {
                    await addCustodyRecord(activeCase.case_id, req);
                  }}
                />
              )}

              {activeSubTab === 'reports' && (
                <CaseReportsView
                  reports={reports}
                  activeReport={activeReport}
                  onViewReport={viewReport}
                  onVerifyReport={verifyReport}
                />
              )}
            </>
          ) : (
            <div className="bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] p-16 text-center space-y-3 rounded-xs">
              <div className="flex flex-col items-center justify-center space-y-3">
                <Briefcase className="w-12 h-12 text-slate-300 dark:text-slate-600 mx-auto" />
                <h3 className="text-sm font-semibold text-slate-900 dark:text-white">No Investigation Case Selected</h3>
                <p className="text-xs text-slate-500 dark:text-slate-400 max-w-sm mx-auto">
                  Select an existing investigation case from the list on the left or create a new case to link evidential media and operations.
                </p>
                <div className="flex items-center gap-2 justify-center flex-wrap pt-1">
                  <Button
                    color="primary"
                    size="sm"
                    onPress={() => setIsCreateModalOpen(true)}
                    startContent={<Plus className="w-4 h-4" />}
                    className="bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-semibold"
                  >
                    Create New Investigation Case
                  </Button>
                  {isCaseListCollapsed && (
                    <Button
                      variant="bordered"
                      size="sm"
                      onPress={() => setIsCaseListCollapsed(false)}
                      startContent={<PanelLeftOpen className="w-4 h-4" />}
                      className="border-slate-300 dark:border-[#26313D] text-slate-700 dark:text-slate-300 font-semibold"
                    >
                      Browse Cases List
                    </Button>
                  )}
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Modal Dialog */}
      <CreateCaseModal
        isOpen={isCreateModalOpen}
        onClose={() => setIsCreateModalOpen(false)}
        onSubmit={createNewCase}
      />
    </div>
  );
};
