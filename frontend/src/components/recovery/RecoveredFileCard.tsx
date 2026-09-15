import React, { useState } from 'react';
import {
  FileText,
  Image as ImageIcon,
  Archive,
  Database,
  FileCode,
  File,
  Film,
  CheckCircle2,
  XCircle,
  ChevronDown,
  ChevronUp,
  ShieldCheck,
  ExternalLink,
  FolderOpen,
  Loader2,
  AlertTriangle,
} from 'lucide-react';
import { RecoveredFile, ValidationStatus, ConfidenceGrade } from '../../types/recovery';
import { openRecoveredFile, revealRecoveredFile } from '../../services/recovery';

interface RecoveredFileCardProps {
  file: RecoveredFile;
}

export const RecoveredFileCard: React.FC<RecoveredFileCardProps> = ({ file }) => {
  const [expanded, setExpanded] = useState<boolean>(false);
  const [actionState, setActionState] = useState<{
    loading: boolean;
    type?: 'open' | 'reveal';
    error?: string | null;
  }>({ loading: false });

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  };

  const getCategoryIcon = (cat: string) => {
    switch (cat.toLowerCase()) {
      case 'images':
        return <ImageIcon className="w-4 h-4 text-sky-600" />;
      case 'documents':
        return <FileText className="w-4 h-4 text-blue-600" />;
      case 'archives':
        return <Archive className="w-4 h-4 text-amber-600" />;
      case 'media':
        return <Film className="w-4 h-4 text-violet-600" />;
      case 'databases':
        return <Database className="w-4 h-4 text-purple-600" />;
      case 'text_files':
        return <FileCode className="w-4 h-4 text-emerald-600" />;
      default:
        return <File className="w-4 h-4 text-slate-600" />;
    }
  };

  const getConfidenceBadge = (grade: ConfidenceGrade, score: number) => {
    switch (grade) {
      case 'high':
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-emerald-50 text-emerald-700 border border-emerald-200 flex items-center gap-1">
            <ShieldCheck className="w-3 h-3" />
            High ({score}%)
          </span>
        );
      case 'medium':
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-amber-50 text-amber-700 border border-amber-200">
            Medium ({score}%)
          </span>
        );
      case 'low':
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-orange-50 text-orange-700 border border-orange-200">
            Low ({score}%)
          </span>
        );
      case 'uncertain':
      default:
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-rose-50 text-rose-700 border border-rose-200">
            Uncertain ({score}%)
          </span>
        );
    }
  };

  const getValidationBadge = (status: ValidationStatus) => {
    switch (status) {
      case 'valid':
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-emerald-50 text-emerald-600 border border-emerald-200">
            Valid
          </span>
        );
      case 'partially_valid':
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-50 text-amber-600 border border-amber-200">
            Partially Valid
          </span>
        );
      case 'incomplete':
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-orange-50 text-orange-600 border border-orange-200">
            Incomplete
          </span>
        );
      case 'corrupted':
      case 'invalid':
      default:
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-rose-50 text-rose-600 border border-rose-200">
            Corrupted
          </span>
        );
    }
  };

  const handleOpenFile = async () => {
    setActionState({ loading: true, type: 'open', error: null });
    try {
      await openRecoveredFile(file.job_id, file.file_id);
      setActionState({ loading: false });
    } catch (err: unknown) {
      const msg = err && typeof err === 'object' && 'message' in err
        ? String((err as { message: string }).message)
        : String(err);
      setActionState({ loading: false, error: msg });
    }
  };

  const handleRevealFile = async () => {
    setActionState({ loading: true, type: 'reveal', error: null });
    try {
      await revealRecoveredFile(file.job_id, file.file_id);
      setActionState({ loading: false });
    } catch (err: unknown) {
      const msg = err && typeof err === 'object' && 'message' in err
        ? String((err as { message: string }).message)
        : String(err);
      setActionState({ loading: false, error: msg });
    }
  };

  return (
    <div className="border border-slate-200 rounded-md bg-white p-3 space-y-2 hover:border-slate-300 transition-colors shadow-2xs">
      <div className="flex items-start justify-between">
        <div className="flex items-start space-x-2.5">
          <div className="p-1.5 rounded bg-slate-50 border border-slate-100 mt-0.5">
            {getCategoryIcon(file.category)}
          </div>
          <div>
            <div className="flex items-center gap-2">
              <span className="text-xs font-semibold text-slate-900 font-mono">
                {file.suggested_filename}
              </span>
              {getValidationBadge(file.validation_status)}
            </div>
            <div className="text-[11px] text-slate-500 mt-0.5 flex flex-wrap gap-x-3 gap-y-0.5">
              <span>Offset: <strong className="font-mono text-slate-700">0x{file.source_offset.toString(16).toUpperCase()}</strong></span>
              <span>Size: <strong className="text-slate-700">{formatBytes(file.size_bytes)}</strong></span>
              <span>Type: <strong className="text-slate-700">{file.file_type}</strong></span>
              <span>Method: <strong className="text-slate-700">{file.recovery_method.replace('_', ' ')}</strong></span>
            </div>
          </div>
        </div>
        <div className="flex items-center space-x-2">
          {getConfidenceBadge(file.confidence_grade, file.confidence_score)}
          <button
            onClick={() => setExpanded(!expanded)}
            className="p-1 text-slate-400 hover:text-slate-600 rounded cursor-pointer"
            title="Toggle evidence factors"
          >
            {expanded ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
          </button>
        </div>
      </div>

      <div className="flex items-center justify-between pt-1 border-t border-slate-100">
        <div className="text-[10px] font-mono text-slate-400 truncate max-w-xs sm:max-w-md">
          SHA-256: {file.sha256_hash}
        </div>
        <div className="flex items-center gap-1.5 shrink-0">
          <button
            onClick={handleOpenFile}
            disabled={actionState.loading}
            className="px-2 py-1 text-[11px] font-medium text-slate-700 bg-slate-100 hover:bg-slate-200 border border-slate-200 rounded flex items-center gap-1 transition-colors cursor-pointer disabled:opacity-50"
            title="Open file using default system application"
          >
            {actionState.loading && actionState.type === 'open' ? (
              <Loader2 className="w-3 h-3 animate-spin text-slate-600" />
            ) : (
              <ExternalLink className="w-3 h-3 text-slate-600" />
            )}
            Open
          </button>
          <button
            onClick={handleRevealFile}
            disabled={actionState.loading}
            className="px-2 py-1 text-[11px] font-medium text-slate-700 bg-slate-100 hover:bg-slate-200 border border-slate-200 rounded flex items-center gap-1 transition-colors cursor-pointer disabled:opacity-50"
            title="Reveal in OS file manager"
          >
            {actionState.loading && actionState.type === 'reveal' ? (
              <Loader2 className="w-3 h-3 animate-spin text-slate-600" />
            ) : (
              <FolderOpen className="w-3 h-3 text-slate-600" />
            )}
            Reveal
          </button>
        </div>
      </div>

      {actionState.error && (
        <div className="text-[11px] p-2 rounded bg-rose-50 border border-rose-200 text-rose-800 flex items-center gap-1.5">
          <AlertTriangle className="w-3.5 h-3.5 text-rose-600 shrink-0" />
          <span>{actionState.error}</span>
        </div>
      )}

      {expanded && (
        <div className="pt-2 border-t border-slate-100 space-y-1.5">
          <div className="text-[10px] font-semibold text-slate-600 uppercase tracking-wider">
            Evidence Factors & Scoring Breakdown
          </div>
          <div className="space-y-1">
            {file.evidence_factors.map((factor, idx) => (
              <div
                key={idx}
                className={`text-[11px] p-1.5 rounded flex items-center justify-between ${
                  factor.passed ? 'bg-emerald-50/50 text-emerald-800' : 'bg-rose-50/50 text-rose-800'
                }`}
              >
                <div className="flex items-center gap-1.5">
                  {factor.passed ? (
                    <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600 shrink-0" />
                  ) : (
                    <XCircle className="w-3.5 h-3.5 text-rose-500 shrink-0" />
                  )}
                  <span>{factor.description}</span>
                </div>
                <span className="font-mono text-[10px] font-bold">
                  {factor.weight > 0 ? `+${factor.weight}` : factor.weight}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
