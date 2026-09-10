import React from 'react';

export const App: React.FC = () => {
  return (
    <div className="min-h-screen bg-slate-900 text-slate-100 flex flex-col">
      <header className="border-b border-slate-800 px-6 py-4 flex items-center justify-between">
        <h1 className="text-xl font-bold tracking-wider text-cyan-400">LOCARDX</h1>
        <span className="text-xs px-2.5 py-1 rounded bg-slate-800 text-slate-400 border border-slate-700">
          Scaffold Phase
        </span>
      </header>
      <main className="flex-1 p-8 max-w-6xl mx-auto w-full">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <div className="bg-slate-800/60 border border-slate-700 rounded-lg p-6">
            <h2 className="text-lg font-semibold text-rose-400 mb-2">Secure Drive Eraser</h2>
            <p className="text-sm text-slate-400">
              Whole-disk sanitization, multi-pass overwrites, and hardware sanitize dispatch.
            </p>
          </div>
          <div className="bg-slate-800/60 border border-slate-700 rounded-lg p-6">
            <h2 className="text-lg font-semibold text-amber-400 mb-2">Secure File Eraser</h2>
            <p className="text-sm text-slate-400">
              Surgical file/folder destruction, metadata zeroing, and ADS clearing.
            </p>
          </div>
          <div className="bg-slate-800/60 border border-slate-700 rounded-lg p-6">
            <h2 className="text-lg font-semibold text-emerald-400 mb-2">File Carving & Recovery</h2>
            <p className="text-sm text-slate-400">
              Read-only evidence carving, TSK filesystem traversal, and artifact reconstruction.
            </p>
          </div>
        </div>
      </main>
      <footer className="border-t border-slate-800 px-6 py-3 text-xs text-slate-500 text-center">
        LocardX Forensic Platform &bull; Evidence Integrity & Secure Sanitization
      </footer>
    </div>
  );
};

export default App;
