import { useState, useEffect, useCallback, useRef } from 'react';
import { OperationDto } from '../types/operation';
import * as opService from '../services/operations';
import { useAuthStore } from '../stores/authStore';

export function useOperation() {
  const sessionToken = useAuthStore((s) => s.sessionToken);

  const [operations, setOperations] = useState<OperationDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [stateFilter, setStateFilter] = useState<string>('ALL');
  const [typeFilter, setTypeFilter] = useState<string>('ALL');

  const [selectedOperation, setSelectedOperation] = useState<OperationDto | null>(null);
  const pollTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchOperations = useCallback(async () => {
    try {
      setError(null);
      const data = await opService.listOperations(
        100,
        stateFilter === 'ALL' ? undefined : stateFilter,
        typeFilter === 'ALL' ? undefined : typeFilter
      );
      setOperations(data);

      // If an operation was selected, refresh its details
      setSelectedOperation((prev) => {
        if (!prev) return null;
        const updated = data.find((o) => o.operation_id === prev.operation_id);
        return updated || prev;
      });
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Failed to fetch operations';
      setError(msg);
    } finally {
      setLoading(false);
    }
  }, [stateFilter, typeFilter]);

  // Initial fetch and dependency update
  useEffect(() => {
    fetchOperations();
  }, [fetchOperations]);

  // Adaptive polling: poll every 1.5s if any operation is 'Running' or 'Queued'
  useEffect(() => {
    const hasActive = operations.some(
      (o) => o.current_state === 'Running' || o.current_state === 'Queued' || o.current_state === 'Cancelling'
    );

    if (hasActive) {
      pollTimerRef.current = setInterval(() => {
        fetchOperations();
      }, 1500);
    } else if (pollTimerRef.current) {
      clearInterval(pollTimerRef.current);
      pollTimerRef.current = null;
    }

    return () => {
      if (pollTimerRef.current) {
        clearInterval(pollTimerRef.current);
      }
    };
  }, [operations, fetchOperations]);

  const cancel = async (operationId: string) => {
    try {
      await opService.cancelOperation(operationId, sessionToken || undefined);
      await fetchOperations();
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Failed to cancel operation';
      setError(msg);
    }
  };

  const submitHash = async (path: string) => {
    const op = await opService.submitIntegrityHashOperation(path, sessionToken || undefined);
    await fetchOperations();
    setSelectedOperation(op);
    return op;
  };

  const submitVerify = async (path: string, expectedDigest: string) => {
    const op = await opService.submitIntegrityVerifyOperation(
      path,
      expectedDigest,
      sessionToken || undefined
    );
    await fetchOperations();
    setSelectedOperation(op);
    return op;
  };

  return {
    operations,
    loading,
    error,
    stateFilter,
    setStateFilter,
    typeFilter,
    setTypeFilter,
    selectedOperation,
    setSelectedOperation,
    refresh: fetchOperations,
    cancel,
    submitHash,
    submitVerify,
  };
}
