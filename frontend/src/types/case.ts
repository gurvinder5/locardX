export type CaseStatus = 'open' | 'in_progress' | 'completed' | 'archived';

export type EvidenceType = 'physical_storage' | 'acquisition_image' | 'recovered_dataset' | 'logical_file';

export type CustodyEventType =
  | 'evidence_introduced'
  | 'evidence_acquired'
  | 'evidence_verified'
  | 'evidence_analyzed'
  | 'evidence_exported'
  | 'report_generated'
  | 'case_status_changed';

export interface Case {
  case_id: string;
  case_reference: string;
  title: string;
  description: string;
  status: CaseStatus;
  lead_investigator: string;
  created_at: string;
  updated_at: string;
  closed_at: string | null;
  metadata_json: string;
}

export interface CreateCaseRequest {
  case_reference: string;
  title: string;
  description: string;
  metadata_json?: string;
}

export interface UpdateCaseRequest {
  title?: string;
  description?: string;
  metadata_json?: string;
}

export interface CaseOperation {
  id: string;
  case_id: string;
  operation_id: string;
  operation_type: string;
  associated_by: string;
  associated_at: string;
  notes: string | null;
}

export interface CaseEvidence {
  evidence_id: string;
  case_id: string;
  evidence_type: EvidenceType;
  identifier: string;
  label: string;
  sha256: string | null;
  size_bytes: number | null;
  introduced_by: string;
  introduced_at: string;
  status: string;
  notes: string | null;
}

export interface AddEvidenceRequest {
  evidence_type: EvidenceType;
  identifier: string;
  label: string;
  sha256?: string | null;
  size_bytes?: number | null;
  notes?: string | null;
}

export interface CustodyEvent {
  custody_id: string;
  case_id: string;
  evidence_id: string | null;
  event_type: CustodyEventType;
  actor_id: string;
  timestamp: string;
  action: string;
  details: string;
  audit_event_id: string | null;
  audit_hash: string | null;
}

export interface RecordCustodyRequest {
  evidence_id?: string | null;
  event_type: CustodyEventType;
  action: string;
  details: string;
}

export interface CaseTimelineItem {
  id: string;
  timestamp: string;
  item_type: 'Custody' | 'Audit';
  actor_id: string;
  title: string;
  details: string;
  audit_hash: string | null;
  is_verified: boolean;
}

export interface CaseSummary {
  case: Case;
  operation_count: number;
  evidence_count: number;
  acquisition_count: number;
  recovery_job_count: number;
  recovered_file_count: number;
  erasure_count: number;
  report_count: number;
  custody_event_count: number;
  audit_chain_status: string;
  last_activity_at: string;
}

export interface RecoveredFileSnippet {
  file_id: string;
  filename: string;
  file_type: string;
  size_bytes: number;
  confidence_score: number;
  confidence_grade: string;
  sha256_hash: string;
  recovery_method: string;
}

export interface CaseAcquisitionReportInfo {
  acquisition_id: string;
  operation_id: string;
  source_device_id: string;
  source_display_name: string;
  source_serial: string | null;
  source_capacity_bytes: number;
  destination_path: string;
  image_format: string;
  image_size_bytes: number;
  image_sha256: string;
  status: string;
  started_at: string;
  completed_at: string;
  audit_reference: string;
}

export interface CaseRecoveryReportInfo {
  job_id: string;
  operation_id: string;
  acquisition_id: string;
  source_image_sha256: string;
  recovery_mode: string;
  status: string;
  files_recovered: number;
  candidates_evaluated: number;
  elapsed_seconds: number;
  category_counts: Record<string, number>;
  confidence_distribution: Record<string, number>;
  sample_files: RecoveredFileSnippet[];
  audit_reference: string;
}

export interface CaseErasureReportInfo {
  operation_id: string;
  plan_id: string;
  physical_device_id: string;
  display_name: string;
  serial_number: string | null;
  method: string;
  execution_mode: string;
  status: string;
  verification_outcome: string;
  verification_strategy: string;
  evidence_digest: string | null;
  limitations: string[];
  audit_reference: string;
  completed_at: string;
}

export interface ReportAuditIntegrity {
  is_valid: boolean;
  total_events: number;
  last_verified_sequence: number;
  audit_root_hash: string;
}

export interface ReportIntegrity {
  audit_chain_reference: string;
  report_digest: string;
  generated_at: string;
}

export interface CaseForensicReport {
  report_id: string;
  case_id: string;
  title: string;
  case_info: {
    case_id: string;
    case_reference: string;
    title: string;
    description: string;
    status: string;
    lead_investigator: string;
    created_at: string;
    closed_at: string | null;
  };
  acquisitions: CaseAcquisitionReportInfo[];
  recoveries: CaseRecoveryReportInfo[];
  erasures: CaseErasureReportInfo[];
  custody_timeline: {
    custody_id: string;
    evidence_id: string | null;
    timestamp: string;
    event_type: string;
    actor_id: string;
    action: string;
    details: string;
    audit_hash: string | null;
  }[];
  audit_integrity: ReportAuditIntegrity;
  integrity: ReportIntegrity;
}

export interface CaseReportSummary {
  report_id: string;
  case_id: string;
  report_type: string;
  title: string;
  report_digest: string;
  generated_by: string;
  generated_at: string;
}

export interface AuditEvent {
  sequence_number: number;
  event_id: string;
  event_type: string;
  timestamp: string;
  actor_id: string | null;
  target_ref: string | null;
  details: string;
  prev_hash: string;
  current_hash: string;
}

export interface AuditChainVerification {
  is_valid: boolean;
  total_events: number;
  last_verified_sequence: number;
  broken_sequence: number | null;
  details: string;
}
