import { useState, useEffect, useCallback, useRef } from 'react';
import { AcquisitionArtifact } from '../types/acquisition';
import {
  RecoveryOptions,
  RecoveryPlan,
  RecoveryProgress,
  RecoveryResult,
  RecoverySourceSnapshot,
  RecoveryWorkflowState,
} from '../types/recovery';
import {
  cancelRecovery,
  createRecoveryPlan,
  exportRecoveredFiles,
  getRecoveryJob,
  getRecoveryProgress,
  listRecoverySources,
  startRecovery,
  validateRecoverySource,
} from '../services/recovery';

export function extractErrorMessage(err: unknown): string {
  if (!err) return 'An unknown error occurred';
  if (err instanceof Error) return err.message;
  if (typeof err === 'object') {
    if ('message' in err && typeof (err as any).message === 'string' && (err as any).message.trim().length > 0) {
      return (err as any).message;
    }
    if ('code' in err && typeof (err as any).code === 'string') {
      return (err as any).code;
    }
  }
  return String(err);
}

export function formatRecoveryFailureReason(reason: unknown): string {
  if (!reason) return 'Unknown recovery failure';
  if (typeof reason === 'string') return reason;
  if (typeof reason === 'object') {
    if ('type' in reason) {
      const details = (reason as any).details;
      switch ((reason as any).type) {
        case 'filesystem_analysis_failed':
          return `Filesystem analysis failed: ${details || 'Unable to parse filesystem metadata'}`;
        case 'unsupported_filesystem':
          return `Unsupported filesystem: ${details || 'No recognized filesystem detected'}`;
        case 'corrupt_filesystem':
          return `Corrupted filesystem structure: ${details || 'Metadata corruption detected'}`;
        case 'image_not_found':
          return `Evidence image not found: ${details}`;
        case 'corrupt_image':
          return `Evidence image corruption detected: ${details}`;
        case 'read_error':
          return `Read I/O failure on evidence image: ${details}`;
        case 'cancelled':
          return 'Recovery operation was cancelled by operator';
        default:
          return details || (reason as any).type;
      }
    }
  }
  return String(reason);
}

