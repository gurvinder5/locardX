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
        <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-emerald-50 text-emerald-700 border border-emerald-200">
          <CheckCircle2 className="w-3.5 h-3.5" />
          VERIFIED
        </span>
      );
    }

    if (status === 'Mismatch' || (typeof status === 'object' && 'Mismatch' in status)) {
      return (
        <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-rose-50 text-rose-700 border border-rose-200">
          <XCircle className="w-3.5 h-3.5" />
          MISMATCH
        </span>
      );
    }

    if (typeof status === 'object' && 'UnableToVerify' in status) {
      return (
        <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-amber-50 text-amber-700 border border-amber-200">
          <AlertTriangle className="w-3.5 h-3.5" />
          UNABLE TO VERIFY
        </span>
      );
    }

    return (
      <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-blue-50 text-blue-700 border border-blue-200">
        <Hash className="w-3.5 h-3.5" />
        CALCULATED
      </span>
    );
  };

  return (
    <div className="space-y-6 max-w-7xl mx-auto p-4 sm:p-6 lg:p-8">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 border-b border-slate-200 pb-5">
        <div>
          <div className="flex items-center gap-2">
            <div className="p-2 rounded-lg bg-indigo-50 border border-indigo-100 text-indigo-600">
              <ShieldCheck className="w-6 h-6" />
            </div>
            <div>
              <h1 className="text-xl font-bold text-slate-900">
                Cryptographic Integrity & Evidence Verification
              </h1>
              <p className="text-sm text-slate-500">
                Strictly read-only SHA-256 evidence hashing and integrity verification.
              </p>
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <span className="inline-flex items-center gap-1 px-2.5 py-1 rounded-md text-xs font-medium bg-slate-100 text-slate-700 border border-slate-200">
            <Shield className="w-3.5 h-3.5 text-emerald-600" />
            Read-Only Protection Active
          </span>
        </div>
      </div>

      {/* Safety Notice */}
      <div className="p-4 rounded-lg bg-slate-100/80 border border-slate-200 text-sm text-slate-600 flex items-start gap-3">
        <ShieldCheck className="w-5 h-5 text-emerald-600 shrink-0 mt-0.5" />
        <div>
          <p className="font-medium text-slate-900">Non-Destructive Forensic Verification</p>
          <p className="mt-0.5 text-xs text-slate-500">
            Hashing executes strictly read-only streaming passes in 64 KiB chunks with bounded memory.
            Target files are never written, modified, or truncated. Every calculation is anchored into the
            tamper-evident SHA-256 audit hash-chain.
          </p>
        </div>
      </div>

      {/* Error Alert */}
      {errorMessage && (
        <div className="p-4 rounded-lg bg-rose-50 border border-rose-200 text-sm text-rose-700 flex items-start gap-3">
          <AlertTriangle className="w-5 h-5 text-rose-500 shrink-0 mt-0.5" />
          <div className="flex-1">
            <p className="font-semibold">Verification Error</p>
            <p className="mt-0.5 text-xs text-rose-600">{errorMessage}</p>
          </div>
          <button
            onClick={() => setErrorMessage(null)}
            className="text-xs text-rose-500 hover:text-rose-700 font-medium"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Input Form Card */}
      <div className="bg-white rounded-xl border border-slate-200 shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-slate-100 bg-slate-50/50">
          <h2 className="text-sm font-semibold text-slate-800 flex items-center gap-2">
            <Search className="w-4 h-4 text-slate-500" />
            Target Specification & Parameters
          </h2>
        </div>
        <form onSubmit={handleCalculate} className="p-6 space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
            <div className="md:col-span-3 space-y-1">
              <label className="block text-xs font-semibold text-slate-700 uppercase tracking-wider">
                Target File Path
              </label>
              <input
                type="text"
                value={targetPath}
                onChange={(e) => setTargetPath(e.target.value)}
                placeholder="e.g. C:\Evidence\disk_image.raw or /tmp/evidence.bin"
                className="w-full px-3 py-2 text-sm rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 font-mono"
              />
              <p className="text-[11px] text-slate-500">
                Absolute or relative path to the evidence file on the local system.
              </p>
            </div>

            <div className="space-y-1">
              <label className="block text-xs font-semibold text-slate-700 uppercase tracking-wider">
                Cryptographic Algorithm
              </label>
              <select
                value={algorithm}
                disabled
                className="w-full px-3 py-2 text-sm rounded-lg border border-slate-300 bg-slate-50 text-slate-600 cursor-not-allowed font-medium"
              >
                <option value="Sha256">SHA-256 (FIPS 180-4)</option>
              </select>
              <p className="text-[11px] text-slate-500">Standard 256-bit NIST cryptographic hash.</p>
            </div>
          </div>

          <div className="space-y-1">
            <label className="block text-xs font-semibold text-slate-700 uppercase tracking-wider">
              Expected SHA-256 Digest (Optional for calculation, required for verification)
            </label>
            <input
              type="text"
              value={expectedDigest}
              onChange={(e) => setExpectedDigest(e.target.value)}
              placeholder="e.g. e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
              className="w-full px-3 py-2 text-sm rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 font-mono text-xs"
            />
            <p className="text-[11px] text-slate-500">
              Provide known hash to perform cryptographic comparison. Constant-time comparison prevents timing analysis.
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-3 pt-2">
            <button
              type="submit"
              disabled={isCalculating || isVerifying || !targetPath.trim()}
              className="inline-flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-lg text-white bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shadow-sm"
            >
              {isCalculating ? (
                <RefreshCw className="w-4 h-4 animate-spin" />
              ) : (
                <Hash className="w-4 h-4" />
              )}
              {isCalculating ? 'Computing SHA-256...' : 'Calculate Hash'}
            </button>

            <button
              type="button"
              onClick={handleVerify}
              disabled={
                isCalculating ||
                isVerifying ||
                !targetPath.trim() ||
                !expectedDigest.trim()
              }
              className="inline-flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-lg text-white bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shadow-sm"
            >
              {isVerifying ? (
                <RefreshCw className="w-4 h-4 animate-spin" />
              ) : (
                <CheckCircle2 className="w-4 h-4" />
              )}
              {isVerifying ? 'Verifying Integrity...' : 'Verify Against Expected'}
            </button>
          </div>
        </form>
      </div>

      {/* Verification Result Display */}
      {verificationResult && (
        <div className="bg-white rounded-xl border border-slate-200 shadow-sm overflow-hidden animate-in fade-in duration-200">
          <div className="px-6 py-4 border-b border-slate-100 flex items-center justify-between bg-slate-50/50">
            <h2 className="text-sm font-semibold text-slate-800 flex items-center gap-2">
              <ShieldCheck className="w-4 h-4 text-slate-600" />
              Verification Results
            </h2>
            <div>{renderStatusBadge(verificationResult.status)}</div>
          </div>

          <div className="p-6 space-y-6">
            {/* Status summary banner */}
            {verificationResult.status === 'Verified' && (
              <div className="p-4 rounded-lg bg-emerald-50 border border-emerald-200 flex items-start gap-3">
                <CheckCircle2 className="w-5 h-5 text-emerald-600 shrink-0 mt-0.5" />
                <div>
                  <h3 className="text-sm font-bold text-emerald-900">
                    Cryptographic Integrity Confirmed
                  </h3>
                  <p className="text-xs text-emerald-700 mt-0.5">
                    The calculated SHA-256 digest strictly matches the expected digest. The target evidence
                    has not been modified or corrupted.
                  </p>
                </div>
              </div>
            )}

            {typeof verificationResult.status === 'object' &&
              'Mismatch' in verificationResult.status && (
                <div className="p-4 rounded-lg bg-rose-50 border border-rose-200 flex items-start gap-3">
                  <XCircle className="w-5 h-5 text-rose-600 shrink-0 mt-0.5" />
                  <div>
                    <h3 className="text-sm font-bold text-rose-900">
                      Cryptographic Mismatch Detected
                    </h3>
                    <p className="text-xs text-rose-700 mt-0.5">
                      The calculated SHA-256 digest DOES NOT MATCH the expected digest.
                      Target contents differ from the recorded reference.
                    </p>
                  </div>
                </div>
              )}

            {typeof verificationResult.status === 'object' &&
              'UnableToVerify' in verificationResult.status && (
                <div className="p-4 rounded-lg bg-amber-50 border border-amber-200 flex items-start gap-3">
                  <AlertTriangle className="w-5 h-5 text-amber-600 shrink-0 mt-0.5" />
                  <div>
                    <h3 className="text-sm font-bold text-amber-900">
                      Unable to Complete Verification
                    </h3>
                    <p className="text-xs text-amber-700 mt-0.5">
                      {verificationResult.status.UnableToVerify.reason}
                    </p>
                  </div>
                </div>
              )}

            {/* Target Identity Metadata */}
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 p-4 rounded-lg bg-slate-50 border border-slate-100 text-xs">
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Target Name</span>
                <span className="font-semibold text-slate-800 truncate block mt-0.5">
                  {verificationResult.target.display_name}
                </span>
              </div>
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Size Processed</span>
                <span className="font-semibold text-slate-800 block mt-0.5">
                  {formatBytes(verificationResult.bytes_processed)}
                </span>
              </div>
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Elapsed Time</span>
                <span className="font-semibold text-slate-800 block mt-0.5">
                  {verificationResult.duration_ms} ms
                </span>
              </div>
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Target Type</span>
                <span className="font-semibold text-slate-800 block mt-0.5">
                  {verificationResult.target.target_type}
                </span>
              </div>
            </div>

            {/* Hashes Comparison */}
            <div className="space-y-3">
              <div className="space-y-1">
                <div className="flex items-center justify-between text-xs">
                  <span className="font-semibold text-slate-600 uppercase tracking-wider text-[11px]">
                    Expected Digest
                  </span>
                  <button
                    onClick={() => handleCopy(verificationResult.expected_digest, 'expected')}
                    className="inline-flex items-center gap-1 text-[11px] text-indigo-600 hover:text-indigo-800 font-medium"
                  >
                    {copiedKey === 'expected' ? (
                      <Check className="w-3.5 h-3.5 text-emerald-600" />
                    ) : (
                      <Copy className="w-3.5 h-3.5" />
                    )}
                    {copiedKey === 'expected' ? 'Copied' : 'Copy'}
                  </button>
                </div>
                <div className="p-3 rounded-lg bg-slate-900 text-slate-100 font-mono text-xs break-all">
                  {verificationResult.expected_digest}
                </div>
              </div>

              {verificationResult.calculated_digest && (
                <div className="space-y-1">
                  <div className="flex items-center justify-between text-xs">
                    <span className="font-semibold text-slate-600 uppercase tracking-wider text-[11px]">
                      Calculated SHA-256 Digest
                    </span>
                    <button
                      onClick={() =>
                        handleCopy(verificationResult.calculated_digest!, 'calculated')
                      }
                      className="inline-flex items-center gap-1 text-[11px] text-indigo-600 hover:text-indigo-800 font-medium"
                    >
                      {copiedKey === 'calculated' ? (
                        <Check className="w-3.5 h-3.5 text-emerald-600" />
                      ) : (
                        <Copy className="w-3.5 h-3.5" />
                      )}
                      {copiedKey === 'calculated' ? 'Copied' : 'Copy'}
                    </button>
                  </div>
                  <div
                    className={`p-3 rounded-lg font-mono text-xs break-all ${
                      verificationResult.status === 'Verified'
                        ? 'bg-emerald-950 text-emerald-200 border border-emerald-800'
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
        <div className="bg-white rounded-xl border border-slate-200 shadow-sm overflow-hidden animate-in fade-in duration-200">
          <div className="px-6 py-4 border-b border-slate-100 flex items-center justify-between bg-slate-50/50">
            <h2 className="text-sm font-semibold text-slate-800 flex items-center gap-2">
              <Hash className="w-4 h-4 text-slate-600" />
              Calculated Digest
            </h2>
            <div>{renderStatusBadge('Calculated')}</div>
          </div>

          <div className="p-6 space-y-4">
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 p-4 rounded-lg bg-slate-50 border border-slate-100 text-xs">
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Target Name</span>
                <span className="font-semibold text-slate-800 truncate block mt-0.5">
                  {hashResult.target.display_name}
                </span>
              </div>
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">File Size</span>
                <span className="font-semibold text-slate-800 block mt-0.5">
                  {formatBytes(hashResult.bytes_processed)}
                </span>
              </div>
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Computation Time</span>
                <span className="font-semibold text-slate-800 block mt-0.5">
                  {hashResult.duration_ms} ms
                </span>
              </div>
              <div>
                <span className="text-slate-400 block uppercase font-medium text-[10px]">Algorithm</span>
                <span className="font-semibold text-slate-800 block mt-0.5">
                  {hashResult.algorithm}
                </span>
              </div>
            </div>

            <div className="space-y-1">
              <div className="flex items-center justify-between text-xs">
                <span className="font-semibold text-slate-600 uppercase tracking-wider text-[11px]">
                  Cryptographic SHA-256 Digest
                </span>
                <button
                  onClick={() => handleCopy(hashResult.digest, 'hash-res')}
                  className="inline-flex items-center gap-1 text-[11px] text-indigo-600 hover:text-indigo-800 font-medium"
                >
                  {copiedKey === 'hash-res' ? (
                    <Check className="w-3.5 h-3.5 text-emerald-600" />
                  ) : (
                    <Copy className="w-3.5 h-3.5" />
                  )}
                  {copiedKey === 'hash-res' ? 'Copied' : 'Copy'}
                </button>
              </div>
              <div className="p-3.5 rounded-lg bg-slate-900 text-emerald-400 font-mono text-xs break-all select-all">
                {hashResult.digest}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* History Table Card */}
      <div className="bg-white rounded-xl border border-slate-200 shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-slate-100 flex items-center justify-between bg-slate-50/50">
          <div className="flex items-center gap-2">
            <Clock className="w-4 h-4 text-slate-500" />
            <h2 className="text-sm font-semibold text-slate-800">
              Integrity Operations & Audit Record History
            </h2>
          </div>
          <button
            onClick={loadHistory}
            disabled={loadingRecords}
            className="inline-flex items-center gap-1.5 px-3 py-1 text-xs font-medium rounded-md text-slate-600 hover:text-slate-900 hover:bg-slate-100 transition-colors"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loadingRecords ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs text-slate-600">
            <thead className="bg-slate-50 text-[11px] font-semibold text-slate-500 uppercase tracking-wider border-b border-slate-100">
              <tr>
                <th className="px-6 py-3">Status</th>
                <th className="px-6 py-3">Target File</th>
                <th className="px-6 py-3">Algorithm</th>
                <th className="px-6 py-3">Digest</th>
                <th className="px-6 py-3">Size</th>
                <th className="px-6 py-3">Actor</th>
                <th className="px-6 py-3">Recorded At</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100">
              {records.length === 0 ? (
                <tr>
                  <td colSpan={7} className="px-6 py-8 text-center text-slate-400">
                    {loadingRecords
                      ? 'Loading integrity records...'
                      : 'No integrity records found. Hash or verify a target file to create an audit-anchored record.'}
                  </td>
                </tr>
              ) : (
                records.map((rec) => (
                  <tr key={rec.record_id} className="hover:bg-slate-50/60 transition-colors">
                    <td className="px-6 py-3 whitespace-nowrap">
                      {renderStatusBadge(rec.verification_status)}
                    </td>
                    <td className="px-6 py-3">
                      <div className="flex items-center gap-2">
                        <FileText className="w-4 h-4 text-slate-400 shrink-0" />
                        <div>
                          <p className="font-semibold text-slate-800 truncate max-w-xs" title={rec.target_path}>
                            {rec.target_name}
                          </p>
                          <p className="text-[10px] text-slate-400 font-mono truncate max-w-xs" title={rec.target_path}>
                            {rec.target_path}
                          </p>
                        </div>
                      </div>
                    </td>
                    <td className="px-6 py-3 whitespace-nowrap font-medium text-slate-700">
                      {rec.algorithm}
                    </td>
                    <td className="px-6 py-3">
                      <div className="flex items-center gap-1.5 font-mono text-[11px] text-slate-700">
                        <span className="truncate max-w-[140px]" title={rec.digest}>
                          {rec.digest}
                        </span>
                        <button
                          onClick={() => handleCopy(rec.digest, rec.record_id)}
                          className="text-slate-400 hover:text-slate-700 p-0.5"
                          title="Copy full digest"
                        >
                          {copiedKey === rec.record_id ? (
                            <Check className="w-3.5 h-3.5 text-emerald-600" />
                          ) : (
                            <Copy className="w-3.5 h-3.5" />
                          )}
                        </button>
                      </div>
                    </td>
                    <td className="px-6 py-3 whitespace-nowrap text-slate-600">
                      {formatBytes(rec.bytes_processed)}
                    </td>
                    <td className="px-6 py-3 whitespace-nowrap text-slate-600">
                      {rec.actor_id || <span className="text-slate-400 italic">system</span>}
                    </td>
                    <td className="px-6 py-3 whitespace-nowrap text-slate-500 text-[11px]">
                      {new Date(rec.created_at).toLocaleString()}
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
