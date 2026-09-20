import React, { useEffect, useState } from 'react';
import {
  ClipboardList,
  ShieldCheck,
  ShieldAlert,
  Search,
  RefreshCw,
  Hash,
  Clock,
  User,
  Filter,
  CheckCircle2,
  XCircle,
  Copy,
  Check,
} from 'lucide-react';
import { listAuditEvents, verifyAuditChain } from '../services/case';
import { AuditEvent, AuditChainVerification } from '../types/case';

export const AuditTrailPage: React.FC = () => {
  const [events, setEvents] = useState<AuditEvent[]>([]);
  const [verification, setVerification] = useState<AuditChainVerification | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [verifying, setVerifying] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [copiedHash, setCopiedHash] = useState<string | null>(null);

  // Filters
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [typeFilter, setTypeFilter] = useState<string>('all');
  const [limit, setLimit] = useState<number>(100);

  const loadData = async () => {
    try {
      setLoading(true);
      setError(null);
      const [eventList, verifResult] = await Promise.all([
        listAuditEvents(limit),
        verifyAuditChain(),
      ]);
      setEvents(eventList);
      setVerification(verifResult);
    } catch (err: any) {
      console.error('Failed to load audit events:', err);
      setError(err?.message || 'Failed to query audit records from secure store.');
    } finally {
      setLoading(false);
    }
  };

  const handleVerifyChain = async () => {
    try {
      setVerifying(true);
      const result = await verifyAuditChain();
      setVerification(result);
    } catch (err: any) {
      console.error('Audit verification error:', err);
      setError(err?.message || 'Verification process encountered an error.');
    } finally {
      setVerifying(false);
    }
  };

  useEffect(() => {
    loadData();
  }, [limit]);

  const copyToClipboard = (text: string, id: string) => {
    navigator.clipboard.writeText(text);
    setCopiedHash(id);
    setTimeout(() => setCopiedHash(null), 2000);
  };

  // Distinct event types
  const eventTypes = Array.from(new Set(events.map((e) => e.event_type))).sort();

  // Filtered events
  const filteredEvents = events.filter((e) => {
    const matchesSearch =
      searchQuery === '' ||
      e.event_id.toLowerCase().includes(searchQuery.toLowerCase()) ||
      e.event_type.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (e.target_ref && e.target_ref.toLowerCase().includes(searchQuery.toLowerCase())) ||
      (e.actor_id && e.actor_id.toLowerCase().includes(searchQuery.toLowerCase())) ||
      e.details.toLowerCase().includes(searchQuery.toLowerCase()) ||
      e.current_hash.toLowerCase().includes(searchQuery.toLowerCase());

    const matchesType = typeFilter === 'all' || e.event_type === typeFilter;

    return matchesSearch && matchesType;
  });

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
        <div>
          <h2 className="text-base font-bold tracking-tight text-slate-900 flex items-center gap-2">
            <ClipboardList className="w-5 h-5 text-indigo-600" />
            Tamper-Evident Audit Trail
          </h2>
          <p className="text-xs text-slate-500 mt-0.5">
            Cryptographic SHA-256 hash-chained operational ledger verifying all authentication, case, imaging, recovery, and sanitization events.
          </p>
        </div>

        <div className="flex items-center gap-2.5">
          <button
            onClick={handleVerifyChain}
            disabled={verifying}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white rounded text-xs font-medium shadow-2xs transition-colors"
          >
            <ShieldCheck className={`w-3.5 h-3.5 ${verifying ? 'animate-spin' : ''}`} />
            <span>{verifying ? 'Verifying Hashes...' : 'Verify Cryptographic Chain'}</span>
          </button>

          <button
            onClick={loadData}
            disabled={loading}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-white border border-slate-200 hover:bg-slate-50 text-slate-700 rounded text-xs font-medium shadow-2xs transition-colors"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin text-indigo-600' : ''}`} />
            <span>Refresh</span>
          </button>
        </div>
      </div>

      {/* Verification Status Card */}
      {verification && (
        <div
          className={`p-4 rounded-md border flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 shadow-2xs ${
            verification.is_valid
              ? 'bg-emerald-50/70 border-emerald-200 text-emerald-950'
              : 'bg-rose-50 border-rose-200 text-rose-950'
          }`}
        >
          <div className="flex items-center gap-3">
            {verification.is_valid ? (
              <div className="w-9 h-9 rounded-full bg-emerald-100 flex items-center justify-center text-emerald-600 shrink-0">
                <CheckCircle2 className="w-5 h-5" />
              </div>
            ) : (
              <div className="w-9 h-9 rounded-full bg-rose-100 flex items-center justify-center text-rose-600 shrink-0">
                <XCircle className="w-5 h-5" />
              </div>
            )}
            <div>
              <div className="flex items-center gap-2">
                <span className="text-xs font-bold uppercase tracking-wider">
                  {verification.is_valid ? 'Audit Chain Verified' : 'Audit Integrity Broken'}
                </span>
                <span
                  className={`text-[10px] font-semibold px-2 py-0.5 rounded ${
                    verification.is_valid
                      ? 'bg-emerald-100 text-emerald-800'
                      : 'bg-rose-100 text-rose-800'
                  }`}
                >
                  {verification.total_events} Sequential Records Validated
                </span>
              </div>
              <p className="text-xs mt-0.5 text-slate-600">
                {verification.details}
              </p>
            </div>
          </div>

          <div className="text-right shrink-0">
            <div className="text-[11px] font-mono text-slate-500">
              Last Verified Sequence: #{verification.last_verified_sequence}
            </div>
            {verification.broken_sequence && (
              <div className="text-[11px] font-mono text-rose-600 font-bold">
                Violation Detected at Sequence: #{verification.broken_sequence}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Error alert */}
      {error && (
        <div className="p-3.5 bg-rose-50 border border-rose-200 rounded text-xs text-rose-700 flex items-center gap-2">
          <ShieldAlert className="w-4 h-4 shrink-0 text-rose-600" />
          <span>{error}</span>
        </div>
      )}

      {/* Filter and Search Bar */}
      <div className="bg-white border border-slate-200 rounded-md p-3.5 shadow-2xs flex flex-col sm:flex-row items-center justify-between gap-3">
        <div className="relative w-full sm:w-72">
          <Search className="w-3.5 h-3.5 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
          <input
            type="text"
            placeholder="Search events, targets, hashes..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-8 pr-3 py-1.5 text-xs bg-slate-50 border border-slate-200 rounded focus:bg-white focus:outline-none focus:border-indigo-500"
          />
        </div>

        <div className="flex items-center gap-3 w-full sm:w-auto justify-between sm:justify-end">
          <div className="flex items-center gap-1.5 text-xs text-slate-600">
            <Filter className="w-3.5 h-3.5 text-slate-400" />
            <select
              value={typeFilter}
              onChange={(e) => setTypeFilter(e.target.value)}
              className="text-xs bg-slate-50 border border-slate-200 rounded px-2.5 py-1.5 focus:bg-white focus:outline-none focus:border-indigo-500 text-slate-700"
            >
              <option value="all">All Event Types ({events.length})</option>
              {eventTypes.map((type) => (
                <option key={type} value={type}>
                  {type}
                </option>
              ))}
            </select>
          </div>

          <div className="flex items-center gap-1.5 text-xs text-slate-600">
            <span className="text-[11px] text-slate-400">Limit:</span>
            <select
              value={limit}
              onChange={(e) => setLimit(Number(e.target.value))}
              className="text-xs bg-slate-50 border border-slate-200 rounded px-2 py-1.5 focus:bg-white focus:outline-none focus:border-indigo-500 text-slate-700"
            >
              <option value={50}>50</option>
              <option value={100}>100</option>
              <option value={250}>250</option>
              <option value={500}>500</option>
            </select>
          </div>
        </div>
      </div>

      {/* Events Table / Ledger */}
      <div className="bg-white border border-slate-200 rounded-md shadow-2xs overflow-hidden">
        <div className="px-4 py-3 border-b border-slate-200 bg-slate-50/50 flex items-center justify-between text-xs font-semibold text-slate-700">
          <div className="flex items-center gap-2">
            <Hash className="w-4 h-4 text-slate-500" />
            <span>Cryptographic Chain Entries</span>
            <span className="text-[10px] bg-slate-200 text-slate-700 px-1.5 py-0.5 rounded font-mono">
              {filteredEvents.length} shown
            </span>
          </div>
          <span className="text-[11px] text-slate-400 font-normal">
            Linked list where current_hash = SHA256(prev_hash + sequence + event_data)
          </span>
        </div>

        {filteredEvents.length === 0 ? (
          <div className="p-12 text-center text-slate-400 text-xs">
            {loading ? 'Reading audit chain from SQLite store...' : 'No audit records match the current filter.'}
          </div>
        ) : (
          <div className="divide-y divide-slate-100 overflow-x-auto">
            {filteredEvents.map((event) => (
              <div
                key={event.event_id}
                className="p-4 hover:bg-slate-50/80 transition-colors space-y-2 text-xs"
              >
                {/* Event header line */}
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-xs font-bold text-slate-800 bg-slate-100 px-1.5 py-0.5 rounded border border-slate-200">
                      #{event.sequence_number}
                    </span>
                    <span className="font-semibold text-indigo-700 bg-indigo-50 px-2 py-0.5 rounded text-[11px] border border-indigo-100">
                      {event.event_type}
                    </span>
                    {event.target_ref && (
                      <span className="font-mono text-[11px] text-slate-600 bg-slate-100 px-1.5 py-0.5 rounded border border-slate-200">
                        {event.target_ref}
                      </span>
                    )}
                  </div>

                  <div className="flex items-center gap-3 text-[11px] text-slate-400 font-mono">
                    <span className="flex items-center gap-1 text-slate-500">
                      <User className="w-3 h-3 text-slate-400" />
                      {event.actor_id || 'SYSTEM'}
                    </span>
                    <span className="flex items-center gap-1 text-slate-500">
                      <Clock className="w-3 h-3 text-slate-400" />
                      {new Date(event.timestamp).toLocaleString()}
                    </span>
                  </div>
                </div>

                {/* Event Details */}
                <div className="text-slate-700 bg-slate-50 p-2.5 rounded border border-slate-150 font-sans text-xs">
                  {event.details}
                </div>

                {/* Hashes: Prev Hash -> Current Hash */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-2 pt-1 font-mono text-[10px]">
                  <div className="flex items-center justify-between bg-slate-50/80 px-2.5 py-1.5 rounded border border-slate-200/80 text-slate-500">
                    <span className="shrink-0 text-slate-400 mr-2">PREV:</span>
                    <span className="truncate" title={event.prev_hash}>
                      {event.prev_hash}
                    </span>
                    <button
                      onClick={() => copyToClipboard(event.prev_hash, `${event.event_id}-prev`)}
                      className="ml-1.5 p-0.5 hover:text-slate-800"
                      title="Copy previous hash"
                    >
                      {copiedHash === `${event.event_id}-prev` ? (
                        <Check className="w-3 h-3 text-emerald-600" />
                      ) : (
                        <Copy className="w-3 h-3" />
                      )}
                    </button>
                  </div>

                  <div className="flex items-center justify-between bg-slate-50/80 px-2.5 py-1.5 rounded border border-slate-200/80 text-slate-700">
                    <span className="shrink-0 text-indigo-600 font-semibold mr-2">HASH:</span>
                    <span className="truncate font-semibold text-slate-800" title={event.current_hash}>
                      {event.current_hash}
                    </span>
                    <button
                      onClick={() => copyToClipboard(event.current_hash, `${event.event_id}-curr`)}
                      className="ml-1.5 p-0.5 hover:text-slate-800"
                      title="Copy current SHA-256 hash"
                    >
                      {copiedHash === `${event.event_id}-curr` ? (
                        <Check className="w-3 h-3 text-emerald-600" />
                      ) : (
                        <Copy className="w-3 h-3" />
                      )}
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default AuditTrailPage;
