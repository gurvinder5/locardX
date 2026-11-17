import React, { useEffect, useState } from 'react';
import {
  ShieldCheck,
  Hash,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Copy,
  Check,
  RefreshCw,
  FileText,
  Clock,
  Shield,
  Search,
} from 'lucide-react';
import {
  Button,
  Input,
  Chip,
  Table,
  TableHeader,
  TableColumn,
  TableBody,
  TableRow,
  TableCell,
} from '@heroui/react';
import {
  HashResult,
  IntegrityRecord,
  VerificationResult,
  VerificationStatus,
} from '../types/integrity';
import {
  calculateFileHash,
  listIntegrityRecords,
  verifyFileHash,
} from '../services/integrity';
import { useAuthStore } from '../stores/authStore';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

export const IntegrityVerificationPage: React.FC = () => {
  const sessionToken = useAuthStore((s) => s.sessionToken);

  const [targetPath, setTargetPath] = useState('');
  const [expectedDigest, setExpectedDigest] = useState('');
  const [algorithm] = useState<'Sha256'>('Sha256');

  const [isCalculating, setIsCalculating] = useState(false);
  const [isVerifying, setIsVerifying] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const [hashResult, setHashResult] = useState<HashResult | null>(null);
  const [verificationResult, setVerificationResult] = useState<VerificationResult | null>(null);

  const [records, setRecords] = useState<IntegrityRecord[]>([]);
  const [loadingRecords, setLoadingRecords] = useState(false);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const loadHistory = async () => {
    try {
      setLoadingRecords(true);
      const data = await listIntegrityRecords(50);
      setRecords(data);
    } catch {
      // Non-critical background fetch failure
    } finally {
      setLoadingRecords(false);
    }
  };

  useEffect(() => {
    loadHistory();
  }, []);

  const handleCopy = (text: string, key: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const handleCalculate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!targetPath.trim()) {
      setErrorMessage('Please enter a target file path.');
      return;
    }

    try {
      setIsCalculating(true);
      setErrorMessage(null);
      setVerificationResult(null);

      const result = await calculateFileHash(targetPath.trim(), sessionToken || undefined);
      setHashResult(result);
      await loadHistory();
    } catch (err: unknown) {
      const msg =
        err && typeof err === 'object' && 'message' in err
          ? String((err as { message: unknown }).message)
          : err instanceof Error
          ? err.message
          : 'Failed to calculate file hash';
      setErrorMessage(msg);
      setHashResult(null);
    } finally {
      setIsCalculating(false);
    }
  };

  const handleVerify = async () => {
    if (!targetPath.trim()) {
      setErrorMessage('Please enter a target file path.');
      return;
    }

    const cleanedExpected = expectedDigest.trim().toLowerCase();
    if (!cleanedExpected) {
      setErrorMessage('Please enter an expected SHA-256 digest to verify against.');
      return;
    }

    if (cleanedExpected.length !== 64 || !/^[0-9a-f]{64}$/.test(cleanedExpected)) {
      setErrorMessage(
        'Expected digest must be exactly 64 hexadecimal characters (256 bits).'
      );
      return;
    }

    try {
      setIsVerifying(true);
      setErrorMessage(null);
      setHashResult(null);

      const result = await verifyFileHash(
        targetPath.trim(),
        cleanedExpected,
        sessionToken || undefined
      );
      setVerificationResult(result);
      await loadHistory();
    } catch (err: unknown) {
      const msg =
        err && typeof err === 'object' && 'message' in err
          ? String((err as { message: unknown }).message)
          : err instanceof Error
          ? err.message
          : 'Failed to verify file hash';
      setErrorMessage(msg);
      setVerificationResult(null);
    } finally {
      setIsVerifying(false);
    }
  };

  const renderStatusBadge = (status: VerificationStatus | string) => {
    if (status === 'Verified') {
      return (
        <Chip
          size="sm"
          variant="flat"
          startContent={<CheckCircle2 className="w-3.5 h-3.5 text-slate-700 dark:text-slate-300" />}
          className="text-xs font-semibold bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700"
        >
          VERIFIED
        </Chip>
      );
    }

    if (status === 'Mismatch' || (typeof status === 'object' && 'Mismatch' in status)) {
      return (
        <Chip
          size="sm"
          variant="flat"
          startContent={<XCircle className="w-3.5 h-3.5 text-rose-600 dark:text-rose-400" />}
          className="text-xs font-semibold bg-rose-50 dark:bg-rose-950/30 text-rose-700 dark:text-rose-400 border border-rose-200 dark:border-rose-900/40"
        >
          MISMATCH
        </Chip>
      );
    }

    if (typeof status === 'object' && 'UnableToVerify' in status) {
      return (
        <Chip
          size="sm"
          variant="flat"
          startContent={<AlertTriangle className="w-3.5 h-3.5 text-amber-700 dark:text-amber-400" />}
          className="text-xs font-semibold bg-amber-50 dark:bg-amber-950/30 text-amber-700 dark:text-amber-400 border border-amber-200 dark:border-amber-900/40"
        >
          UNABLE TO VERIFY
        </Chip>
      );
    }

    return (
      <Chip
        size="sm"
        variant="flat"
        startContent={<Hash className="w-3.5 h-3.5 text-slate-700 dark:text-slate-300" />}
        className="text-xs font-semibold bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-workstation-border"
      >
        CALCULATED
      </Chip>
    );
  };

  return (
    <div className="space-y-6 w-full min-w-0">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 border-b border-slate-200 dark:border-workstation-border pb-5">
        <div>
          <div className="flex items-center gap-2">
            <div className="p-2 rounded-lg bg-slate-100 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border text-slate-800 dark:text-slate-200">
              <ShieldCheck className="w-6 h-6" />
            </div>
            <div>
              <h1 className="text-xl font-bold text-slate-900 dark:text-slate-100 tracking-tight">
                Cryptographic Integrity & Evidence Verification
              </h1>
              <p className="text-sm text-slate-500 dark:text-slate-400">
                Strictly read-only SHA-256 evidence hashing and integrity verification.
              </p>
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Chip
            size="sm"
            variant="flat"
            startContent={<Shield className="w-3.5 h-3.5 text-slate-700 dark:text-slate-300" />}
            className="text-xs font-medium bg-slate-100 dark:bg-workstation-surface text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-workstation-border"
          >
            Read-Only Protection Active
          </Chip>
        </div>
      </div>

      {/* Safety Notice */}
      <div className="border-l-2 border-slate-500 bg-slate-50 dark:bg-workstation-surface p-3.5 text-xs text-slate-800 dark:text-slate-200 flex items-start gap-3">
        <ShieldCheck className="w-5 h-5 text-slate-700 dark:text-slate-300 shrink-0 mt-0.5" />
        <div>
          <p className="font-semibold text-slate-900 dark:text-slate-100">Non-Destructive Forensic Verification</p>
          <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
            Hashing executes strictly read-only streaming passes in 64 KiB chunks with bounded memory.
            Target files are never written, modified, or truncated. Every calculation is anchored into the
            tamper-evident SHA-256 audit hash-chain.
          </p>
        </div>
      </div>

      {/* Error Alert */}
      {errorMessage && (
        <div className="p-4 rounded-lg bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 text-sm text-rose-700 dark:text-rose-300 flex items-start gap-3">
          <AlertTriangle className="w-5 h-5 text-rose-500 shrink-0 mt-0.5" />
          <div className="flex-1">
            <p className="font-semibold">Verification Error</p>
            <p className="mt-0.5 text-xs text-rose-600 dark:text-rose-400">{errorMessage}</p>
          </div>
          <button
            onClick={() => setErrorMessage(null)}
            className="text-xs text-rose-500 hover:text-rose-700 font-medium"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Input Form */}
      <div className="bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border rounded-xs overflow-hidden">
        <div className="px-5 py-3 border-b border-slate-100 dark:border-workstation-border bg-slate-50/50 dark:bg-workstation-surface">
          <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-2 uppercase tracking-wider">
            <Search className="w-4 h-4 text-slate-500" />
            Target Specification & Parameters
          </h2>
        </div>
        <div className="p-5">
          <form onSubmit={handleCalculate} className="space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              <div className="md:col-span-3 space-y-1">
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider">
                  Target File Path
                </label>
                <Input
                  size="sm"
                  value={targetPath}
                  onValueChange={setTargetPath}
                  placeholder="e.g. C:\Evidence\disk_image.raw or /tmp/evidence.bin"
                  className="font-mono text-sm"
                  classNames={{
                    inputWrapper: 'bg-white dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border',
                  }}
                />
                <p className="text-[11px] text-slate-500 dark:text-slate-400">
                  Absolute or relative path to the evidence file on the local system.
                </p>
              </div>

              <div className="space-y-1">
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider">
                  Cryptographic Algorithm
                </label>
                <select
                  value={algorithm}
                  disabled
                  className="w-full px-3 py-2 text-sm rounded-lg border border-slate-300 dark:border-workstation-border bg-slate-50 dark:bg-workstation-surface text-slate-600 dark:text-slate-400 cursor-not-allowed font-medium"
                >
                  <option value="Sha256">SHA-256 (FIPS 180-4)</option>
                </select>
                <p className="text-[11px] text-slate-500 dark:text-slate-400">Standard 256-bit NIST cryptographic hash.</p>
              </div>
            </div>

            <div className="space-y-1">
              <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider">
                Expected SHA-256 Digest (Optional for calculation, required for verification)
              </label>
              <Input
                size="sm"
                value={expectedDigest}
                onValueChange={setExpectedDigest}
                placeholder="e.g. e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                className="font-mono text-xs"
                classNames={{
                  inputWrapper: 'bg-white dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border',
                }}
              />
              <p className="text-[11px] text-slate-500 dark:text-slate-400">
                Provide known hash to perform cryptographic comparison. Constant-time comparison prevents timing analysis.
              </p>
            </div>

            <div className="flex flex-wrap items-center gap-3 pt-2">
              <Button
                type="submit"
                isDisabled={isCalculating || isVerifying || !targetPath.trim()}
                isLoading={isCalculating}
                startContent={!isCalculating && <Hash className="w-4 h-4" />}
                className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 font-medium text-xs px-4 shadow-sm"
              >
                {isCalculating ? 'Computing SHA-256...' : 'Calculate Hash'}
              </Button>

              <Button
                type="button"
                onPress={handleVerify}
                isDisabled={
                  isCalculating ||
                  isVerifying ||
                  !targetPath.trim() ||
                  !expectedDigest.trim()
                }
                isLoading={isVerifying}
                startContent={!isVerifying && <CheckCircle2 className="w-4 h-4" />}
                className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 font-medium text-xs px-4 shadow-sm"
              >
                {isVerifying ? 'Verifying Integrity...' : 'Verify Against Expected'}
              </Button>
            </div>
          </form>
        </div>
      </div>

      {/* Verification Result Display */}
      {verificationResult && (
        <div className="bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border rounded-xs overflow-hidden">
          <div className="px-5 py-3 border-b border-slate-100 dark:border-workstation-border flex items-center justify-between bg-slate-50/50 dark:bg-workstation-surface">
            <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-2 uppercase tracking-wider">
              <ShieldCheck className="w-4 h-4 text-slate-600 dark:text-slate-400" />
              Verification Results
            </h2>
            <div>{renderStatusBadge(verificationResult.status)}</div>
          </div>

          <div className="p-5 space-y-4">
            {/* Status summary banner */}
            {verificationResult.status === 'Verified' && (
              <div className="p-4 rounded-lg bg-slate-100 dark:bg-workstation-surface border border-slate-300 dark:border-slate-700 flex items-start gap-3">
                <CheckCircle2 className="w-5 h-5 text-slate-700 dark:text-slate-300 shrink-0 mt-0.5" />
                <div>
                  <h3 className="text-sm font-bold text-slate-900 dark:text-slate-100">
                    Cryptographic Integrity Confirmed
                  </h3>
                  <p className="text-xs text-slate-700 dark:text-slate-300 mt-0.5">
                    The calculated SHA-256 digest strictly matches the expected digest. The target evidence
                    has not been modified or corrupted.
                  </p>
                </div>
              </div>
            )}

            {typeof verificationResult.status === 'object' &&
              'Mismatch' in verificationResult.status && (
                <div className="p-4 rounded-lg bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 flex items-start gap-3">
                  <XCircle className="w-5 h-5 text-rose-600 dark:text-rose-400 shrink-0 mt-0.5" />
                  <div>
                    <h3 className="text-sm font-bold text-rose-900 dark:text-rose-200">
                      Cryptographic Mismatch Detected
                    </h3>
                    <p className="text-xs text-rose-700 dark:text-rose-300 mt-0.5">
                      The calculated SHA-256 digest DOES NOT MATCH the expected digest.
                      Target contents differ from the recorded reference.
                    </p>
                  </div>
                </div>
              )}

            {typeof verificationResult.status === 'object' &&
              'UnableToVerify' in verificationResult.status && (
                <div className="p-4 rounded-lg bg-amber-50 dark:bg-amber-950/30 border border-amber-200 dark:border-amber-900/40 flex items-start gap-3">
                  <AlertTriangle className="w-5 h-5 text-amber-600 dark:text-amber-400 shrink-0 mt-0.5" />
                  <div>
                    <h3 className="text-sm font-bold text-amber-900 dark:text-amber-200">
                      Unable to Complete Verification
                    </h3>
                    <p className="text-xs text-amber-700 dark:text-amber-300 mt-0.5">
                      {verificationResult.status.UnableToVerify.reason}
                    </p>
                  </div>
                </div>
              )}

            {/* Target Identity Metadata */}
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 p-4 rounded-lg bg-slate-50 dark:bg-workstation-surface border border-slate-100 dark:border-workstation-border text-xs">
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Target Name</span>
                <span className="font-semibold text-slate-800 dark:text-slate-200 truncate block mt-0.5">
                  {verificationResult.target.display_name}
                </span>
              </div>
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Size Processed</span>
                <span className="font-semibold text-slate-800 dark:text-slate-200 block mt-0.5 font-mono">
                  {formatBytes(verificationResult.bytes_processed)}
                </span>
              </div>
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Elapsed Time</span>
                <span className="font-semibold text-slate-800 dark:text-slate-200 block mt-0.5 font-mono">
                  {verificationResult.duration_ms} ms
                </span>
              </div>
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Target Type</span>
                <span className="font-semibold text-slate-800 dark:text-slate-200 block mt-0.5">
                  {verificationResult.target.target_type}
                </span>
              </div>
            </div>

            {/* Hashes Comparison */}
            <div className="space-y-3">
              <div className="space-y-1">
                <div className="flex items-center justify-between text-xs">
                  <span className="font-semibold text-slate-600 dark:text-slate-400 uppercase tracking-wider text-[11px]">
                    Expected Digest
                  </span>
                  <Button
                    size="sm"
                    variant="light"
                    onPress={() => handleCopy(verificationResult.expected_digest, 'expected')}
                    startContent={copiedKey === 'expected' ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                    className="text-[11px] text-slate-600 dark:text-slate-300 font-medium h-6 px-2"
                  >
                    {copiedKey === 'expected' ? 'Copied' : 'Copy'}
                  </Button>
                </div>
                <div className="p-3 rounded-lg bg-slate-900 dark:bg-workstation-surface text-slate-100 font-mono text-xs break-all border border-slate-800 dark:border-workstation-border">
                  {verificationResult.expected_digest}
                </div>
              </div>

              {verificationResult.calculated_digest && (
                <div className="space-y-1">
                  <div className="flex items-center justify-between text-xs">
                    <span className="font-semibold text-slate-600 dark:text-slate-400 uppercase tracking-wider text-[11px]">
                      Calculated SHA-256 Digest
                    </span>
                    <Button
                      size="sm"
                      variant="light"
                      onPress={() => handleCopy(verificationResult.calculated_digest!, 'calculated')}
                      startContent={copiedKey === 'calculated' ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                      className="text-[11px] text-slate-600 dark:text-slate-300 font-medium h-6 px-2"
                    >
                      {copiedKey === 'calculated' ? 'Copied' : 'Copy'}
                    </Button>
                  </div>
                  <div
                    className={`p-3 rounded-lg font-mono text-xs break-all ${
                      verificationResult.status === 'Verified'
                        ? 'bg-slate-900 dark:bg-workstation-surface text-slate-100 border border-slate-700 dark:border-workstation-border'
                        : 'bg-rose-950 text-rose-200 border border-rose-800'
                    }`}
                  >
                    {verificationResult.calculated_digest}
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Pure Calculation Result Display */}
      {hashResult && !verificationResult && (
        <div className="bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border rounded-xs overflow-hidden">
          <div className="px-5 py-3 border-b border-slate-100 dark:border-workstation-border flex items-center justify-between bg-slate-50/50 dark:bg-workstation-surface">
            <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-2 uppercase tracking-wider">
              <Hash className="w-4 h-4 text-slate-600 dark:text-slate-400" />
              Calculated Digest
            </h2>
            <div>{renderStatusBadge('Calculated')}</div>
          </div>

          <div className="p-5 space-y-4">
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 p-4 rounded-lg bg-slate-50 dark:bg-workstation-surface border border-slate-100 dark:border-workstation-border text-xs">
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Target Name</span>
                <span className="font-semibold text-slate-800 dark:text-slate-200 truncate block mt-0.5">
                  {hashResult.target.display_name}
                </span>
              </div>
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">File Size</span>
                <span className="font-semibold text-slate-800 dark:text-slate-200 block mt-0.5 font-mono">
                  {formatBytes(hashResult.bytes_processed)}
                </span>
              </div>
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Computation Time</span>
                <span className="font-semibold text-slate-800 dark:text-slate-200 block mt-0.5 font-mono">
                  {hashResult.duration_ms} ms
                </span>
              </div>
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Algorithm</span>
                <span className="font-semibold text-slate-800 dark:text-slate-200 block mt-0.5">
                  {hashResult.algorithm}
                </span>
              </div>
            </div>

            <div className="space-y-1">
              <div className="flex items-center justify-between text-xs">
                <span className="font-semibold text-slate-600 dark:text-slate-400 uppercase tracking-wider text-[11px]">
                  Cryptographic SHA-256 Digest
                </span>
                <Button
                  size="sm"
                  variant="light"
                  onPress={() => handleCopy(hashResult.digest, 'hash-res')}
                  startContent={copiedKey === 'hash-res' ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                  className="text-[11px] text-slate-600 dark:text-slate-300 font-medium h-6 px-2"
                >
                  {copiedKey === 'hash-res' ? 'Copied' : 'Copy'}
                </Button>
              </div>
              <div className="p-3.5 rounded-lg bg-slate-900 dark:bg-workstation-surface text-slate-200 font-mono text-xs break-all select-all border border-slate-800 dark:border-workstation-border">
                {hashResult.digest}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* History Table */}
      <div className="bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border rounded-xs overflow-hidden">
        <div className="px-5 py-3 border-b border-slate-100 dark:border-workstation-border flex items-center justify-between bg-slate-50/50 dark:bg-workstation-surface">
          <div className="flex items-center gap-2">
            <Clock className="w-4 h-4 text-slate-500" />
            <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200 uppercase tracking-wider">
              Integrity Operations & Audit Record History
            </h2>
          </div>
          <Button
            size="sm"
            variant="flat"
            onPress={loadHistory}
            isDisabled={loadingRecords}
            startContent={<RefreshCw className={`w-3.5 h-3.5 ${loadingRecords ? 'animate-spin' : ''}`} />}
            className="text-xs font-medium text-slate-600 dark:text-slate-300 bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border h-7 px-3"
          >
            Refresh
          </Button>
        </div>

        <Table
          aria-label="Integrity Records History Table"
          removeWrapper
          classNames={{
            th: 'bg-slate-50 dark:bg-workstation-surface text-slate-500 dark:text-slate-400 text-[11px] font-semibold uppercase tracking-wider',
            td: 'py-3 text-xs text-slate-700 dark:text-slate-300 border-b border-slate-100 dark:border-workstation-border',
          }}
        >
          <TableHeader>
            <TableColumn>STATUS</TableColumn>
            <TableColumn>TARGET FILE</TableColumn>
            <TableColumn>ALGORITHM</TableColumn>
            <TableColumn>DIGEST</TableColumn>
            <TableColumn>SIZE</TableColumn>
            <TableColumn>ACTOR</TableColumn>
            <TableColumn>RECORDED AT</TableColumn>
          </TableHeader>
          <TableBody emptyContent="No integrity records found. Hash or verify a target file to create an audit-anchored record.">
            {records.map((rec) => (
              <TableRow key={rec.record_id} className="hover:bg-slate-50/60 dark:hover:bg-workstation-surface/50">
                <TableCell>
                  {renderStatusBadge(rec.verification_status)}
                </TableCell>
                <TableCell>
                  <div className="flex items-center gap-2">
                    <FileText className="w-4 h-4 text-slate-400 shrink-0" />
                    <div>
                      <p className="font-semibold text-slate-800 dark:text-slate-200 truncate max-w-xs" title={rec.target_path}>
                        {rec.target_name}
                      </p>
                      <p className="text-[10px] text-slate-400 font-mono truncate max-w-xs" title={rec.target_path}>
                        {rec.target_path}
                      </p>
                    </div>
                  </div>
                </TableCell>
                <TableCell className="font-medium text-slate-700 dark:text-slate-300">
                  {rec.algorithm}
                </TableCell>
                <TableCell>
                  <div className="flex items-center gap-1.5 font-mono text-[11px] text-slate-700 dark:text-slate-300">
                    <span className="truncate max-w-[140px]" title={rec.digest}>
                      {rec.digest}
                    </span>
                    <Button
                      isIconOnly
                      size="sm"
                      variant="light"
                      onPress={() => handleCopy(rec.digest, rec.record_id)}
                      className="w-5 h-5 min-w-5 text-slate-400 hover:text-slate-700 dark:hover:text-slate-200"
                      title="Copy full digest"
                    >
                      {copiedKey === rec.record_id ? (
                        <Check className="w-3.5 h-3.5 text-slate-800 dark:text-slate-200" />
                      ) : (
                        <Copy className="w-3.5 h-3.5" />
                      )}
                    </Button>
                  </div>
                </TableCell>
                <TableCell className="font-mono">
                  {formatBytes(rec.bytes_processed)}
                </TableCell>
                <TableCell>
                  {rec.actor_id || <span className="text-slate-400 italic">system</span>}
                </TableCell>
                <TableCell className="font-mono text-[11px] text-slate-400">
                  {new Date(rec.created_at).toLocaleString()}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </div>
  );
};

export default IntegrityVerificationPage;
