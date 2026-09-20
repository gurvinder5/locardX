import React, { useState } from 'react';
import {
  Briefcase,
  Activity,
  HardDrive,
  ShieldCheck,
  FileText,
  Plus,
} from 'lucide-react';
import { useCases } from '../hooks/useCases';
import { CaseList } from '../components/case/CaseList';
import { CaseOverview } from '../components/case/CaseOverview';
import { CaseOperationsView } from '../components/case/CaseOperationsView';
import { CaseEvidenceView } from '../components/case/CaseEvidenceView';
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
    activeReport,
    loading,
    selectCase,
    createNewCase,
    changeCaseStatus,
    linkOperation,
    linkEvidence,
    addCustodyRecord,
    produceCaseReport,
    viewReport,
    verifyReport,
  } = useCases();

  const [activeSubTab, setActiveSubTab] = useState<'operations' | 'evidence' | 'custody' | 'reports'>('operations');
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [reportLoading, setReportLoading] = useState(false);

  const handleGenerateReport = async () => {
    if (!activeCase) return;
    try {
      setReportLoading(true);
      await produceCaseReport(activeCase.case_id);
      setActiveSubTab('reports');
    } catch (err) {
      console.error('Failed to generate report:', err);
    } finally {
      setReportLoading(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left Column: Case Browser */}
        <div className="lg:col-span-4">
          <CaseList
            cases={cases}
            activeCaseId={activeCase?.case_id || null}
            onSelectCase={selectCase}
            onOpenCreateModal={() => setIsCreateModalOpen(true)}
            loading={loading}
          />
        </div>

        {/* Right Column: Active Case Workspace */}
        <div className="lg:col-span-8 space-y-6">
          {activeCase ? (
            <>
              {/* Overview & KPI */}
              <CaseOverview
                currentCase={activeCase}
                summary={summary}
                onChangeStatus={(st: CaseStatus) => changeCaseStatus(activeCase.case_id, st)}
                onGenerateReport={handleGenerateReport}
                reportLoading={reportLoading}
              />

              {/* Sub-tab Navigation */}
              <div className="bg-white border border-slate-200 rounded-md p-1 flex space-x-1 shadow-2xs">
                <button
                  onClick={() => setActiveSubTab('operations')}
                  className={`flex-1 py-1.5 px-3 rounded text-xs font-semibold flex items-center justify-center gap-2 transition-colors ${
                    activeSubTab === 'operations'
                      ? 'bg-slate-900 text-white shadow-2xs'
                      : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
                  }`}
                >
                  <Activity className="w-3.5 h-3.5 text-indigo-400" />
                  <span>Operations ({operations.length})</span>
                </button>

                <button
                  onClick={() => setActiveSubTab('evidence')}
                  className={`flex-1 py-1.5 px-3 rounded text-xs font-semibold flex items-center justify-center gap-2 transition-colors ${
                    activeSubTab === 'evidence'
                      ? 'bg-slate-900 text-white shadow-2xs'
                      : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
                  }`}
                >
                  <HardDrive className="w-3.5 h-3.5 text-sky-400" />
                  <span>Evidence ({evidence.length})</span>
                </button>

                <button
                  onClick={() => setActiveSubTab('custody')}
                  className={`flex-1 py-1.5 px-3 rounded text-xs font-semibold flex items-center justify-center gap-2 transition-colors ${
                    activeSubTab === 'custody'
                      ? 'bg-slate-900 text-white shadow-2xs'
                      : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
                  }`}
                >
                  <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
                  <span>Chain of Custody ({timeline.length})</span>
                </button>

                <button
                  onClick={() => setActiveSubTab('reports')}
                  className={`flex-1 py-1.5 px-3 rounded text-xs font-semibold flex items-center justify-center gap-2 transition-colors ${
                    activeSubTab === 'reports'
                      ? 'bg-slate-900 text-white shadow-2xs'
                      : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
                  }`}
                >
                  <FileText className="w-3.5 h-3.5 text-purple-400" />
                  <span>Reports ({reports.length})</span>
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
            <div className="bg-white border border-slate-200 rounded-md p-16 text-center space-y-3 shadow-2xs">
              <Briefcase className="w-12 h-12 text-slate-300 mx-auto" />
              <h3 className="text-sm font-semibold text-slate-800">No Investigation Case Selected</h3>
              <p className="text-xs text-slate-500 max-w-sm mx-auto">
                Select an existing investigation case from the list on the left or create a new case to link evidential media and operations.
              </p>
              <button
                onClick={() => setIsCreateModalOpen(true)}
                className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded bg-sky-600 text-white hover:bg-sky-700 transition-colors shadow-2xs"
              >
                <Plus className="w-3.5 h-3.5" />
                <span>Create New Investigation Case</span>
              </button>
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

export default CaseManagementPage;
