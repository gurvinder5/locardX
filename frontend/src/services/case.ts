import { invoke } from '@tauri-apps/api/core';
import {
  Case,
  CaseEvidence,
  CaseForensicReport,
  CaseOperation,
  CaseReportSummary,
  CaseSummary,
  CaseTimelineItem,
  CreateCaseRequest,
  CustodyEvent,
  RecordCustodyRequest,
  UpdateCaseRequest,
  AuditEvent,
  AuditChainVerification,
} from '../types/case';

function isTauriEnvironment(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

// In-memory simulation state for browser development
let mockCases: Case[] = [
  {
    case_id: 'case-mock-001',
    case_reference: 'CASE-2026-001',
    title: 'Operation Ironclad - Forensic Triage',
    description: 'Investigation into seized solid-state media and network attached storage.',
    status: 'open',
    lead_investigator: 'lead_examiner',
    created_at: new Date(Date.now() - 3600000 * 24).toISOString(),
    updated_at: new Date().toISOString(),
    closed_at: null,
    metadata_json: '{}',
  },
];

let mockOperations: CaseOperation[] = [
  {
    id: 'c-op-1',
    case_id: 'case-mock-001',
    operation_id: 'op-acq-001',
    operation_type: 'ForensicAcquisition',
    associated_by: 'lead_examiner',
    associated_at: new Date(Date.now() - 3600000 * 20).toISOString(),
    notes: 'Physical bitstream acquisition of target disk',
  },
  {
    id: 'c-op-2',
    case_id: 'case-mock-001',
    operation_id: 'op-rec-001',
    operation_type: 'Recovery',
    associated_by: 'lead_examiner',
    associated_at: new Date(Date.now() - 3600000 * 15).toISOString(),
    notes: 'Signature carving and NTFS MFT reconstruction',
  },
];

let mockEvidence: CaseEvidence[] = [
  {
    evidence_id: 'ev-001',
    case_id: 'case-mock-001',
    evidence_type: 'physical_storage',
    identifier: '\\\\.\\PhysicalDrive1',
    label: 'Samsung SSD 980 1TB (Seized)',
    sha256: null,
    size_bytes: 1000204886016,
    introduced_by: 'lead_examiner',
    introduced_at: new Date(Date.now() - 3600000 * 23).toISOString(),
    status: 'Active',
    notes: 'Tamper seal #98721 intact',
  },
  {
    evidence_id: 'ev-002',
    case_id: 'case-mock-001',
    evidence_type: 'acquisition_image',
    identifier: '/evidence/samsung_980.dd',
    label: 'Bitstream DD Image - Samsung SSD 980',
    sha256: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
    size_bytes: 1000204886016,
    introduced_by: 'lead_examiner',
    introduced_at: new Date(Date.now() - 3600000 * 19).toISOString(),
    status: 'Active',
    notes: 'Raw DD bitstream image',
  },
];

let mockCustody: CustodyEvent[] = [
  {
    custody_id: 'cust-001',
    case_id: 'case-mock-001',
    evidence_id: 'ev-001',
    event_type: 'evidence_introduced',
    actor_id: 'lead_examiner',
    timestamp: new Date(Date.now() - 3600000 * 23).toISOString(),
    action: 'Evidence Introduced',
    details: 'Bagged and logged physical drive into evidence locker',
    audit_event_id: 'aud-001',
    audit_hash: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08',
  },
  {
    custody_id: 'cust-002',
    case_id: 'case-mock-001',
    evidence_id: 'ev-002',
    event_type: 'evidence_acquired',
    actor_id: 'lead_examiner',
    timestamp: new Date(Date.now() - 3600000 * 19).toISOString(),
    action: 'Bitstream DD Image Acquired',
    details: 'Streaming SHA-256 computed in single-pass read',
    audit_event_id: 'aud-002',
    audit_hash: '5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8',
  },
];

export async function createCase(sessionToken: string, request: CreateCaseRequest): Promise<Case> {
  if (isTauriEnvironment()) {
    return await invoke<Case>('create_case', {
      sessionToken,
      request,
    });
  }

  const newCase: Case = {
    case_id: `case-mock-${Date.now()}`,
    case_reference: request.case_reference,
    title: request.title,
    description: request.description,
    status: 'open',
    lead_investigator: 'current_user',
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    closed_at: null,
    metadata_json: request.metadata_json || '{}',
  };
  mockCases.unshift(newCase);
  return newCase;
}

export async function getCase(caseId: string): Promise<Case | null> {
  if (isTauriEnvironment()) {
    return await invoke<Case | null>('get_case', { caseId });
  }
  return mockCases.find((c) => c.case_id === caseId) || null;
}

export async function listCases(
  statusFilter?: string,
  limit = 50,
  offset = 0
): Promise<{ cases: Case[]; total: number }> {
  if (isTauriEnvironment()) {
    return await invoke<{ cases: Case[]; total: number }>('list_cases', {
      statusFilter,
      limit,
      offset,
    });
  }

  let filtered = mockCases;
  if (statusFilter) {
    filtered = mockCases.filter((c) => c.status === statusFilter);
  }
  return {
    cases: filtered.slice(offset, offset + limit),
    total: filtered.length,
  };
}

export async function updateCase(
  sessionToken: string,
  caseId: string,
  request: UpdateCaseRequest
): Promise<Case> {
  if (isTauriEnvironment()) {
    return await invoke<Case>('update_case', {
      sessionToken,
      caseId,
      request,
    });
  }

  const c = mockCases.find((item) => item.case_id === caseId);
  if (!c) throw new Error(`Case ${caseId} not found`);
  if (request.title) c.title = request.title;
  if (request.description) c.description = request.description;
  if (request.metadata_json) c.metadata_json = request.metadata_json;
  c.updated_at = new Date().toISOString();
  return c;
}

export async function updateCaseStatus(
  sessionToken: string,
  caseId: string,
  status: string
): Promise<Case> {
  if (isTauriEnvironment()) {
    return await invoke<Case>('update_case_status', {
      sessionToken,
      caseId,
      status,
    });
  }

  const c = mockCases.find((item) => item.case_id === caseId);
  if (!c) throw new Error(`Case ${caseId} not found`);
  c.status = status as Case['status'];
  c.updated_at = new Date().toISOString();
  if (status === 'completed' || status === 'archived') {
    c.closed_at = new Date().toISOString();
  } else {
    c.closed_at = null;
  }
  return c;
}

export async function associateOperationToCase(
  sessionToken: string,
  caseId: string,
  operationId: string,
  operationType: string,
  notes?: string
): Promise<CaseOperation> {
  if (isTauriEnvironment()) {
    return await invoke<CaseOperation>('associate_operation_to_case', {
      sessionToken,
      caseId,
      operationId,
      operationType,
      notes,
    });
  }

  const op: CaseOperation = {
    id: `c-op-${Date.now()}`,
    case_id: caseId,
    operation_id: operationId,
    operation_type: operationType,
    associated_by: 'current_user',
    associated_at: new Date().toISOString(),
    notes: notes || null,
  };
  mockOperations.push(op);
  return op;
}

export async function listCaseOperations(caseId: string): Promise<CaseOperation[]> {
  if (isTauriEnvironment()) {
    return await invoke<CaseOperation[]>('list_case_operations', { caseId });
  }
  return mockOperations.filter((op) => op.case_id === caseId);
}

export async function addCaseEvidence(
  sessionToken: string,
  caseId: string,
  request: {
    evidence_type: string;
    identifier: string;
    label: string;
    sha256?: string | null;
    size_bytes?: number | null;
    notes?: string | null;
  }
): Promise<CaseEvidence> {
  if (isTauriEnvironment()) {
    return await invoke<CaseEvidence>('add_case_evidence', {
      sessionToken,
      caseId,
      request,
    });
  }

  const ev: CaseEvidence = {
    evidence_id: `ev-${Date.now()}`,
    case_id: caseId,
    evidence_type: request.evidence_type as CaseEvidence['evidence_type'],
    identifier: request.identifier,
    label: request.label,
    sha256: request.sha256 || null,
    size_bytes: request.size_bytes || null,
    introduced_by: 'current_user',
    introduced_at: new Date().toISOString(),
    status: 'Active',
    notes: request.notes || null,
  };
  mockEvidence.push(ev);
  return ev;
}

export async function listCaseEvidence(caseId: string): Promise<CaseEvidence[]> {
  if (isTauriEnvironment()) {
    return await invoke<CaseEvidence[]>('list_case_evidence', { caseId });
  }
  return mockEvidence.filter((ev) => ev.case_id === caseId);
}

export async function recordCaseCustodyEvent(
  sessionToken: string,
  caseId: string,
  request: RecordCustodyRequest
): Promise<CustodyEvent> {
  if (isTauriEnvironment()) {
    return await invoke<CustodyEvent>('record_case_custody_event', {
      sessionToken,
      caseId,
      request,
    });
  }

  const ce: CustodyEvent = {
    custody_id: `cust-${Date.now()}`,
    case_id: caseId,
    evidence_id: request.evidence_id || null,
    event_type: request.event_type,
    actor_id: 'current_user',
    timestamp: new Date().toISOString(),
    action: request.action,
    details: request.details,
    audit_event_id: `aud-${Date.now()}`,
    audit_hash: 'mock-sha256-digest',
  };
  mockCustody.push(ce);
  return ce;
}

export async function getCaseTimeline(caseId: string): Promise<CaseTimelineItem[]> {
  if (isTauriEnvironment()) {
    return await invoke<CaseTimelineItem[]>('get_case_timeline', { caseId });
  }

  return mockCustody
    .filter((c) => c.case_id === caseId)
    .map((c) => ({
      id: c.custody_id,
      timestamp: c.timestamp,
      item_type: 'Custody',
      actor_id: c.actor_id,
      title: c.action,
      details: c.details,
      audit_hash: c.audit_hash,
      is_verified: true,
    }));
}

export async function getCaseSummary(caseId: string): Promise<CaseSummary> {
  if (isTauriEnvironment()) {
    return await invoke<CaseSummary>('get_case_summary', { caseId });
  }

  const c = mockCases.find((item) => item.case_id === caseId) || mockCases[0];
  const ops = mockOperations.filter((op) => op.case_id === caseId);
  const ev = mockEvidence.filter((e) => e.case_id === caseId);
  const cust = mockCustody.filter((cu) => cu.case_id === caseId);

  return {
    case: c,
    operation_count: ops.length,
    evidence_count: ev.length,
    acquisition_count: 1,
    recovery_job_count: 1,
    recovered_file_count: 142,
    erasure_count: 0,
    report_count: 1,
    custody_event_count: cust.length,
    audit_chain_status: 'Verified',
    last_activity_at: c.updated_at,
  };
}

export async function generateCaseReport(
  sessionToken: string,
  caseId: string
): Promise<CaseForensicReport> {
  if (isTauriEnvironment()) {
    return await invoke<CaseForensicReport>('generate_case_report', {
      sessionToken,
      caseId,
    });
  }

  const c = mockCases.find((item) => item.case_id === caseId) || mockCases[0];
  return {
    report_id: `crep-mock-${Date.now()}`,
    case_id: caseId,
    title: `Forensic Investigation Report - ${c.case_reference}`,
    case_info: {
      case_id: c.case_id,
      case_reference: c.case_reference,
      title: c.title,
      description: c.description,
      status: c.status,
      lead_investigator: c.lead_investigator,
      created_at: c.created_at,
      closed_at: c.closed_at,
    },
    acquisitions: [],
    recoveries: [],
    erasures: [],
    custody_timeline: [],
    audit_integrity: {
      is_valid: true,
      total_events: 12,
      last_verified_sequence: 12,
      audit_root_hash: 'mock-audit-root',
    },
    integrity: {
      audit_chain_reference: 'mock-audit-root',
      report_digest: 'mock-digest-387123',
      generated_at: new Date().toISOString(),
    },
  };
}

export async function getCaseReport(reportId: string): Promise<CaseForensicReport | null> {
  if (isTauriEnvironment()) {
    return await invoke<CaseForensicReport | null>('get_case_report', { reportId });
  }
  return null;
}

export async function listCaseReports(caseId: string): Promise<CaseReportSummary[]> {
  if (isTauriEnvironment()) {
    return await invoke<CaseReportSummary[]>('list_case_reports', { caseId });
  }
  return [];
}

export async function verifyCaseReportIntegrity(report: CaseForensicReport): Promise<boolean> {
  if (isTauriEnvironment()) {
    return await invoke<boolean>('verify_case_report_integrity', { report });
  }
  return true;
}

export async function listAuditEvents(limit = 100): Promise<AuditEvent[]> {
  if (isTauriEnvironment()) {
    return await invoke<AuditEvent[]>('list_audit_events', { limit });
  }
  return [
    {
      sequence_number: 1,
      event_id: 'evt-001',
      event_type: 'SYSTEM_STARTUP',
      timestamp: new Date().toISOString(),
      actor_id: null,
      target_ref: null,
      details: 'LocardX Workstation core booted',
      prev_hash: '0000000000000000000000000000000000000000000000000000000000000000',
      current_hash: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08',
    },
  ];
}

export async function verifyAuditChain(): Promise<AuditChainVerification> {
  if (isTauriEnvironment()) {
    return await invoke<AuditChainVerification>('verify_audit_chain');
  }
  return {
    is_valid: true,
    total_events: 42,
    last_verified_sequence: 42,
    broken_sequence: null,
    details: 'Audit log cryptographic hash chain verified. Zero tampering detected.',
  };
}
