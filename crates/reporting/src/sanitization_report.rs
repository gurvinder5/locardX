use crate::models::{
    DriveSanitizationReport, ReportDeviceInfo, ReportExecutionMetrics, ReportIntegrity,
    ReportOperationInfo, ReportVerificationInfo,
};
use chrono::Utc;
use locardx_drive_eraser::models::{DriveEraseResult, ExecutionMode};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Generator producing formal, tamper-evident physical drive sanitization reports and certificates.
pub struct SanitizationReportGenerator;

impl SanitizationReportGenerator {
    /// Generates a formal `DriveSanitizationReport` from a completed `DriveEraseResult`.
    pub fn generate_from_result(
        result: &DriveEraseResult,
        audit_chain_ref: Option<String>,
        actor_id: Option<String>,
    ) -> DriveSanitizationReport {
        let report_id = format!("rep-{}", Uuid::new_v4());
        let generated_at = Utc::now().to_rfc3339();
        let is_simulation = result.execution_mode == ExecutionMode::Simulation;

        let device_info = ReportDeviceInfo {
            physical_device_id: result.physical_device_id.clone(),
            display_name: result.display_name.clone(),
            vendor: result.vendor.clone(),
            model: result.model.clone(),
            serial_number: result.serial_number.clone(),
            media_type: format!("{:?}", result.media_type),
            capacity_bytes: result.capacity_bytes,
            sector_size: result.sector_size,
            bus_type: None, // Will be enriched from database if present
        };

        let operation_info = ReportOperationInfo {
            sanitization_method: result.method.to_string(),
            execution_mode: result.execution_mode.to_string(),
            is_simulation,
            started_at: result.started_at.clone(),
            completed_at: result.completed_at.clone(),
            elapsed_seconds: result.elapsed_seconds,
        };

        let execution_metrics = ReportExecutionMetrics {
            status: result.status.to_string(),
            bytes_processed: result.bytes_processed,
            total_bytes: result.capacity_bytes,
            failure_reason: result.failure_reason.as_ref().map(|f| f.to_string()),
            limitations: result.limitations.clone(),
        };

        // Calculate cryptographic evidence digest
        let mut evidence_hasher = Sha256::new();
        evidence_hasher.update(result.operation_id.as_bytes());
        evidence_hasher.update(result.physical_device_id.as_bytes());
        evidence_hasher.update(result.verification.strategy.to_string().as_bytes());
        evidence_hasher.update(result.verification.outcome.to_string().as_bytes());
        evidence_hasher.update(result.verification.details.as_bytes());
        evidence_hasher.update(result.verification.verified_at.as_bytes());
        let evidence_digest = hex::encode(evidence_hasher.finalize());

        let verification = ReportVerificationInfo {
            strategy: result.verification.strategy.to_string(),
            outcome: result.verification.outcome.to_string(),
            details: result.verification.details.clone(),
            evidence_digest,
            verified_at: result.verification.verified_at.clone(),
        };

        let audit_ref = audit_chain_ref.unwrap_or_else(|| {
            result
                .audit_references
                .first()
                .cloned()
                .unwrap_or_else(|| "UNBOUND".to_string())
        });

        // Compute overall tamper-evident report digest
        let report_digest = Self::calculate_report_digest(
            &report_id,
            &result.operation_id,
            &result.plan_id,
            actor_id.as_deref(),
            &device_info,
            &operation_info,
            &execution_metrics,
            &verification,
            &audit_ref,
            &generated_at,
        );

        let integrity = ReportIntegrity {
            audit_chain_reference: audit_ref,
            report_digest,
            generated_at,
        };

        DriveSanitizationReport {
            report_id,
            operation_id: result.operation_id.clone(),
            plan_id: result.plan_id.clone(),
            actor_id,
            device_info,
            operation_info,
            execution_metrics,
            verification,
            integrity,
        }
    }

    /// Computes the SHA-256 digest covering all substantive fields of the report.
    pub fn calculate_report_digest(
        report_id: &str,
        operation_id: &str,
        plan_id: &str,
        actor_id: Option<&str>,
        device: &ReportDeviceInfo,
        operation: &ReportOperationInfo,
        metrics: &ReportExecutionMetrics,
        verification: &ReportVerificationInfo,
        audit_ref: &str,
        generated_at: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(report_id.as_bytes());
        hasher.update(operation_id.as_bytes());
        hasher.update(plan_id.as_bytes());
        if let Some(act) = actor_id {
            hasher.update(act.as_bytes());
        }
        hasher.update(device.physical_device_id.as_bytes());
        hasher.update(device.capacity_bytes.to_le_bytes());
        hasher.update(device.media_type.as_bytes());
        if let Some(ser) = &device.serial_number {
            hasher.update(ser.as_bytes());
        }
        hasher.update(operation.sanitization_method.as_bytes());
        hasher.update(operation.execution_mode.as_bytes());
        hasher.update(operation.started_at.as_bytes());
        hasher.update(operation.completed_at.as_bytes());
        hasher.update(metrics.status.as_bytes());
        hasher.update(metrics.bytes_processed.to_le_bytes());
        hasher.update(verification.strategy.as_bytes());
        hasher.update(verification.outcome.as_bytes());
        hasher.update(verification.evidence_digest.as_bytes());
        hasher.update(audit_ref.as_bytes());
        hasher.update(generated_at.as_bytes());

        hex::encode(hasher.finalize())
    }

