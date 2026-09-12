import { useState, useEffect, useCallback, useRef } from 'react';
import { AcquisitionArtifact } from '../types/acquisition';
import {
  RecoveryOptions,
  RecoveryPlan,
  RecoveryProgress,
  RecoveryResult,
  RecoverySourceSnapshot,
} from '../types/recovery';
import {
  cancelRecovery,
  createRecoveryPlan,
  exportRecoveredFiles,
  getRecoveryProgress,
  listRecoverySources,
  startRecovery,
  validateRecoverySource,
} from '../services/recovery';

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

  const activeOpIdRef = useRef<string | null>(null);

  const refreshSources = useCallback(async () => {
    setLoadingSources(true);
    setError(null);
    try {
      const srcList = await listRecoverySources();
      setSources(srcList);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
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
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setValidatedSource(null);
    }
  }, []);

  const preparePlan = useCallback(
    async (options: RecoveryOptions) => {
      if (!selectedArtifact) {
        setError('Please select an evidence acquisition artifact first.');
        return null;
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
        return newPlan;
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        return null;
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

    const dummyOpId = `op-rec-${Date.now()}`;
    activeOpIdRef.current = dummyOpId;

    // Progress polling interval
    const interval = setInterval(async () => {
      if (activeOpIdRef.current) {
        try {
          const prog = await getRecoveryProgress(activeOpIdRef.current);
          if (prog) {
            setProgress(prog);
          }
        } catch {
          // ignore transient poll error
        }
      }
    }, 500);

    try {
      const result = await startRecovery({
        plan,
        session_token: sessionToken,
      });
      setActiveResult(result);
      activeOpIdRef.current = null;
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      clearInterval(interval);
      setIsScanning(false);
    }
  }, [plan, sessionToken]);

  const cancelCurrent = useCallback(async () => {
    if (activeOpIdRef.current) {
      try {
        await cancelRecovery(activeOpIdRef.current, sessionToken);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    }
  }, [sessionToken]);

  const exportFiles = useCallback(async (jobId: string, exportDir: string) => {
    try {
      const count = await exportRecoveredFiles({ job_id: jobId, export_dir: exportDir });
      return count;
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    }
  }, []);

  const resetState = useCallback(() => {
    setSelectedArtifact(null);
    setValidatedSource(null);
    setPlan(null);
    setActiveResult(null);
    setProgress(null);
    setError(null);
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
    refreshSources,
    selectArtifact,
    preparePlan,
    runRecovery,
    cancelCurrent,
    exportFiles,
    resetState,
  };
}
