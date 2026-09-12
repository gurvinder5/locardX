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
  const [activeTab, setActiveTab] = useState<'markdown' | 'certificate'>('markdown');
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
      <div className="bg-white border border-slate-200 rounded-md p-5 shadow-2xs space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <FileText className="w-4 h-4 text-purple-600" />
            <h3 className="text-xs font-semibold text-slate-800 uppercase tracking-wider">
              Generated Case Reports
            </h3>
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-slate-100 text-slate-600 border border-slate-200">
              {reports.length}
            </span>
          </div>
        </div>

        {reports.length === 0 ? (
          <div className="py-8 text-center text-xs text-slate-400">
            No formal reports generated for this case yet. Click &quot;Generate Case Report&quot; on the overview tab to compile an evidentiary report.
          </div>
        ) : (
          <div className="divide-y divide-slate-100">
            {reports.map((r) => {
              const isSelected = activeReport?.report_id === r.report_id;
              return (
                <div
                  key={r.report_id}
                  className={`py-3 px-2 flex items-center justify-between gap-3 rounded transition-colors ${
                    isSelected ? 'bg-purple-50/50' : 'hover:bg-slate-50'
                  }`}
                >
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-bold text-slate-900">{r.title}</span>
                      <span className="text-[9px] uppercase font-bold tracking-wider px-1.5 py-0.5 rounded bg-purple-50 text-purple-700 border border-purple-200">
                        {r.report_type}
                      </span>
                    </div>
                    <div className="flex items-center gap-3 text-[10px] text-slate-400">
                      <span>By: {r.generated_by}</span>
                      <span>&bull;</span>
                      <span>{new Date(r.generated_at).toLocaleString()}</span>
                      <span>&bull;</span>
                      <span className="font-mono">Digest: {r.report_digest.slice(0, 16)}...</span>
                    </div>
                  </div>

                  <button
                    onClick={() => onViewReport(r.report_id)}
                    className="inline-flex items-center gap-1 px-2.5 py-1 text-xs font-medium rounded bg-white border border-slate-200 text-slate-700 hover:bg-slate-50 transition-colors shadow-2xs"
                  >
                    <Eye className="w-3.5 h-3.5 text-purple-600" />
                    <span>View</span>
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Active Report Inspector */}
      {activeReport && (
        <div className="bg-white border border-slate-200 rounded-md p-5 shadow-2xs space-y-4">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-slate-200">
            <div>
              <div className="flex items-center gap-2">
                <FileCheck className="w-4 h-4 text-emerald-600" />
                <h4 className="text-sm font-bold text-slate-900">{activeReport.title}</h4>
              </div>
              <p className="text-[11px] text-slate-400 font-mono mt-0.5">
                Report ID: {activeReport.report_id} &bull; Generated: {new Date(activeReport.integrity.generated_at).toLocaleString()}
              </p>
            </div>

            <div className="flex items-center gap-2">
              <button
                onClick={handleVerify}
                disabled={verifying}
                className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded bg-slate-900 text-white hover:bg-slate-800 disabled:opacity-50 transition-colors shadow-2xs"
              >
                <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
                <span>{verifying ? 'Checking...' : 'Verify Cryptographic Digest'}</span>
              </button>
            </div>
          </div>

          {/* Verification Banner */}
          {verificationResult !== null && (
            <div
              className={`p-3 rounded-md flex items-center justify-between text-xs border ${
                verificationResult
                  ? 'bg-emerald-50 text-emerald-800 border-emerald-200'
                  : 'bg-rose-50 text-rose-800 border-rose-200'
              }`}
            >
              <div className="flex items-center gap-2">
                {verificationResult ? (
                  <CheckCircle2 className="w-4 h-4 text-emerald-600" />
                ) : (
                  <AlertTriangle className="w-4 h-4 text-rose-600" />
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

          {/* Tab Navigation */}
          <div className="flex items-center justify-between border-b border-slate-200">
            <div className="flex space-x-2">
              <button
                onClick={() => setActiveTab('markdown')}
                className={`px-3 py-1.5 text-xs font-semibold border-b-2 transition-colors ${
                  activeTab === 'markdown'
                    ? 'border-purple-600 text-purple-700'
                    : 'border-transparent text-slate-500 hover:text-slate-800'
                }`}
              >
                Markdown Presentation
              </button>
              <button
                onClick={() => setActiveTab('certificate')}
                className={`px-3 py-1.5 text-xs font-semibold border-b-2 transition-colors ${
                  activeTab === 'certificate'
                    ? 'border-purple-600 text-purple-700'
                    : 'border-transparent text-slate-500 hover:text-slate-800'
                }`}
              >
                Evidentiary Text Certificate
              </button>
            </div>

            <button
              onClick={() => handleCopy(JSON.stringify(activeReport, null, 2))}
              className="inline-flex items-center gap-1 text-[11px] text-slate-500 hover:text-slate-800 transition-colors py-1"
            >
              {copied ? <Check className="w-3 h-3 text-emerald-600" /> : <Copy className="w-3 h-3" />}
              <span>{copied ? 'Copied JSON' : 'Copy JSON'}</span>
            </button>
          </div>

          {/* Report Viewer */}
          <div className="bg-slate-50 border border-slate-200 rounded-md p-4 max-h-[500px] overflow-y-auto font-mono text-xs text-slate-800 leading-relaxed whitespace-pre-wrap">
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
      )}
    </div>
  );
};
