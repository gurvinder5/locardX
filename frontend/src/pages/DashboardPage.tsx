import React from 'react';
import { ShieldCheck, FileText, Search, Cpu } from 'lucide-react';
import { AppInfo } from '../services/tauri';

interface DashboardPageProps {
  appInfo: AppInfo | null;
  loading: boolean;
}

export const DashboardPage: React.FC<DashboardPageProps> = ({ appInfo, loading }) => {
  return (
    <div className="space-y-6 text-slate-800 font-sans">
      {/* Workstation Status Banner */}
      <section className="bg-white border border-slate-200 rounded-md p-5 shadow-2xs">
        <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
          <div>
            <h2 className="text-sm font-semibold text-slate-900 flex items-center gap-2">
              <ShieldCheck className="w-4 h-4 text-emerald-600" />
              LocardX Forensic Core &bull; Rust IPC Engine
            </h2>
            <p className="text-xs text-slate-500 mt-1 max-w-2xl">
              Non-destructive SQLite storage, cryptographic SHA-256 audit chaining, and Win32 read-only device inspection are verified and active.
            </p>
          </div>
          <div className="flex items-center gap-2 font-mono text-xs text-slate-700 bg-slate-50 px-3 py-1.5 rounded border border-slate-200">
            <span className="w-2 h-2 rounded-full bg-emerald-500" />
            <span>IPC: {loading ? 'Querying...' : appInfo?.build_status || 'Operational'}</span>
          </div>
        </div>
      </section>

      {/* Core Architectural Pillars */}
      <div>
        <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-500 mb-3">
          Architecture & System Modules
        </h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {/* Pillar 1 */}
          <div className="bg-white border border-slate-200 rounded-md p-5 shadow-2xs hover:border-slate-300 transition-colors">
            <div className="flex items-center justify-between mb-2.5">
              <div className="flex items-center gap-2 text-slate-800">
                <Cpu className="w-4 h-4 text-slate-600" />
                <h4 className="text-xs font-semibold">Secure Drive Eraser</h4>
              </div>
              <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded bg-slate-100 text-slate-700 border border-slate-200">
                Pillar 1
              </span>
            </div>
            <p className="text-xs text-slate-500 leading-relaxed">
              Block sanitization, NIST SP 800-88 compliance, and hardware ATA/NVMe sanitize interlocks.
            </p>
          </div>

          {/* Pillar 2 */}
          <div className="bg-white border border-slate-200 rounded-md p-5 shadow-2xs hover:border-slate-300 transition-colors">
            <div className="flex items-center justify-between mb-2.5">
              <div className="flex items-center gap-2 text-slate-800">
                <FileText className="w-4 h-4 text-slate-600" />
                <h4 className="text-xs font-semibold">Secure File Eraser</h4>
              </div>
              <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded bg-slate-100 text-slate-700 border border-slate-200">
                Pillar 2
              </span>
            </div>
            <p className="text-xs text-slate-500 leading-relaxed">
              Targeted file destruction, NTFS alternate data stream wiping, and slack space clearing.
            </p>
          </div>

          {/* Pillar 3 */}
          <div className="bg-white border border-slate-200 rounded-md p-5 shadow-2xs hover:border-slate-300 transition-colors">
            <div className="flex items-center justify-between mb-2.5">
              <div className="flex items-center gap-2 text-slate-800">
                <Search className="w-4 h-4 text-slate-600" />
                <h4 className="text-xs font-semibold">Carving & Recovery</h4>
              </div>
              <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded bg-slate-100 text-slate-700 border border-slate-200">
                Pillar 3
              </span>
            </div>
            <p className="text-xs text-slate-500 leading-relaxed">
              Read-only evidence preservation, signature carving, The Sleuth Kit traversal, and artifact scoring.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default DashboardPage;
