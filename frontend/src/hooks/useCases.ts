import { useState, useEffect, useCallback } from 'react';
import {
  Case,
  CaseEvidence,
  CaseForensicReport,
  CaseOperation,
  CaseReportSummary,
  CaseSummary,
  CaseTimelineItem,
  CreateCaseRequest,
  RecordCustodyRequest,
  UpdateCaseRequest,
  CaseAcquisitionItem,
  CaseRecoveryItem,
  CaseErasureItem,
} from '../types/case';
import * as caseService from '../services/case';
import { useAuthStore } from '../stores/authStore';

export function useCases() {
  const { sessionToken } = useAuthStore();
  const [cases, setCases] = useState<Case[]>([]);
  const [activeCase, setActiveCase] = useState<Case | null>(null);
  const [summary, setSummary] = useState<CaseSummary | null>(null);
  const [operations, setOperations] = useState<CaseOperation[]>([]);
  const [evidence, setEvidence] = useState<CaseEvidence[]>([]);
  const [timeline, setTimeline] = useState<CaseTimelineItem[]>([]);
  const [reports, setReports] = useState<CaseReportSummary[]>([]);
  const [acquisitions, setAcquisitions] = useState<CaseAcquisitionItem[]>([]);
  const [recoveries, setRecoveries] = useState<CaseRecoveryItem[]>([]);
  const [erasures, setErasures] = useState<CaseErasureItem[]>([]);
  const [activeReport, setActiveReport] = useState<CaseForensicReport | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const loadCases = useCallback(async (statusFilter?: string) => {
    try {
      setLoading(true);
      setError(null);
      const res = await caseService.listCases(statusFilter);
      setCases(res.cases);
    } catch (err: any) {
      setError(err?.message || 'Failed to list cases');
    } finally {
      setLoading(false);
    }
  }, []);

  const selectCase = useCallback(async (caseId: string) => {
    try {
      setLoading(true);
      setError(null);
      const c = await caseService.getCase(caseId);
      setActiveCase(c);
      if (c) {
        const [summ, ops, ev, tl, reps, acqs, recs, eras] = await Promise.all([
          caseService.getCaseSummary(caseId).catch(() => null),
          caseService.listCaseOperations(caseId).catch(() => []),
          caseService.listCaseEvidence(caseId).catch(() => []),
          caseService.getCaseTimeline(caseId).catch(() => []),
          caseService.listCaseReports(caseId).catch(() => []),
          caseService.listCaseAcquisitions(caseId).catch(() => []),
          caseService.listCaseRecoveries(caseId).catch(() => []),
          caseService.listCaseErasures(caseId).catch(() => []),
        ]);
        setSummary(summ);
        setOperations(ops);
        setEvidence(ev);
        setTimeline(tl);
        setReports(reps);
        setAcquisitions(acqs);
        setRecoveries(recs);
        setErasures(eras);
      }
    } catch (err: any) {
      setError(err?.message || 'Failed to load case details');
    } finally {
      setLoading(false);
    }
  }, []);

  const createNewCase = useCallback(
    async (request: CreateCaseRequest): Promise<Case> => {
      const token = sessionToken || '';
      const created = await caseService.createCase(token, request);
      await loadCases();
      await selectCase(created.case_id);
      return created;
    },
    [sessionToken, loadCases, selectCase]
  );

  const modifyCase = useCallback(
    async (caseId: string, request: UpdateCaseRequest): Promise<Case> => {
      const token = sessionToken || '';
      const updated = await caseService.updateCase(token, caseId, request);
      setActiveCase(updated);
      await loadCases();
      return updated;
    },
    [sessionToken, loadCases]
  );

  const changeCaseStatus = useCallback(
    async (caseId: string, status: string): Promise<Case> => {
      const token = sessionToken || '';
      const updated = await caseService.updateCaseStatus(token, caseId, status);
      setActiveCase(updated);
      await selectCase(caseId);
      await loadCases();
      return updated;
    },
    [sessionToken, selectCase, loadCases]
  );

  const linkOperation = useCallback(
    async (caseId: string, operationId: string, operationType: string, notes?: string) => {
      const token = sessionToken || '';
      const op = await caseService.associateOperationToCase(
        token,
        caseId,
        operationId,
        operationType,
        notes
      );
      await selectCase(caseId);
      return op;
    },
    [sessionToken, selectCase]
  );

  const linkEvidence = useCallback(
    async (
      caseId: string,
      request: {
        evidence_type: string;
        identifier: string;
        label: string;
        sha256?: string | null;
        size_bytes?: number | null;
        notes?: string | null;
      }
    ) => {
      const token = sessionToken || '';
      const ev = await caseService.addCaseEvidence(token, caseId, request);
      await selectCase(caseId);
      return ev;
    },
    [sessionToken, selectCase]
  );

  const addCustodyRecord = useCallback(
    async (caseId: string, request: RecordCustodyRequest) => {
      const token = sessionToken || '';
      const ce = await caseService.recordCaseCustodyEvent(token, caseId, request);
      await selectCase(caseId);
      return ce;
    },
    [sessionToken, selectCase]
  );

  const produceCaseReport = useCallback(
    async (caseId: string) => {
      const token = sessionToken || '';
      const report = await caseService.generateCaseReport(token, caseId);
      setActiveReport(report);
      await selectCase(caseId);
      return report;
    },
    [sessionToken, selectCase]
  );

  const viewReport = useCallback(async (reportId: string) => {
    const report = await caseService.getCaseReport(reportId);
    setActiveReport(report);
    return report;
  }, []);

  const verifyReport = useCallback(async (report: CaseForensicReport) => {
    return await caseService.verifyCaseReportIntegrity(report);
  }, []);

  const reopenActiveCase = useCallback(
    async (caseId: string): Promise<Case> => {
      const token = sessionToken || '';
      const reopened = await caseService.reopenCase(token, caseId);
      setActiveCase(reopened);
      await selectCase(caseId);
      await loadCases();
      return reopened;
    },
    [sessionToken, selectCase, loadCases]
  );

  const setAsActiveCase = useCallback(
    async (caseId: string): Promise<Case> => {
      const setCase = await caseService.setActiveCase(caseId);
      await selectCase(caseId);
      return setCase;
    },
    [selectCase]
  );

  const clearCurrentActiveCase = useCallback(
    async (): Promise<void> => {
      await caseService.clearActiveCase();
      setActiveCase(null);
      setSummary(null);
      setOperations([]);
      setEvidence([]);
      setTimeline([]);
      setReports([]);
      setAcquisitions([]);
      setRecoveries([]);
      setErasures([]);
    },
    []
  );

  useEffect(() => {
    loadCases();
  }, [loadCases]);

  useEffect(() => {
    async function initActiveCase() {
      try {
        const active = await caseService.getActiveCase();
        if (active) {
          await selectCase(active.case_id);
        }
      } catch (err) {
        console.error('Failed to restore active case:', err);
      }
    }
    initActiveCase();
  }, [selectCase]);

  return {
    cases,
    activeCase,
    summary,
    operations,
    evidence,
    timeline,
    reports,
    acquisitions,
    recoveries,
    erasures,
    activeReport,
    loading,
    error,
    loadCases,
    selectCase,
    createNewCase,
    modifyCase,
    changeCaseStatus,
    reopenActiveCase,
    setAsActiveCase,
    clearCurrentActiveCase,
    linkOperation,
    linkEvidence,
    addCustodyRecord,
    produceCaseReport,
    viewReport,
    verifyReport,
    setActiveReport,
  };
}
