import React, { useState, useEffect } from 'react';
import {
  Search,
  History,
  RefreshCw,
} from 'lucide-react';
import { RecoveryWizard } from '../components/recovery/RecoveryWizard';
import { RecoveryResults } from '../components/recovery/RecoveryResults';
import { exportRecoveredFiles, listRecoveryJobs } from '../services/recovery';
import { RecoveryResult } from '../types/recovery';
import { useAuthStore } from '../stores/authStore';

export const ForensicRecoveryPage: React.FC = () => {
  const { sessionToken } = useAuthStore();
  const [activeTab, setActiveTab] = useState<'wizard' | 'history'>('wizard');
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
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-slate-200 pb-4">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <Search className="w-5 h-5 text-indigo-600" />
            <h2 className="text-base font-bold text-slate-900">
              Forensic File Recovery & Evidential Carving
            </h2>
          </div>
          <p className="text-xs text-slate-500">
            Strictly read-only bitstream carving consuming verified AcquisitionArtifacts. Non-destructive output isolation.
          </p>
        </div>

        {/* View Switcher */}
        <div className="flex items-center space-x-2 bg-slate-100 p-1 rounded-md">
          <button
            onClick={() => {
              setActiveTab('wizard');
              setSelectedHistoricalJob(null);
            }}
            className={`px-3 py-1.5 text-xs font-medium rounded transition-colors ${
              activeTab === 'wizard'
                ? 'bg-white text-slate-900 shadow-2xs font-semibold'
                : 'text-slate-600 hover:text-slate-900'
            }`}
          >
            New Recovery Job
          </button>
          <button
            onClick={() => setActiveTab('history')}
            className={`px-3 py-1.5 text-xs font-medium rounded transition-colors flex items-center gap-1.5 ${
              activeTab === 'history'
                ? 'bg-white text-slate-900 shadow-2xs font-semibold'
                : 'text-slate-600 hover:text-slate-900'
            }`}
          >
            <History className="w-3.5 h-3.5" />
            Historical Jobs
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
            <h3 className="text-xs font-semibold text-slate-800">
              Completed Forensic Recovery Jobs ({jobs.length})
            </h3>
            <button
              onClick={fetchJobs}
              disabled={loadingJobs}
              className="text-xs text-slate-500 hover:text-slate-700 flex items-center gap-1"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${loadingJobs ? 'animate-spin' : ''}`} />
              Refresh
            </button>
          </div>

          {jobs.length === 0 ? (
            <div className="p-8 text-center bg-white border border-slate-200 rounded-md text-slate-400 text-xs">
              No historical recovery jobs recorded yet.
            </div>
          ) : (
            <div className="grid grid-cols-1 gap-2.5">
              {jobs.map((job) => (
                <div
                  key={job.job_id}
                  onClick={() => setSelectedHistoricalJob(job)}
                  className="p-4 bg-white border border-slate-200 hover:border-slate-300 rounded-md cursor-pointer transition-all shadow-2xs flex items-center justify-between"
                >
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-bold text-slate-900 font-mono">
                        {job.job_id}
                      </span>
                      <span
                        className={`px-2 py-0.5 rounded text-[10px] font-semibold ${
                          job.status === 'Completed'
                            ? 'bg-emerald-50 text-emerald-700 border border-emerald-200'
                            : job.status === 'Cancelled'
                            ? 'bg-amber-50 text-amber-700 border border-amber-200'
                            : 'bg-rose-50 text-rose-700 border border-rose-200'
                        }`}
                      >
                        {job.status}
                      </span>
                    </div>
                    <div className="text-[11px] text-slate-500 flex flex-wrap gap-x-4">
                      <span>Source: <strong className="font-mono text-slate-700">{job.source_image_path}</strong></span>
                      <span>Recovered: <strong className="text-slate-700">{job.files_recovered} files</strong></span>
                      <span>Elapsed: <strong className="text-slate-700">{job.elapsed_seconds.toFixed(2)}s</strong></span>
                    </div>
                  </div>
                  <div className="text-right text-[11px] text-slate-400">
                    <div>{job.completed_at ? new Date(job.completed_at).toLocaleString() : job.started_at}</div>
                    <span className="text-indigo-600 text-xs font-medium hover:underline">
                      View Results &rarr;
                    </span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {activeTab === 'history' && selectedHistoricalJob && (
        <div className="space-y-3">
          <button
            onClick={() => setSelectedHistoricalJob(null)}
            className="text-xs text-slate-600 hover:text-slate-900 font-medium"
          >
            &larr; Back to Historical Jobs
          </button>
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
