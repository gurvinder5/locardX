import { useEffect, useCallback } from 'react';
import { AcquisitionArtifact } from '../types/acquisition';
import { RecoveryOptions } from '../types/recovery';
import {
  useRecoveryStore,
  extractErrorMessage,
  formatRecoveryFailureReason,
} from '../stores/recoveryStore';

export { extractErrorMessage, formatRecoveryFailureReason };

export function useRecovery(sessionToken?: string | null) {
  const store = useRecoveryStore();

  useEffect(() => {
    store.fetchSources();
  }, []);

  const refreshSources = useCallback(async () => {
    await store.fetchSources();
  }, [store.fetchSources]);

  const selectArtifact = useCallback(
    async (artifact: AcquisitionArtifact) => {
      await store.selectArtifact(artifact);
    },
    [store.selectArtifact]
  );

  const preparePlan = useCallback(
    async (options: RecoveryOptions) => {
      return await store.preparePlan(options, sessionToken);
    },
    [store.preparePlan, sessionToken]
  );

  const runRecovery = useCallback(async () => {
    await store.runRecovery(sessionToken);
  }, [store.runRecovery, sessionToken]);

  const cancelCurrent = useCallback(async () => {
    await store.cancelCurrent(sessionToken);
  }, [store.cancelCurrent, sessionToken]);

  const exportFiles = useCallback(
    async (jobId: string, exportDir: string) => {
      return await store.exportFiles(jobId, exportDir);
    },
    [store.exportFiles]
  );

  const resetState = useCallback(() => {
    store.resetWizard();
  }, [store.resetWizard]);

  return {
    sources: store.sources,
    loadingSources: store.loadingSources,
    selectedArtifact: store.selectedArtifact,
    validatedSource: store.validatedSource,
    plan: store.plan,
    isPlanning: store.isPlanning,
    activeResult: store.activeResult,
    progress: store.progress,
    isScanning: store.isScanning,
    error: store.error,
    workflowState: store.workflowState,
    setWorkflowState: (state: any) => useRecoveryStore.setState({ workflowState: state }),
    refreshSources,
    selectArtifact,
    preparePlan,
    runRecovery,
    cancelCurrent,
    exportFiles,
    resetState,
  };
}
