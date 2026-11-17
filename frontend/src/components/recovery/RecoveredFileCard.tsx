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
import { Button, Chip } from '@heroui/react';
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
        return <ImageIcon className="w-4 h-4 text-slate-600 dark:text-slate-400" />;
      case 'documents':
        return <FileText className="w-4 h-4 text-slate-600 dark:text-slate-400" />;
      case 'archives':
        return <Archive className="w-4 h-4 text-slate-600 dark:text-slate-400" />;
      case 'media':
        return <Film className="w-4 h-4 text-slate-600 dark:text-slate-400" />;
      case 'databases':
        return <Database className="w-4 h-4 text-slate-600 dark:text-slate-400" />;
      case 'text_files':
        return <FileCode className="w-4 h-4 text-slate-600 dark:text-slate-400" />;
      default:
        return <File className="w-4 h-4 text-slate-600 dark:text-slate-400" />;
    }
  };

  const getConfidenceBadge = (grade: ConfidenceGrade, score: number) => {
    switch (grade) {
      case 'high':
        return (
          <Chip
            size="sm"
            variant="flat"
            startContent={<ShieldCheck className="w-3 h-3 text-slate-700 dark:text-slate-300" />}
            className="text-[10px] font-semibold bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700"
          >
            High ({score}%)
          </Chip>
        );
      case 'medium':
        return (
          <Chip
            size="sm"
            variant="flat"
            className="text-[10px] font-semibold bg-amber-50 dark:bg-amber-950/30 text-amber-700 dark:text-amber-400 border border-amber-200 dark:border-amber-900/40"
          >
            Medium ({score}%)
          </Chip>
        );
      case 'low':
        return (
          <Chip
            size="sm"
            variant="flat"
            className="text-[10px] font-semibold bg-amber-100 dark:bg-amber-950/50 text-amber-800 dark:text-amber-300 border border-amber-300 dark:border-amber-800/50"
          >
            Low ({score}%)
          </Chip>
        );
      case 'uncertain':
      default:
        return (
          <Chip
            size="sm"
            variant="flat"
            className="text-[10px] font-semibold bg-rose-50 dark:bg-rose-950/30 text-rose-700 dark:text-rose-400 border border-rose-200 dark:border-rose-900/40"
          >
            Uncertain ({score}%)
          </Chip>
        );
    }
  };

  const getValidationBadge = (status: ValidationStatus) => {
    switch (status) {
      case 'valid':
        return (
          <Chip
            size="sm"
            variant="flat"
            className="text-[10px] font-medium bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700"
          >
            Valid
          </Chip>
        );
      case 'partially_valid':
        return (
          <Chip
            size="sm"
            variant="flat"
            className="text-[10px] font-medium bg-amber-50 dark:bg-amber-950/30 text-amber-700 dark:text-amber-400 border border-amber-200 dark:border-amber-900/40"
          >
            Partially Valid
          </Chip>
        );
      case 'incomplete':
        return (
          <Chip
            size="sm"
            variant="flat"
            className="text-[10px] font-medium bg-amber-100 dark:bg-amber-950/50 text-amber-800 dark:text-amber-300 border border-amber-300 dark:border-amber-800/50"
          >
            Incomplete
          </Chip>
        );
      case 'corrupted':
      case 'invalid':
      default:
        return (
          <Chip
            size="sm"
            variant="flat"
            className="text-[10px] font-medium bg-rose-50 dark:bg-rose-950/30 text-rose-600 dark:text-rose-400 border border-rose-200 dark:border-rose-900/40"
          >
            Corrupted
          </Chip>
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
    <div className="border border-slate-200 dark:border-[#26313D] bg-white dark:bg-[#151C24] hover:border-slate-300 dark:hover:border-slate-700 transition-colors p-3 space-y-2 rounded-xs">
        <div className="flex items-start justify-between">
          <div className="flex items-start space-x-2.5">
            <div className="p-1.5 rounded bg-slate-50 dark:bg-workstation-surface border border-slate-100 dark:border-workstation-border mt-0.5">
              {getCategoryIcon(file.category)}
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="text-xs font-semibold text-slate-900 dark:text-slate-100 font-mono">
                  {file.suggested_filename}
                </span>
                {getValidationBadge(file.validation_status)}
              </div>
              <div className="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5 flex flex-wrap gap-x-3 gap-y-0.5">
                <span>Offset: <strong className="font-mono text-slate-700 dark:text-slate-300">0x{file.source_offset.toString(16).toUpperCase()}</strong></span>
                <span>Size: <strong className="text-slate-700 dark:text-slate-300">{formatBytes(file.size_bytes)}</strong></span>
                <span>Type: <strong className="text-slate-700 dark:text-slate-300">{file.file_type}</strong></span>
                <span>Method: <strong className="text-slate-700 dark:text-slate-300">{file.recovery_method.replace('_', ' ')}</strong></span>
              </div>
            </div>
          </div>
          <div className="flex items-center space-x-2">
            {getConfidenceBadge(file.confidence_grade, file.confidence_score)}
            <Button
              isIconOnly
              size="sm"
              variant="light"
              onPress={() => setExpanded(!expanded)}
              aria-label="Toggle evidence factors"
              className="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
            >
              {expanded ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
            </Button>
          </div>
        </div>

        <div className="flex items-center justify-between pt-1 border-t border-slate-100 dark:border-workstation-border">
          <div className="text-[10px] font-mono text-slate-400 dark:text-slate-500 truncate max-w-xs sm:max-w-md">
            SHA-256: {file.sha256_hash}
          </div>
          <div className="flex items-center gap-1.5 shrink-0">
            <Button
              size="sm"
              variant="flat"
              onPress={handleOpenFile}
              isDisabled={actionState.loading}
              startContent={
                actionState.loading && actionState.type === 'open' ? (
                  <Loader2 className="w-3 h-3 animate-spin text-slate-600" />
                ) : (
                  <ExternalLink className="w-3 h-3 text-slate-600 dark:text-slate-400" />
                )
              }
              className="text-[11px] font-medium h-7 px-2.5 bg-slate-100 dark:bg-workstation-surface text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-workstation-border"
            >
              Open
            </Button>
            <Button
              size="sm"
              variant="flat"
              onPress={handleRevealFile}
              isDisabled={actionState.loading}
              startContent={
                actionState.loading && actionState.type === 'reveal' ? (
                  <Loader2 className="w-3 h-3 animate-spin text-slate-600" />
                ) : (
                  <FolderOpen className="w-3 h-3 text-slate-600 dark:text-slate-400" />
                )
              }
              className="text-[11px] font-medium h-7 px-2.5 bg-slate-100 dark:bg-workstation-surface text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-workstation-border"
            >
              Reveal
            </Button>
          </div>
        </div>

        {actionState.error && (
          <div className="text-[11px] p-2 rounded bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 text-rose-800 dark:text-rose-300 flex items-center gap-1.5">
            <AlertTriangle className="w-3.5 h-3.5 text-rose-600 shrink-0" />
            <span>{actionState.error}</span>
          </div>
        )}

        {expanded && (
          <div className="pt-2 border-t border-slate-100 dark:border-workstation-border space-y-1.5">
            <div className="text-[10px] font-semibold text-slate-600 dark:text-slate-400 uppercase tracking-wider">
              Evidence Factors & Scoring Breakdown
            </div>
            <div className="space-y-1">
              {file.evidence_factors.map((factor, idx) => (
                <div
                  key={idx}
                  className={`text-[11px] p-1.5 rounded flex items-center justify-between ${
                    factor.passed
                      ? 'bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200'
                      : 'bg-rose-50/50 dark:bg-rose-950/20 text-rose-800 dark:text-rose-300'
                  }`}
                >
                  <div className="flex items-center gap-1.5">
                    {factor.passed ? (
                      <CheckCircle2 className="w-3.5 h-3.5 text-slate-700 dark:text-slate-300 shrink-0" />
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