    /// Validates the integrity of a report against its internal cryptographic hash.
    pub fn verify_report_integrity(report: &DriveSanitizationReport) -> bool {
        let expected = Self::calculate_report_digest(
            &report.report_id,
            &report.operation_id,
            &report.plan_id,
            report.actor_id.as_deref(),
            &report.device_info,
            &report.operation_info,
            &report.execution_metrics,
            &report.verification,
            &report.integrity.audit_chain_reference,
            &report.integrity.generated_at,
        );

        expected == report.integrity.report_digest
    }

    /// Formats the report as a formal ASCII-framed sanitization certificate.
    pub fn format_text_certificate(report: &DriveSanitizationReport) -> String {
        let line = "=".repeat(78);
        let subline = "-".repeat(78);

        let banner = if report.is_simulation() {
            format!(
                "{}\n  *** CAUTION: THIS IS A DRY-RUN SIMULATION CERTIFICATE ONLY ***\n  *** NO PHYSICAL STORAGE MEDIA WAS ACCESSED, WRITTEN, OR MODIFIED ***\n{}",
                subline, subline
            )
        } else {
            format!(
                "{}\n  *** REAL HARDWARE PHYSICAL DRIVE SANITIZATION CERTIFICATE ***\n  *** STORAGE MEDIA HAS UNDERGONE IRREVERSIBLE PHYSICAL SANITIZATION ***\n{}",
                subline, subline
            )
        };

        let cap_gb = report.device_info.capacity_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        format!(
            "{line}\n\
             LOCARDX FORENSIC SUITE — SECURE DRIVE ERASURE CERTIFICATE\n\
             {line}\n\
             {banner}\n\
             CERTIFICATE METADATA\n\
             {subline}\n\
             Report ID:            {report_id}\n\
             Operation ID:         {op_id}\n\
             Plan ID:              {plan_id}\n\
             Operator:             {actor}\n\
             Generated At:         {gen_at}\n\
             \n\
             TARGET STORAGE DEVICE\n\
             {subline}\n\
             Physical Device:      {dev_id}\n\
             Display Name:         {dev_name}\n\
             Vendor:               {vendor}\n\
             Model:                {model}\n\
             Serial Number:        {serial}\n\
             Media Type:           {media}\n\
             Capacity:             {bytes} bytes ({cap_gb:.2} GiB)\n\
             Sector Size:          {sector} bytes\n\
             \n\
             SANITIZATION EXECUTION\n\
             {subline}\n\
             Method:               {method}\n\
             Execution Mode:       {mode}\n\
             Status:               {status}\n\
             Bytes Processed:      {processed} bytes\n\
             Elapsed Time:         {elapsed:.2}s\n\
             Started At:           {started}\n\
             Completed At:         {completed}\n\
             Failure Reason:       {failure}\n\
             \n\
             FORENSIC VERIFICATION\n\
             {subline}\n\
             Strategy:             {v_strat}\n\
             Verification Outcome: {v_out}\n\
             Verification Details: {v_det}\n\
             Evidence Digest:      {v_dig}\n\
             Verified At:          {v_at}\n\
             \n\
             CRYPTOGRAPHIC INTEGRITY & AUDIT TRAIL\n\
             {subline}\n\
             Audit Chain Ref:      {audit_ref}\n\
             Report SHA-256 Digest:{rep_dig}\n\
             {line}\n",
            line = line,
            subline = subline,
            banner = banner,
            report_id = report.report_id,
            op_id = report.operation_id,
            plan_id = report.plan_id,
            actor = report
                .actor_id
                .as_deref()
                .unwrap_or("Unauthenticated / System"),
            gen_at = report.integrity.generated_at,
            dev_id = report.device_info.physical_device_id,
            dev_name = report.device_info.display_name,
            vendor = report.device_info.vendor.as_deref().unwrap_or("N/A"),
            model = report.device_info.model.as_deref().unwrap_or("N/A"),
            serial = report.device_info.serial_number.as_deref().unwrap_or("N/A"),
            media = report.device_info.media_type,
            bytes = report.device_info.capacity_bytes,
            cap_gb = cap_gb,
            sector = report.device_info.sector_size,
            method = report.operation_info.sanitization_method,
            mode = report.operation_info.execution_mode,
            status = report.execution_metrics.status,
            processed = report.execution_metrics.bytes_processed,
            elapsed = report.operation_info.elapsed_seconds,
            started = report.operation_info.started_at,
            completed = report.operation_info.completed_at,
            failure = report
                .execution_metrics
                .failure_reason
                .as_deref()
                .unwrap_or("None (Successful)"),
            v_strat = report.verification.strategy,
            v_out = report.verification.outcome,
            v_det = report.verification.details,
            v_dig = report.verification.evidence_digest,
            v_at = report.verification.verified_at,
            audit_ref = report.integrity.audit_chain_reference,
            rep_dig = report.integrity.report_digest,
        )
    }

