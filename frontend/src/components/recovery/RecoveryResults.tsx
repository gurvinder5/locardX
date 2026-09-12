import React, { useState } from 'react';
import {
  Download,
  Search,
  CheckCircle2,
  FileText,
  ShieldCheck,
  FolderDown,
  X,
} from 'lucide-react';
import { RecoveryReport, RecoveryResult } from '../../types/recovery';
import { RecoveredFileCard } from './RecoveredFileCard';
import { getRecoveryReport } from '../../services/recovery';

interface RecoveryResultsProps {
  result: RecoveryResult;
  onExport: (jobId: string, exportDir: string) => Promise<number>;
  onNewJob: () => void;
}

export const RecoveryResults: React.FC<RecoveryResultsProps> = ({
  result,
  onExport,
  onNewJob,
}) => {
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [exportDir, setExportDir] = useState<string>('C:\\RecoveredFiles');
  const [isExporting, setIsExporting] = useState<boolean>(false);
  const [exportedCount, setExportedCount] = useState<number | null>(null);
  const [report, setReport] = useState<RecoveryReport | null>(null);
  const [showReportModal, setShowReportModal] = useState<boolean>(false);
  const [exportError, setExportError] = useState<string | null>(null);

  const categories = [
    { key: 'all', label: 'All Categories' },
    { key: 'images', label: 'Images' },
    { key: 'documents', label: 'Documents' },
    { key: 'archives', label: 'Archives' },
    { key: 'databases', label: 'Databases' },
    { key: 'text_files', label: 'Text Files' },
  ];

  const filteredFiles = result.recovered_files.filter((file) => {
    const matchesCategory =
      selectedCategory === 'all' ||
      file.category.toLowerCase() === selectedCategory.toLowerCase();
    const matchesSearch =
      searchQuery.trim() === '' ||
      file.suggested_filename.toLowerCase().includes(searchQuery.toLowerCase()) ||
      file.file_type.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  const avgConfidence =
    result.recovered_files.length > 0
      ? Math.round(
          result.recovered_files.reduce((acc, f) => acc + f.confidence_score, 0) /
            result.recovered_files.length
        )
      : 0;

  const handleExport = async () => {
    if (!exportDir.trim()) return;
    setIsExporting(true);
    setExportError(null);
    try {
      const count = await onExport(result.job_id, exportDir);
      setExportedCount(count);
    } catch (err) {
      setExportError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsExporting(false);
    }
  };

  const handleViewReport = async () => {
    try {
      const rep = await getRecoveryReport(result.job_id);
      if (rep) {
        setReport(rep);
        setShowReportModal(true);
      }
    } catch {
      // ignore
    }
  };

  return (
    <div className="space-y-4">
      {/* Header Metric Cards */}
      <div className="grid grid-cols-4 gap-3">
        <div className="p-3.5 bg-white border border-slate-200 rounded-md shadow-2xs">
          <div className="text-[10px] text-slate-400 font-semibold uppercase tracking-wider">
            Files Recovered
          </div>
          <div className="text-xl font-bold text-slate-900 font-mono mt-0.5">
            {result.files_recovered}
          </div>
        </div>
        <div className="p-3.5 bg-white border border-slate-200 rounded-md shadow-2xs">
          <div className="text-[10px] text-slate-400 font-semibold uppercase tracking-wider">
            Candidates Evaluated
          </div>
          <div className="text-xl font-bold text-slate-700 font-mono mt-0.5">
            {result.candidates_evaluated}
          </div>
        </div>
        <div className="p-3.5 bg-white border border-slate-200 rounded-md shadow-2xs">
          <div className="text-[10px] text-slate-400 font-semibold uppercase tracking-wider">
            Avg Confidence
          </div>
          <div className="text-xl font-bold text-emerald-600 font-mono mt-0.5">
            {avgConfidence}%
          </div>
        </div>
        <div className="p-3.5 bg-white border border-slate-200 rounded-md shadow-2xs">
          <div className="text-[10px] text-slate-400 font-semibold uppercase tracking-wider">
            Elapsed Time
          </div>
          <div className="text-xl font-bold text-slate-700 font-mono mt-0.5">
            {result.elapsed_seconds.toFixed(2)}s
          </div>
        </div>
      </div>

      {/* Export Bar */}
      <div className="p-4 bg-slate-50 border border-slate-200 rounded-md space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <FolderDown className="w-4 h-4 text-indigo-600" />
            <span className="text-xs font-semibold text-slate-800">
              Export Recovered Evidence Files
            </span>
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={handleViewReport}
              className="text-xs font-medium text-indigo-600 hover:text-indigo-800 flex items-center gap-1"
            >
              <FileText className="w-3.5 h-3.5" />
              View Forensic Report
            </button>
            <button
              onClick={onNewJob}
              className="text-xs font-medium text-slate-600 hover:text-slate-800 border border-slate-200 px-2 py-1 rounded bg-white"
            >
              New Job
            </button>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <input
            type="text"
            value={exportDir}
            onChange={(e) => setExportDir(e.target.value)}
            placeholder="Destination directory path..."
            className="flex-1 px-3 py-1.5 text-xs border border-slate-300 rounded bg-white font-mono focus:outline-none focus:ring-1 focus:ring-indigo-500"
          />
          <button
            onClick={handleExport}
            disabled={isExporting || !exportDir.trim()}
            className="px-3.5 py-1.5 bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-medium rounded flex items-center gap-1.5 transition-colors disabled:opacity-50"
          >
            <Download className="w-3.5 h-3.5" />
            {isExporting ? 'Exporting...' : 'Export Files'}
          </button>
        </div>
        {exportedCount !== null && (
          <div className="text-[11px] text-emerald-700 font-medium flex items-center gap-1.5 pt-1">
            <CheckCircle2 className="w-3.5 h-3.5" />
            Successfully exported {exportedCount} files into {exportDir} organized by category folders.
          </div>
        )}
        {exportError && (
          <div className="text-[11px] text-rose-600 font-medium pt-1">
            Export failed: {exportError}
          </div>
        )}
      </div>

      {/* Filter and Search Controls */}
      <div className="flex items-center justify-between gap-3 pt-2">
        <div className="flex items-center gap-1 overflow-x-auto pb-1">
          {categories.map((cat) => (
            <button
              key={cat.key}
              onClick={() => setSelectedCategory(cat.key)}
              className={`px-2.5 py-1 text-xs font-medium rounded transition-colors whitespace-nowrap ${
                selectedCategory === cat.key
                  ? 'bg-slate-900 text-white font-semibold'
                  : 'bg-white border border-slate-200 text-slate-600 hover:bg-slate-50'
              }`}
            >
              {cat.label}
            </button>
          ))}
        </div>
        <div className="relative w-64">
          <Search className="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-2.5" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search recovered files..."
            className="w-full pl-8 pr-3 py-1.5 text-xs border border-slate-200 rounded-md bg-white focus:outline-none focus:ring-1 focus:ring-slate-400"
          />
        </div>
      </div>

      {/* Recovered File Cards List */}
      <div className="space-y-2">
        {filteredFiles.length === 0 ? (
          <div className="p-8 text-center bg-white border border-slate-200 rounded-md text-slate-400 text-xs">
            No recovered files match current filter criteria.
          </div>
        ) : (
          filteredFiles.map((file) => <RecoveredFileCard key={file.file_id} file={file} />)
        )}
      </div>

      {/* Report Modal */}
      {showReportModal && report && (
        <div className="fixed inset-0 bg-slate-900/40 backdrop-blur-2xs flex items-center justify-center p-4 z-50">
          <div className="bg-white rounded-lg border border-slate-200 max-w-lg w-full p-5 space-y-4 shadow-xl">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3">
              <div className="flex items-center gap-2">
                <ShieldCheck className="w-5 h-5 text-emerald-600" />
                <h3 className="text-sm font-semibold text-slate-900">
                  Forensic Recovery Report
                </h3>
              </div>
              <button
                onClick={() => setShowReportModal(false)}
                className="text-slate-400 hover:text-slate-600"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="space-y-2 text-xs text-slate-600">
              <div className="flex justify-between py-1 border-b border-slate-50">
                <span className="text-slate-500">Report ID:</span>
                <span className="font-mono">{report.report_id}</span>
              </div>
              <div className="flex justify-between py-1 border-b border-slate-50">
                <span className="text-slate-500">Job ID:</span>
                <span className="font-mono">{report.job_id}</span>
              </div>
              <div className="flex justify-between py-1 border-b border-slate-50">
                <span className="text-slate-500">Total Files Recovered:</span>
                <span className="font-bold">{report.total_files_recovered}</span>
              </div>
              <div className="flex justify-between py-1 border-b border-slate-50">
                <span className="text-slate-500">Average Confidence:</span>
                <span className="font-bold text-emerald-600">{report.average_confidence.toFixed(1)}%</span>
              </div>
              <div className="py-1">
                <span className="text-slate-500 block mb-1">Report Cryptographic Digest:</span>
                <span className="font-mono text-[10px] break-all bg-slate-50 p-1.5 rounded block border border-slate-100">
                  {report.report_digest}
                </span>
              </div>
              <div className="py-1">
                <span className="text-slate-500 block mb-1">Source Image SHA-256:</span>
                <span className="font-mono text-[10px] break-all bg-slate-50 p-1.5 rounded block border border-slate-100">
                  {report.source_image_sha256}
                </span>
              </div>
            </div>

            <div className="pt-2 flex justify-end">
              <button
                onClick={() => setShowReportModal(false)}
                className="px-4 py-1.5 bg-slate-900 hover:bg-slate-800 text-white text-xs font-medium rounded"
              >
                Close Report
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
