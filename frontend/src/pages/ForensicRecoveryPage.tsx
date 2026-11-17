import React, { useState, useEffect } from 'react';
import {
  Search,
  History,
  RefreshCw,
  ArrowLeft,
} from 'lucide-react';
import {
  Button,
  Chip,
  Table,
  TableHeader,
  TableBody,
  TableColumn,
  TableRow,
  TableCell,
} from '@heroui/react';
import { RecoveryWizard } from '../components/recovery/RecoveryWizard';
import { RecoveryResults } from '../components/recovery/RecoveryResults';
import { exportRecoveredFiles, listRecoveryJobs } from '../services/recovery';
import { RecoveryResult } from '../types/recovery';
import { useAuthStore } from '../stores/authStore';

export const ForensicRecoveryPage: React.FC = () => {
  const { sessionToken } = useAuthStore();
  const [activeTab, setActiveTab] = useState<string>('wizard');
  const [jobs, setJobs] = useState<RecoveryResult[]>([]);
  const [loadingJobs, setLoadingJobs] = useState<boolean>(false);
  const [selectedHistoricalJob, setSelectedHistoricalJob] = useState<RecoveryResult | null>(null);

  const fetchJobs = async () => {
    setLoadingJobs(true);
    try {
      const jobList = await listRecoveryJobs();
      setJobs(jobList);
    } catch {
      // ignore
    } finally {
      setLoadingJobs(false);
    }
  };

  useEffect(() => {
    if (activeTab === 'history') {
      fetchJobs();
    }
  }, [activeTab]);

  return (
    <div className="space-y-6 text-slate-800 dark:text-slate-200 font-sans">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-slate-200 dark:border-[#26313D] pb-4">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <Search className="w-5 h-5 text-slate-700 dark:text-slate-300" />
            <h2 className="text-base font-bold text-slate-900 dark:text-white">
              Forensic File Recovery & Evidential Carving
            </h2>
          </div>
          <p className="text-xs text-slate-500 dark:text-slate-400">
            Strictly read-only bitstream carving consuming verified AcquisitionArtifacts. Non-destructive output isolation.
          </p>
        </div>

        {/* View Switcher Segmented Control */}
        <div className="inline-flex rounded-xs p-1 bg-slate-100 dark:bg-[#10161D] border border-slate-200 dark:border-[#26313D] text-xs">
          <button
            type="button"
            onClick={() => {
              setActiveTab('wizard');
              setSelectedHistoricalJob(null);
            }}
            className={`px-3 py-1.5 rounded-xs font-semibold transition-all cursor-pointer ${
              activeTab === 'wizard'
                ? 'bg-white dark:bg-[#1B2530] text-slate-900 dark:text-slate-100 font-bold border border-slate-300 dark:border-[#26313D] shadow-2xs'
                : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 border border-transparent'
            }`}
          >
            New Recovery Job
          </button>
          <button
            type="button"
            onClick={() => {
              setActiveTab('history');
              setSelectedHistoricalJob(null);
            }}
            className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xs font-semibold transition-all cursor-pointer ${
              activeTab === 'history'
                ? 'bg-white dark:bg-[#1B2530] text-slate-900 dark:text-slate-100 font-bold border border-slate-300 dark:border-[#26313D] shadow-2xs'
                : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 border border-transparent'
            }`}
          >
            <History className="w-3.5 h-3.5" />
            <span>Historical Jobs</span>
          </button>
        </div>
      </div>

      {/* Main Tab Views */}
      {activeTab === 'wizard' && (
        <RecoveryWizard sessionToken={sessionToken} />
      )}

      {activeTab === 'history' && !selectedHistoricalJob && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-xs font-semibold text-slate-800 dark:text-slate-200 uppercase tracking-wider">
              Completed Forensic Recovery Jobs ({jobs.length})
            </h3>
            <Button
              size="sm"
              variant="light"
              onPress={fetchJobs}
              isDisabled={loadingJobs}
              startContent={<RefreshCw className={`w-3.5 h-3.5 ${loadingJobs ? 'animate-spin' : ''}`} />}
              className="text-xs text-slate-500 dark:text-slate-400 h-8"
            >
              Refresh
            </Button>
          </div>

          {jobs.length === 0 ? (
            <div className="border border-slate-200 dark:border-[#26313D] rounded-sm p-8 text-center text-slate-400 text-xs">
              No historical recovery jobs recorded yet.
            </div>
          ) : (
            <Table
              aria-label="Historical recovery jobs"
              classNames={{
                wrapper: 'bg-transparent shadow-none p-0 border border-slate-200 dark:border-[#26313D] rounded-sm overflow-hidden',
                th: 'bg-slate-50 dark:bg-[#10161D] text-slate-700 dark:text-slate-300 text-[11px] font-semibold uppercase tracking-wider py-2.5 border-b border-slate-200 dark:border-[#26313D]',
                td: 'py-2.5 text-xs border-b border-slate-100 dark:border-[#26313D]/50',
              }}
            >
              <TableHeader>
                <TableColumn>JOB ID</TableColumn>
                <TableColumn>STATUS</TableColumn>
                <TableColumn>SOURCE IMAGE</TableColumn>
                <TableColumn>FILES RECOVERED</TableColumn>
                <TableColumn>DURATION</TableColumn>
                <TableColumn>COMPLETED AT</TableColumn>
                <TableColumn align="end">ACTION</TableColumn>
              </TableHeader>
              <TableBody>
                {jobs.map((job) => (
                  <TableRow key={job.job_id} className="hover:bg-slate-50 dark:hover:bg-[#1B2530]/50 transition-colors">
                    <TableCell className="font-mono font-medium text-slate-900 dark:text-slate-100">
                      {job.job_id}
                    </TableCell>
                    <TableCell>
                      <Chip
                        size="sm"
                        variant="flat"
                        className={`text-[10px] font-semibold uppercase ${
                          job.status?.toLowerCase() === 'completed'
                            ? 'bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700'
                            : job.status?.toLowerCase() === 'cancelled'
                            ? 'bg-amber-50 dark:bg-amber-950/40 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-800'
                            : 'bg-rose-50 dark:bg-rose-950/40 text-rose-700 dark:text-rose-300 border border-rose-200 dark:border-rose-900'
                        }`}
                      >
                        {job.status}
                      </Chip>
                    </TableCell>
                    <TableCell className="font-mono text-slate-600 dark:text-slate-400 max-w-xs truncate">
                      {job.source_image_path}
                    </TableCell>
                    <TableCell className="font-semibold text-slate-800 dark:text-slate-200">
                      {job.files_recovered} files
                    </TableCell>
                    <TableCell className="text-slate-500 dark:text-slate-400 font-mono text-[11px]">
                      {job.elapsed_seconds.toFixed(2)}s
                    </TableCell>
                    <TableCell className="text-slate-500 dark:text-slate-400 font-mono text-[11px]">
                      {job.completed_at ? new Date(job.completed_at).toLocaleString() : job.started_at}
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        size="sm"
                        variant="light"
                        onPress={() => setSelectedHistoricalJob(job)}
                        className="text-xs text-slate-700 dark:text-slate-300 font-medium h-7 px-2"
                      >
                        View Results &rarr;
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </div>
      )}

      {activeTab === 'history' && selectedHistoricalJob && (
        <div className="space-y-3">
          <Button
            size="sm"
            variant="light"
            onPress={() => setSelectedHistoricalJob(null)}
            startContent={<ArrowLeft className="w-3.5 h-3.5" />}
            className="text-xs text-slate-600 dark:text-slate-400 font-medium h-8"
          >
            Back to Historical Jobs
          </Button>
          <RecoveryResults
            result={selectedHistoricalJob}
            onExport={async (jId, dir) => {
              return exportRecoveredFiles({ job_id: jId, export_dir: dir });
            }}
            onNewJob={() => {
              setSelectedHistoricalJob(null);
              setActiveTab('wizard');
            }}
          />
        </div>
      )}
    </div>
  );
};

export default ForensicRecoveryPage;
