import React, { useState } from 'react';
import {
  FileText,
  ShieldCheck,
  Copy,
  Check,
  CheckCircle2,
  AlertTriangle,
  FileCheck,
  Eye,
} from 'lucide-react';
import {
  Button,
  Chip,
  Tabs,
  Tab,
} from '@heroui/react';
import { CaseForensicReport, CaseReportSummary } from '../../types/case';

interface CaseReportsViewProps {
  reports: CaseReportSummary[];
  activeReport: CaseForensicReport | null;
  onViewReport: (reportId: string) => Promise<CaseForensicReport | null>;
  onVerifyReport: (report: CaseForensicReport) => Promise<boolean>;
}

export const CaseReportsView: React.FC<CaseReportsViewProps> = ({
  reports,
  activeReport,
  onViewReport,
  onVerifyReport,
}) => {
  const [activeTab, setActiveTab] = useState<string>('markdown');
  const [verifying, setVerifying] = useState(false);
  const [verificationResult, setVerificationResult] = useState<boolean | null>(null);
  const [copied, setCopied] = useState(false);

  const handleVerify = async () => {
    if (!activeReport) return;
    try {
      setVerifying(true);
      const res = await onVerifyReport(activeReport);
      setVerificationResult(res);
    } catch (err) {
      console.error('Integrity check failed:', err);
      setVerificationResult(false);
    } finally {
      setVerifying(false);
    }
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="space-y-4">
      {/* Reports List */}
      <div className="bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] p-4 space-y-3 rounded-xs">
        <div className="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-[#26313D]">
          <div className="flex items-center gap-2">
            <FileText className="w-4 h-4 text-slate-700 dark:text-slate-300" />
            <h3 className="text-xs font-semibold text-slate-800 dark:text-slate-200 uppercase tracking-wider">
              Generated Case Reports
            </h3>
            <Chip
              size="sm"
              variant="flat"
              className="text-[10px] h-5 font-mono bg-slate-100 dark:bg-[#1B2530] text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]"
            >
              {reports.length}
            </Chip>
          </div>
        </div>

        <div>
          {reports.length === 0 ? (
            <div className="py-8 text-center text-xs text-slate-400">
              No formal reports generated for this case yet. Click &quot;Generate Case Report&quot; on the overview tab to compile an evidentiary report.
            </div>
          ) : (
            <div className="divide-y divide-slate-100 dark:divide-[#26313D]">
              {reports.map((r) => {
                const isSelected = activeReport?.report_id === r.report_id;
                return (
                  <div
                    key={r.report_id}
                    className={`py-3 px-3 flex items-center justify-between gap-3 rounded-lg transition-colors ${
                      isSelected
                        ? 'bg-slate-100 dark:bg-[#1B2530]'
                        : 'hover:bg-slate-50 dark:hover:bg-[#1B2530]/50'
                    }`}
                  >
                    <div className="space-y-1">
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-bold text-slate-900 dark:text-white">{r.title}</span>
                        <Chip size="sm" variant="flat" className="text-[9px] uppercase font-bold tracking-wider bg-slate-100 dark:bg-[#26313D] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700">
                          {r.report_type}
                        </Chip>
                      </div>
                      <div className="flex items-center gap-3 text-[10px] text-slate-400 font-mono">
                        <span>By: {r.generated_by}</span>
                        <span>&bull;</span>
                        <span>{new Date(r.generated_at).toLocaleString()}</span>
                        <span>&bull;</span>
                        <span>Digest: {r.report_digest.slice(0, 16)}...</span>
                      </div>
                    </div>

                    <Button
                      size="sm"
                      variant="bordered"
                      onPress={() => onViewReport(r.report_id)}
                      startContent={<Eye className="w-3.5 h-3.5 text-slate-700 dark:text-slate-300" />}
                      className="border-slate-200 dark:border-[#26313D] text-slate-700 dark:text-slate-300 text-xs font-semibold h-8"
                    >
                      View
                    </Button>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {/* Active Report Inspector */}
      {activeReport && (
        <div className="bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] p-4 space-y-4 rounded-xs">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-slate-200 dark:border-[#26313D]">
            <div>
              <div className="flex items-center gap-2">
                <FileCheck className="w-4 h-4 text-slate-700 dark:text-slate-300" />
                <h4 className="text-sm font-bold text-slate-900 dark:text-white">{activeReport.title}</h4>
              </div>
              <p className="text-[11px] text-slate-400 font-mono mt-0.5">
                Report ID: {activeReport.report_id} &bull; Generated: {new Date(activeReport.integrity.generated_at).toLocaleString()}
              </p>
            </div>

            <Button
              size="sm"
              color="primary"
              onPress={handleVerify}
              isLoading={verifying}
              startContent={!verifying && <ShieldCheck className="w-3.5 h-3.5 text-slate-300" />}
              className="bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-semibold h-8 shadow-xs"
            >
              {verifying ? 'Checking...' : 'Verify Cryptographic Digest'}
            </Button>
          </div>

          <div className="space-y-4">
            {/* Verification Banner */}
            {verificationResult !== null && (
              <div
                className={`p-3 rounded-lg flex items-center justify-between text-xs border ${
                  verificationResult
                    ? 'bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border-slate-300 dark:border-slate-700'
                    : 'bg-rose-50 dark:bg-rose-950/40 text-rose-800 dark:text-rose-200 border-rose-200 dark:border-rose-900'
                }`}
              >
                <div className="flex items-center gap-2">
                  {verificationResult ? (
                    <CheckCircle2 className="w-4 h-4 text-slate-700 dark:text-slate-300" />
                  ) : (
                    <AlertTriangle className="w-4 h-4 text-rose-600 dark:text-rose-400" />
                  )}
                  <span>
                    {verificationResult
                      ? 'Report digest verified! Exact match with canonical SHA-256 calculation. Zero tampering detected.'
                      : 'INTEGRITY ALERT: Report digest mismatch! Substantive fields do not match canonical digest.'}
                  </span>
                </div>
                <span className="font-mono text-[10px]">
                  {activeReport.integrity.report_digest.slice(0, 24)}...
                </span>
              </div>
            )}

            {/* Tab Navigation & Copy Control */}
            <div className="flex items-center justify-between border-b border-slate-200 dark:border-[#26313D] pb-1">
              <Tabs
                selectedKey={activeTab}
                onSelectionChange={(key) => setActiveTab(String(key))}
                size="sm"
                variant="underlined"
                classNames={{
                  cursor: 'bg-slate-900 dark:bg-white',
                  tab: 'text-xs font-semibold text-slate-500 dark:text-slate-400 data-[selected=true]:text-slate-900 dark:data-[selected=true]:text-white',
                }}
              >
                <Tab key="markdown" title="Markdown Presentation" />
                <Tab key="certificate" title="Evidentiary Text Certificate" />
              </Tabs>

              <Button
                size="sm"
                variant="light"
                onPress={() => handleCopy(JSON.stringify(activeReport, null, 2))}
                startContent={copied ? <Check className="w-3.5 h-3.5 text-slate-700 dark:text-slate-300" /> : <Copy className="w-3.5 h-3.5" />}
                className="text-[11px] text-slate-500 dark:text-slate-400 h-7"
              >
                {copied ? 'Copied JSON' : 'Copy JSON'}
              </Button>
            </div>

            {/* Report Viewer */}
            <div className="bg-slate-50 dark:bg-[#10161D] border border-slate-200 dark:border-[#26313D] rounded-lg p-4 max-h-[500px] overflow-y-auto font-mono text-xs text-slate-800 dark:text-slate-200 leading-relaxed whitespace-pre-wrap">
              {activeTab === 'markdown' ? (
                <div>
                  {`# ${activeReport.title}
Case Reference: ${activeReport.case_info.case_reference}
Lead Investigator: ${activeReport.case_info.lead_investigator}
Case Status: ${activeReport.case_info.status}
Generated: ${activeReport.integrity.generated_at}

## 1. Evidential Disk Acquisitions (${activeReport.acquisitions.length})
${
  activeReport.acquisitions.length === 0
    ? 'No acquisitions linked.'
    : activeReport.acquisitions
        .map(
          (a) =>
            `- [${a.acquisition_id}] ${a.source_display_name} (${a.image_size_bytes} bytes)\n  Path: ${a.destination_path}\n  SHA-256: ${a.image_sha256}`
        )
        .join('\n')
}

## 2. Recovery Operations (${activeReport.recoveries.length})
${
  activeReport.recoveries.length === 0
    ? 'No recovery operations linked.'
    : activeReport.recoveries
        .map(
          (r) =>
            `- Job: ${r.job_id} (${r.recovery_mode}) - Files Recovered: ${r.files_recovered} / Evaluated: ${r.candidates_evaluated}`
        )
        .join('\n')
}

## 3. Sanitization Operations (${activeReport.erasures.length})
${
  activeReport.erasures.length === 0
    ? 'No drive erasures linked.'
    : activeReport.erasures
        .map(
          (e) =>
            `- Operation: ${e.operation_id} on ${e.display_name} (${e.method}) - Outcome: ${e.verification_outcome}`
        )
        .join('\n')
}

## 4. Chain of Custody (${activeReport.custody_timeline.length})
${
  activeReport.custody_timeline.length === 0
    ? 'No custody records.'
    : activeReport.custody_timeline
        .map((c) => `[${c.timestamp}] ${c.action} (${c.actor_id}): ${c.details}`)
        .join('\n')
}

## 5. Cryptographic Integrity
Audit Chain Status: ${activeReport.audit_integrity.is_valid ? 'VERIFIED' : 'INTEGRITY COMPROMISED'}
Total Audit Sequence Count: ${activeReport.audit_integrity.total_events}
Audit Root Hash: ${activeReport.audit_integrity.audit_root_hash}
Canonical Report Digest: ${activeReport.integrity.report_digest}
`}
                </div>
              ) : (
                <div>
                  {`================================================================================
               LOCARDX FORENSIC INVESTIGATION CERTIFICATE                      
================================================================================
REPORT ID:           ${activeReport.report_id}
CASE REFERENCE:      ${activeReport.case_info.case_reference}
INVESTIGATOR:        ${activeReport.case_info.lead_investigator}
STATUS:              ${activeReport.case_info.status}
TIMESTAMP:           ${activeReport.integrity.generated_at}
--------------------------------------------------------------------------------
ACQUISITION & EVIDENCE SUMMARY:
Total Acquisitions:  ${activeReport.acquisitions.length}
Total Recoveries:    ${activeReport.recoveries.length}
Total Erasures:      ${activeReport.erasures.length}
Custody Events:      ${activeReport.custody_timeline.length}
--------------------------------------------------------------------------------
AUDIT INTEGRITY:     ${activeReport.audit_integrity.is_valid ? 'CRYPTOGRAPHICALLY VERIFIED' : 'FAILED'}
CANONICAL DIGEST:    ${activeReport.integrity.report_digest}
================================================================================`}
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