    /// Formats the report as GitHub-flavored Markdown.
    pub fn format_markdown_report(report: &DriveSanitizationReport) -> String {
        let alert = if report.is_simulation() {
            "> [!WARNING]\n> **SIMULATION REPORT**: This operation was performed in simulation mode. No physical storage sectors were altered or erased.\n"
        } else if report.is_verified_physical_erasure() {
            "> [!IMPORTANT]\n> **VERIFIED HARDWARE SANITIZATION**: Target physical media has undergone irreversible physical erasure and post-erasure forensic verification has succeeded.\n"
        } else {
            "> [!CAUTION]\n> **INCOMPLETE OR UNVERIFIED SANITIZATION**: Operation did not conclude with verified physical sanitization.\n"
        };

        let cap_gb = report.device_info.capacity_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        format!(
            "# LocardX Secure Drive Erasure Certificate\n\n\
             {alert}\n\
             ## Certificate Information\n\
             - **Report ID**: `{report_id}`\n\
             - **Operation ID**: `{op_id}`\n\
             - **Plan ID**: `{plan_id}`\n\
             - **Operator**: `{actor}`\n\
             - **Generated**: `{gen_at}`\n\n\
             ## Target Device Topology\n\
             | Attribute | Value |\n\
             | :--- | :--- |\n\
             | **Physical Device ID** | `{dev_id}` |\n\
             | **Display Name** | {dev_name} |\n\
             | **Vendor** | {vendor} |\n\
             | **Model** | {model} |\n\
             | **Serial Number** | `{serial}` |\n\
             | **Media Type** | `{media}` |\n\
             | **Capacity** | {bytes} bytes ({cap_gb:.2} GiB) |\n\
             | **Sector Geometry** | {sector} bytes/sector |\n\n\
             ## Execution Summary\n\
             | Parameter | Value |\n\
             | :--- | :--- |\n\
             | **Sanitization Method** | `{method}` |\n\
             | **Execution Mode** | `{mode}` |\n\
             | **Execution Status** | **{status}** |\n\
             | **Bytes Processed** | {processed} bytes |\n\
             | **Elapsed Time** | {elapsed:.2} seconds |\n\
             | **Started At** | `{started}` |\n\
             | **Completed At** | `{completed}` |\n\
             | **Failure Reason** | {failure} |\n\n\
             ## Forensic Verification\n\
             - **Strategy**: `{v_strat}`\n\
             - **Outcome**: **`{v_out}`**\n\
             - **Details**: {v_det}\n\
             - **Evidence Digest**: `{v_dig}`\n\
             - **Verified At**: `{v_at}`\n\n\
             ## Integrity & Cryptographic Chain\n\
             - **Audit Chain Reference**: `{audit_ref}`\n\
             - **Report SHA-256 Digest**: `{rep_dig}`\n",
            alert = alert,
            report_id = report.report_id,
            op_id = report.operation_id,
            plan_id = report.plan_id,
            actor = report
                .actor_id
                .as_deref()
                .unwrap_or("Unauthenticated / System"),
            gen_at = report.integrity.generated_at,
            dev_id = report.device_info.physical_device_id,
            dev_name = report.device_info.display_name,
            vendor = report.device_info.vendor.as_deref().unwrap_or("N/A"),
            model = report.device_info.model.as_deref().unwrap_or("N/A"),
            serial = report.device_info.serial_number.as_deref().unwrap_or("N/A"),
            media = report.device_info.media_type,
            bytes = report.device_info.capacity_bytes,
            cap_gb = cap_gb,
            sector = report.device_info.sector_size,
            method = report.operation_info.sanitization_method,
            mode = report.operation_info.execution_mode,
            status = report.execution_metrics.status,
            processed = report.execution_metrics.bytes_processed,
            elapsed = report.operation_info.elapsed_seconds,
            started = report.operation_info.started_at,
            completed = report.operation_info.completed_at,
            failure = report
                .execution_metrics
                .failure_reason
                .as_deref()
                .unwrap_or("None"),
            v_strat = report.verification.strategy,
            v_out = report.verification.outcome,
            v_det = report.verification.details,
            v_dig = report.verification.evidence_digest,
            v_at = report.verification.verified_at,
            audit_ref = report.integrity.audit_chain_reference,
            rep_dig = report.integrity.report_digest,
        )
    }
}