export function useRecovery(sessionToken?: string | null) {
  const [sources, setSources] = useState<RecoverySourceSnapshot[]>([]);
  const [loadingSources, setLoadingSources] = useState<boolean>(false);
  const [selectedArtifact, setSelectedArtifact] = useState<AcquisitionArtifact | null>(null);
  const [validatedSource, setValidatedSource] = useState<RecoverySourceSnapshot | null>(null);
  const [plan, setPlan] = useState<RecoveryPlan | null>(null);
  const [isPlanning, setIsPlanning] = useState<boolean>(false);
  const [activeResult, setActiveResult] = useState<RecoveryResult | null>(null);
  const [progress, setProgress] = useState<RecoveryProgress | null>(null);
  const [isScanning, setIsScanning] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [workflowState, setWorkflowState] = useState<RecoveryWorkflowState>('IDLE');

  const activeOpIdRef = useRef<string | null>(null);

  const refreshSources = useCallback(async () => {
    setLoadingSources(true);
    setError(null);
    try {
      const srcList = await listRecoverySources();
      setSources(srcList);
    } catch (err) {
      setError(extractErrorMessage(err));
    } finally {
      setLoadingSources(false);
    }
  }, []);

  useEffect(() => {
    refreshSources();
  }, [refreshSources]);

  const selectArtifact = useCallback(async (artifact: AcquisitionArtifact) => {
    setSelectedArtifact(artifact);
    setError(null);
    try {
      const validated = await validateRecoverySource(artifact);
      setValidatedSource(validated);
      setWorkflowState('CONFIGURED');
    } catch (err) {
      setError(extractErrorMessage(err));
      setValidatedSource(null);
      setWorkflowState('IDLE');
    }
  }, []);

  const preparePlan = useCallback(
    async (options: RecoveryOptions) => {
      if (!selectedArtifact) {
        const msg = 'Please select an evidence acquisition artifact first.';
        setError(msg);
        throw new Error(msg);
      }
      setIsPlanning(true);
      setError(null);
      try {
        const newPlan = await createRecoveryPlan({
          artifact: selectedArtifact,
          options,
          session_token: sessionToken,
        });
        setPlan(newPlan);
        setWorkflowState('CONFIGURED');
        return newPlan;
      } catch (err) {
        const msg = extractErrorMessage(err);
        setError(msg);
        throw new Error(msg);
      } finally {
        setIsPlanning(false);
      }
    },
    [selectedArtifact, sessionToken]
  );

  const runRecovery = useCallback(async () => {
    if (!plan) {
      setError('No approved recovery plan ready for execution.');
      return;
    }

    setIsScanning(true);
    setProgress(null);
    setError(null);
    setActiveResult(null);
    setWorkflowState('STARTING');

    const opId = `op-rec-${Date.now()}`;
    activeOpIdRef.current = opId;

    // Fast polling interval (150ms) to ensure smooth telemetry capture
    const interval = setInterval(async () => {
      if (activeOpIdRef.current) {
        try {
          const prog = await getRecoveryProgress(activeOpIdRef.current);
          if (prog) {
            setProgress(prog);
            if (prog.phase) {
              const upper = prog.phase.toUpperCase();
              if (upper.includes('ANALYZ') || upper.includes('MFT') || upper.includes('DIRECTORY')) {
                setWorkflowState('ANALYZING');
              } else if (upper.includes('RECOVER')) {
                setWorkflowState('RECOVERING');
              } else if (upper.includes('VALIDAT')) {
                setWorkflowState('VALIDATING');
              }
            }
          }
        } catch {
          // ignore transient poll error
        }
      }
    }, 150);

    try {
      const res = await startRecovery({
        plan,
        session_token: sessionToken,
        operation_id: opId,
      });

      // Verification of backend response state machine
      if (res.status === 'Completed') {
        // Fetch scoped final job result if available, or use verified returned result
        const scopedJob = res.job_id ? await getRecoveryJob(res.job_id).catch(() => null) : null;
        const finalResult = scopedJob || res;
        setActiveResult(finalResult);
        setWorkflowState('COMPLETED');
      } else if (res.status === 'Cancelled') {
        setWorkflowState('CANCELLED');
        setError('Recovery operation was cancelled by operator.');
      } else {
        setWorkflowState('FAILED');
        const formattedErr = formatRecoveryFailureReason(res.failure_reason);
        setError(formattedErr);
      }

      activeOpIdRef.current = null;
    } catch (err) {
      setWorkflowState('FAILED');
      setError(extractErrorMessage(err));
    } finally {
      clearInterval(interval);
      setIsScanning(false);
    }
  }, [plan, sessionToken]);

  const cancelCurrent = useCallback(async () => {
    if (activeOpIdRef.current) {
      try {
        await cancelRecovery(activeOpIdRef.current, sessionToken);
        setWorkflowState('CANCELLED');
      } catch (err) {
        setError(extractErrorMessage(err));
      }
    }
  }, [sessionToken]);

  const exportFiles = useCallback(async (jobId: string, exportDir: string) => {
    try {
      const count = await exportRecoveredFiles({ job_id: jobId, export_dir: exportDir });
      return count;
    } catch (err) {
      const msg = extractErrorMessage(err);
      setError(msg);
      throw new Error(msg);
    }
  }, []);

  const resetState = useCallback(() => {
    setSelectedArtifact(null);
    setValidatedSource(null);
    setPlan(null);
    setActiveResult(null);
    setProgress(null);
    setError(null);
    setWorkflowState('IDLE');
  }, []);

  return {
    sources,
    loadingSources,
    selectedArtifact,
    validatedSource,
    plan,
    isPlanning,
    activeResult,
    progress,
    isScanning,
    error,
    workflowState,
    setWorkflowState,
    refreshSources,
    selectArtifact,
    preparePlan,
    runRecovery,
    cancelCurrent,
    exportFiles,
    resetState,
  };
}
