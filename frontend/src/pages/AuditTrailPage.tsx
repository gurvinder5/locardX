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
import {
  Button,
  Input,
  Chip,
} from '@heroui/react';
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
    } catch (err: unknown) {
      console.error('Failed to load audit events:', err);
      const msg = err && typeof err === 'object' && 'message' in err ? String((err as { message: string }).message) : 'Failed to query audit records from secure store.';
      setError(msg);
    } finally {
      setLoading(false);
    }
  };

  const handleVerifyChain = async () => {
    try {
      setVerifying(true);
      const result = await verifyAuditChain();
      setVerification(result);
    } catch (err: unknown) {
      console.error('Audit verification error:', err);
      const msg = err && typeof err === 'object' && 'message' in err ? String((err as { message: string }).message) : 'Verification process encountered an error.';
      setError(msg);
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
    <div className="space-y-6 w-full min-w-0">
      {/* Page Header */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 border-b border-slate-200 dark:border-workstation-border pb-5">
        <div>
          <h1 className="text-xl font-bold tracking-tight text-slate-900 dark:text-slate-100 flex items-center gap-2">
            <ClipboardList className="w-5 h-5 text-slate-700 dark:text-slate-300" />
            Tamper-Evident Audit Trail
          </h1>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
            Cryptographic SHA-256 hash-chained operational ledger verifying all authentication, case, imaging, recovery, and sanitization events.
          </p>
        </div>

        <div className="flex items-center gap-2.5">
          <Button
            size="sm"
            onPress={handleVerifyChain}
            isDisabled={verifying}
            isLoading={verifying}
            startContent={!verifying && <ShieldCheck className="w-3.5 h-3.5" />}
            className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 font-semibold text-xs px-4 shadow-sm"
          >
            {verifying ? 'Verifying Hashes...' : 'Verify Cryptographic Chain'}
          </Button>

          <Button
            size="sm"
            variant="flat"
            onPress={loadData}
            isDisabled={loading}
            startContent={<RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />}
            className="text-xs font-medium bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border text-slate-700 dark:text-slate-300"
          >
            Refresh
          </Button>
        </div>
      </div>

      {/* Verification Status Ribbon */}
      {verification && (
        <div
          className={`border rounded-xs p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 ${
            verification.is_valid
              ? 'bg-slate-100 dark:bg-workstation-surface border-slate-300 dark:border-workstation-border text-slate-900 dark:text-slate-100'
              : 'bg-rose-50 dark:bg-rose-950/30 border-rose-200 dark:border-rose-900/50 text-rose-950 dark:text-rose-200'
          }`}
        >
          <div className="flex items-center gap-3">
            {verification.is_valid ? (
              <div className="w-8 h-8 rounded-full bg-slate-200 dark:bg-workstation-panel flex items-center justify-center text-slate-800 dark:text-slate-200 shrink-0">
                <CheckCircle2 className="w-4 h-4" />
              </div>
            ) : (
              <div className="w-8 h-8 rounded-full bg-rose-100 dark:bg-rose-900/50 flex items-center justify-center text-rose-600 dark:text-rose-400 shrink-0">
                <XCircle className="w-4 h-4" />
              </div>
            )}
            <div>
              <div className="flex items-center gap-2">
                <span className="text-xs font-bold uppercase tracking-wider">
                  {verification.is_valid ? 'Audit Chain Verified' : 'Audit Integrity Broken'}
                </span>
                <Chip
                  size="sm"
                  variant="flat"
                  className={`text-[10px] font-semibold ${
                    verification.is_valid
                      ? 'bg-slate-200 dark:bg-workstation-panel text-slate-800 dark:text-slate-200'
                      : 'bg-rose-100 dark:bg-rose-950 text-rose-800 dark:text-rose-300'
                  }`}
                >
                  {verification.total_events} Sequential Records Validated
                </Chip>
              </div>
              <p className="text-xs mt-0.5 text-slate-600 dark:text-slate-300">
                {verification.details}
              </p>
            </div>
          </div>

          <div className="text-right shrink-0">
            <div className="text-[11px] font-mono text-slate-500 dark:text-slate-400">
              Last Verified Sequence: #{verification.last_verified_sequence}
            </div>
            {verification.broken_sequence && (
              <div className="text-[11px] font-mono text-rose-600 dark:text-rose-400 font-bold">
                Violation Detected at Sequence: #{verification.broken_sequence}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Error alert */}
      {error && (
        <div className="p-3.5 bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 rounded-xs text-xs text-rose-700 dark:text-rose-300 flex items-center gap-2">
          <ShieldAlert className="w-4 h-4 shrink-0 text-rose-600" />
          <span>{error}</span>
        </div>
      )}

      {/* Filter and Search Bar */}
      <div className="bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border rounded-xs p-3 flex flex-col sm:flex-row items-center justify-between gap-3">
        <div className="w-full sm:w-72">
          <Input
            size="sm"
            placeholder="Search events, targets, hashes..."
            value={searchQuery}
            onValueChange={setSearchQuery}
            startContent={<Search className="w-3.5 h-3.5 text-slate-400 shrink-0" />}
            classNames={{
              inputWrapper: 'bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded-xs',
            }}
          />
        </div>

        <div className="flex items-center gap-3 w-full sm:w-auto justify-between sm:justify-end">
          <div className="flex items-center gap-1.5 text-xs text-slate-600 dark:text-slate-300">
            <Filter className="w-3.5 h-3.5 text-slate-400" />
            <select
              value={typeFilter}
              onChange={(e) => setTypeFilter(e.target.value)}
              className="text-xs bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded-xs px-2.5 py-1.5 text-slate-700 dark:text-slate-300 focus:outline-none"
            >
              <option value="all">All Event Types ({events.length})</option>
              {eventTypes.map((type) => (
                <option key={type} value={type}>
                  {type}
                </option>
              ))}
            </select>
          </div>

          <div className="flex items-center gap-1.5 text-xs text-slate-600 dark:text-slate-300">
            <span className="text-[11px] text-slate-400">Limit:</span>
            <select
              value={limit}
              onChange={(e) => setLimit(Number(e.target.value))}
              className="text-xs bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded-xs px-2.5 py-1.5 text-slate-700 dark:text-slate-300 focus:outline-none"
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
      <div className="bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border rounded-xs overflow-hidden">
        <div className="px-4 py-3 border-b border-slate-200 dark:border-workstation-border bg-slate-50/50 dark:bg-workstation-surface flex items-center justify-between text-xs font-semibold text-slate-700 dark:text-slate-300">
          <div className="flex items-center gap-2">
            <Hash className="w-4 h-4 text-slate-500" />
            <span className="uppercase tracking-wider font-bold text-slate-800 dark:text-slate-200">Cryptographic Chain Entries</span>
            <Chip size="sm" variant="flat" className="font-mono text-[10px] bg-slate-200 dark:bg-workstation-panel text-slate-700 dark:text-slate-300">
              {filteredEvents.length} shown
            </Chip>
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
          <div className="divide-y divide-slate-100 dark:divide-workstation-border overflow-x-auto">
            {filteredEvents.map((event) => (
              <div
                key={event.event_id}
                className="p-4 hover:bg-slate-50/80 dark:hover:bg-workstation-surface/60 transition-colors space-y-2 text-xs"
              >
                {/* Event header line */}
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-xs font-bold text-slate-800 dark:text-slate-200 bg-slate-100 dark:bg-workstation-surface px-1.5 py-0.5 rounded-xs border border-slate-200 dark:border-workstation-border">
                      #{event.sequence_number}
                    </span>
                    <Chip size="sm" variant="flat" className="font-semibold text-[11px] bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-workstation-border">
                      {event.event_type}
                    </Chip>
                    {event.target_ref && (
                      <span className="font-mono text-[11px] text-slate-600 dark:text-slate-400 bg-slate-100 dark:bg-workstation-surface px-1.5 py-0.5 rounded-xs border border-slate-200 dark:border-workstation-border">
                        {event.target_ref}
                      </span>
                    )}
                  </div>

                  <div className="flex items-center gap-3 text-[11px] text-slate-400 font-mono">
                    <span className="flex items-center gap-1 text-slate-500 dark:text-slate-400">
                      <User className="w-3 h-3 text-slate-400" />
                      {event.actor_id || 'SYSTEM'}
                    </span>
                    <span className="flex items-center gap-1 text-slate-500 dark:text-slate-400">
                      <Clock className="w-3 h-3 text-slate-400" />
                      {new Date(event.timestamp).toLocaleString()}
                    </span>
                  </div>
                </div>

                {/* Event Details */}
                <div className="text-slate-700 dark:text-slate-300 bg-slate-50 dark:bg-workstation-surface p-2.5 rounded-xs border border-slate-200 dark:border-workstation-border font-sans text-xs">
                  {event.details}
                </div>

                {/* Hashes: Prev Hash -> Current Hash */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-2 pt-1 font-mono text-[10px]">
                  <div className="flex items-center justify-between bg-slate-50/80 dark:bg-workstation-surface px-2.5 py-1.5 rounded-xs border border-slate-200/80 dark:border-workstation-border text-slate-500 dark:text-slate-400">
                    <span className="shrink-0 text-slate-400 mr-2">PREV:</span>
                    <span className="truncate" title={event.prev_hash}>
                      {event.prev_hash}
                    </span>
                    <Button
                      isIconOnly
                      size="sm"
                      variant="light"
                      onPress={() => copyToClipboard(event.prev_hash, `${event.event_id}-prev`)}
                      className="w-5 h-5 min-w-5 ml-1.5 text-slate-400 hover:text-slate-800 dark:hover:text-slate-200"
                      title="Copy previous hash"
                    >
                      {copiedHash === `${event.event_id}-prev` ? (
                        <Check className="w-3 h-3 text-slate-800 dark:text-slate-200" />
                      ) : (
                        <Copy className="w-3 h-3" />
                      )}
                    </Button>
                  </div>

                  <div className="flex items-center justify-between bg-slate-50/80 dark:bg-workstation-surface px-2.5 py-1.5 rounded-xs border border-slate-200/80 dark:border-workstation-border text-slate-700 dark:text-slate-300">
                    <span className="shrink-0 text-slate-800 dark:text-slate-200 font-semibold mr-2">HASH:</span>
                    <span className="truncate font-semibold text-slate-800 dark:text-slate-200" title={event.current_hash}>
                      {event.current_hash}
                    </span>
                    <Button
                      isIconOnly
                      size="sm"
                      variant="light"
                      onPress={() => copyToClipboard(event.current_hash, `${event.event_id}-curr`)}
                      className="w-5 h-5 min-w-5 ml-1.5 text-slate-400 hover:text-slate-800 dark:hover:text-slate-200"
                      title="Copy current SHA-256 hash"
                    >
                      {copiedHash === `${event.event_id}-curr` ? (
                        <Check className="w-3 h-3 text-slate-800 dark:text-slate-200" />
                      ) : (
                        <Copy className="w-3 h-3" />
                      )}
                    </Button>
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
