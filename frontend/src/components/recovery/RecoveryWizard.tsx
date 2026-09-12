import React, { useState } from 'react';
import {
  ArrowRight,
  ArrowLeft,
  CheckCircle2,
  ShieldCheck,
  Play,
  RotateCcw,
  Sliders,
  FileCheck,
  AlertCircle,
} from 'lucide-react';
import { useRecovery } from '../../hooks/useRecovery';
import { RecoveryMode, RecoveryOptions, RecoverySourceSnapshot } from '../../types/recovery';
import { RecoverySourceSelector } from './RecoverySourceSelector';
import { RecoveryProgress } from './RecoveryProgress';
import { RecoveryResults } from './RecoveryResults';

interface RecoveryWizardProps {
  sessionToken?: string | null;
}

export const RecoveryWizard: React.FC<RecoveryWizardProps> = ({ sessionToken }) => {
  const {
    sources,
    loadingSources,
    selectedArtifact,
    plan,
    isPlanning,
    activeResult,
    progress,
    isScanning,
    error,
    selectArtifact,
    preparePlan,
    runRecovery,
    cancelCurrent,
    exportFiles,
    resetState,
  } = useRecovery(sessionToken);

  const [currentStep, setCurrentStep] = useState<number>(1);
  const [selectedSourceSnapshot, setSelectedSourceSnapshot] = useState<RecoverySourceSnapshot | null>(null);

  // Recovery Options State
  const [recoveryMode, setRecoveryMode] = useState<RecoveryMode>('all');
  const [outputDir, setOutputDir] = useState<string>('C:\\RecoveredFiles');
  const [minConfidence, setMinConfidence] = useState<number>(40);
  const [enableFragmentation, setEnableFragmentation] = useState<boolean>(true);
  const [selectedTypes, setSelectedTypes] = useState<string[]>([
    'jpeg',
    'png',
    'pdf',
    'zip',
    'docx',
    'sqlite',
    'text',
  ]);

  const fileTypeOptions = [
    { id: 'jpeg', label: 'JPEG Images (.jpg, .jpeg)' },
    { id: 'png', label: 'PNG Images (.png)' },
    { id: 'pdf', label: 'PDF Documents (.pdf)' },
    { id: 'zip', label: 'ZIP Archives (.zip)' },
    { id: 'docx', label: 'Office OpenXML (.docx, .xlsx)' },
    { id: 'sqlite', label: 'SQLite Databases (.db, .sqlite)' },
    { id: 'text', label: 'Plain Text (.txt, .log)' },
  ];

  const handleToggleType = (id: string) => {
    if (selectedTypes.includes(id)) {
      setSelectedTypes(selectedTypes.filter((t) => t !== id));
    } else {
      setSelectedTypes([...selectedTypes, id]);
    }
  };

  const handleSelectSource = (src: RecoverySourceSnapshot) => {
    setSelectedSourceSnapshot(src);
    // Create an AcquisitionArtifact wrapper for validation and planning
    const artifact = {
      acquisition_id: src.acquisition_id,
      image_path: src.image_path,
      image_format: 'raw',
      image_size_bytes: src.image_size_bytes,
      image_sha256: src.image_sha256,
      source_device_snapshot: {
        device_id: src.original_device_id,
        display_name: src.original_device_id,
        serial_number: src.original_serial,
        media_type: 'HDD',
        capacity_bytes: src.image_size_bytes,
        sector_size: 512,
        is_removable: false,
        is_system: false,
        snapshot_timestamp: src.verified_at,
      },
      acquisition_timestamp: src.verified_at,
      is_verified: src.is_trusted,
      audit_reference: 'audit-ref-handoff',
    };
    selectArtifact(artifact);
  };

  const handleGeneratePlan = async () => {
    const options: RecoveryOptions = {
      recovery_mode: recoveryMode,
      target_file_types: selectedTypes.length > 0 ? selectedTypes : null,
      output_directory: outputDir,
      enable_fragment_reconstruction: enableFragmentation,
      min_confidence_score: minConfidence,
      chunk_size_bytes: 1024 * 1024,
    };

    const newPlan = await preparePlan(options);
    if (newPlan) {
      setCurrentStep(3);
    }
  };

  const handleStartExecution = async () => {
    setCurrentStep(4);
    await runRecovery();
  };

  const handleReset = () => {
    resetState();
    setSelectedSourceSnapshot(null);
    setCurrentStep(1);
  };

  return (
    <div className="space-y-6">
      {/* Stepper Header */}
      <div className="flex items-center justify-between border-b border-slate-200 pb-3">
        <div className="flex items-center space-x-6 text-xs">
          <span
            className={`font-semibold flex items-center gap-1.5 ${
              currentStep === 1 ? 'text-indigo-600' : 'text-slate-500'
            }`}
          >
            <span className="w-5 h-5 rounded-full border flex items-center justify-center text-[10px]">
              1
            </span>
            Source Image
          </span>
          <span
            className={`font-semibold flex items-center gap-1.5 ${
              currentStep === 2 ? 'text-indigo-600' : 'text-slate-500'
            }`}
          >
            <span className="w-5 h-5 rounded-full border flex items-center justify-center text-[10px]">
              2
            </span>
            Carving Options
          </span>
          <span
            className={`font-semibold flex items-center gap-1.5 ${
              currentStep === 3 ? 'text-indigo-600' : 'text-slate-500'
            }`}
          >
            <span className="w-5 h-5 rounded-full border flex items-center justify-center text-[10px]">
              3
            </span>
            Pre-Flight Review
          </span>
          <span
            className={`font-semibold flex items-center gap-1.5 ${
              currentStep === 4 ? 'text-indigo-600' : 'text-slate-500'
            }`}
          >
            <span className="w-5 h-5 rounded-full border flex items-center justify-center text-[10px]">
              4
            </span>
            Execution & Results
          </span>
        </div>

        {currentStep > 1 && !isScanning && (
          <button
            onClick={handleReset}
            className="text-xs text-slate-500 hover:text-slate-700 flex items-center gap-1"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            Reset
          </button>
        )}
      </div>

      {error && (
        <div className="p-3 bg-rose-50 border border-rose-200 rounded-md text-xs text-rose-700 flex items-center gap-2">
          <AlertCircle className="w-4 h-4 shrink-0 text-rose-600" />
          <span>{error}</span>
        </div>
      )}

      {/* Step 1: Select Source */}
      {currentStep === 1 && (
        <div className="space-y-4">
          <RecoverySourceSelector
            sources={sources}
            loading={loadingSources}
            selectedSourceId={selectedSourceSnapshot?.source_id ?? null}
            onSelectSource={handleSelectSource}
          />

          <div className="flex justify-end pt-2">
            <button
              onClick={() => setCurrentStep(2)}
              disabled={!selectedArtifact}
              className="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-semibold rounded flex items-center gap-1.5 transition-colors disabled:opacity-40"
            >
              Continue to Options
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* Step 2: Configure Options */}
      {currentStep === 2 && (
        <div className="space-y-5">
          <div className="grid grid-cols-2 gap-4">
            {/* Left Column: Mode & Output */}
            <div className="space-y-4 bg-white border border-slate-200 p-4 rounded-md shadow-2xs">
              <h4 className="text-xs font-semibold text-slate-800 flex items-center gap-1.5">
                <Sliders className="w-4 h-4 text-indigo-600" />
                Recovery Strategy & Modes
              </h4>

              <div className="space-y-2">
                <label className="block text-xs font-medium text-slate-700">Recovery Mode</label>
                <div className="space-y-1.5">
                  {[
                    { key: 'all', title: 'All (Filesystem Metadata + Sliding-Window Carving)' },
                    { key: 'filesystem_only', title: 'Filesystem Metadata Only (NTFS MFT, FAT32 deleted entries)' },
                    { key: 'carving_only', title: 'Raw Bitstream Carving Only (Structure & Signature search)' },
                  ].map((m) => (
                    <label
                      key={m.key}
                      className={`flex items-start gap-2.5 p-2.5 border rounded cursor-pointer text-xs ${
                        recoveryMode === m.key
                          ? 'border-indigo-600 bg-indigo-50/40 text-indigo-900 font-medium'
                          : 'border-slate-200 text-slate-600 hover:bg-slate-50'
                      }`}
                    >
                      <input
                        type="radio"
                        name="rec_mode"
                        checked={recoveryMode === m.key}
                        onChange={() => setRecoveryMode(m.key as RecoveryMode)}
                        className="mt-0.5 text-indigo-600"
                      />
                      <span>{m.title}</span>
                    </label>
                  ))}
                </div>
              </div>

              <div className="space-y-1 pt-2 border-t border-slate-100">
                <label className="block text-xs font-medium text-slate-700">Output Export Directory</label>
                <input
                  type="text"
                  value={outputDir}
                  onChange={(e) => setOutputDir(e.target.value)}
                  placeholder="C:\RecoveredFiles"
                  className="w-full px-3 py-1.5 text-xs border border-slate-300 rounded font-mono bg-white focus:outline-none focus:ring-1 focus:ring-indigo-500"
                />
                <p className="text-[10px] text-slate-400">
                  Recovered files will be written here strictly isolated from the source image.
                </p>
              </div>

              <div className="space-y-1 pt-2 border-t border-slate-100">
                <div className="flex justify-between text-xs font-medium text-slate-700">
                  <span>Minimum Confidence Score Threshold</span>
                  <span className="font-mono font-bold text-indigo-600">{minConfidence}%</span>
                </div>
                <input
                  type="range"
                  min={0}
                  max={95}
                  step={5}
                  value={minConfidence}
                  onChange={(e) => setMinConfidence(parseInt(e.target.value, 10))}
                  className="w-full"
                />
                <div className="flex justify-between text-[10px] text-slate-400">
                  <span>Permissive (0%)</span>
                  <span>Standard (40%)</span>
                  <span>Strict High Only (85%)</span>
                </div>
              </div>
            </div>

            {/* Right Column: File Types */}
            <div className="space-y-4 bg-white border border-slate-200 p-4 rounded-md shadow-2xs">
              <h4 className="text-xs font-semibold text-slate-800 flex items-center gap-1.5">
                <FileCheck className="w-4 h-4 text-indigo-600" />
                Target File Types for Carving
              </h4>
              <div className="space-y-2">
                {fileTypeOptions.map((opt) => (
                  <label
                    key={opt.id}
                    className="flex items-center gap-2 p-2 rounded hover:bg-slate-50 cursor-pointer text-xs text-slate-700"
                  >
                    <input
                      type="checkbox"
                      checked={selectedTypes.includes(opt.id)}
                      onChange={() => handleToggleType(opt.id)}
                      className="rounded text-indigo-600"
                    />
                    <span>{opt.label}</span>
                  </label>
                ))}
              </div>

              <div className="pt-3 border-t border-slate-100">
                <label className="flex items-center gap-2 cursor-pointer text-xs text-slate-700">
                  <input
                    type="checkbox"
                    checked={enableFragmentation}
                    onChange={(e) => setEnableFragmentation(e.target.checked)}
                    className="rounded text-indigo-600"
                  />
                  <span>Enable cluster-aligned fragment reconstruction</span>
                </label>
                <p className="text-[10px] text-slate-400 mt-1 pl-5">
                  Evaluates discontinuities without hallucinating or fabricating missing data.
                </p>
              </div>
            </div>
          </div>

          <div className="flex justify-between pt-2">
            <button
              onClick={() => setCurrentStep(1)}
              className="px-4 py-2 border border-slate-200 text-slate-600 hover:bg-slate-50 text-xs font-medium rounded flex items-center gap-1.5"
            >
              <ArrowLeft className="w-4 h-4" />
              Back
            </button>
            <button
              onClick={handleGeneratePlan}
              disabled={isPlanning}
              className="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-semibold rounded flex items-center gap-1.5 transition-colors disabled:opacity-50"
            >
              Review Pre-Flight Plan
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* Step 3: Pre-Flight Review */}
      {currentStep === 3 && plan && (
        <div className="space-y-4">
          <div className="bg-white border border-slate-200 rounded-md p-5 space-y-4 shadow-2xs">
            <div className="flex items-center gap-2 border-b border-slate-100 pb-3">
              <ShieldCheck className="w-5 h-5 text-emerald-600" />
              <div>
                <h4 className="text-xs font-semibold text-slate-800">
                  Pre-Flight Evidential Safety Verification
                </h4>
                <p className="text-[11px] text-slate-500">
                  Plan generated and cryptographically cross-verified against forensic source
                </p>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4 text-xs">
              <div className="space-y-2">
                <div>
                  <span className="text-slate-400 text-[11px] block">Source Image:</span>
                  <span className="font-mono text-slate-800 font-semibold">{plan.source_image_path}</span>
                </div>
                <div>
                  <span className="text-slate-400 text-[11px] block">Verified Image SHA-256:</span>
                  <span className="font-mono text-[11px] text-slate-700 break-all bg-slate-50 p-1 rounded border border-slate-100 block">
                    {plan.source_image_sha256}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400 text-[11px] block">Recovery Mode:</span>
                  <span className="text-slate-800 font-semibold">{plan.options.recovery_mode.toUpperCase()}</span>
                </div>
              </div>

              <div className="space-y-2">
                <div>
                  <span className="text-slate-400 text-[11px] block">Output Directory:</span>
                  <span className="font-mono text-slate-800 font-semibold">{plan.options.output_directory}</span>
                </div>
                <div>
                  <span className="text-slate-400 text-[11px] block">Target Types:</span>
                  <span className="text-slate-700">
                    {plan.options.target_file_types ? plan.options.target_file_types.join(', ') : 'All registered types'}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400 text-[11px] block">Minimum Confidence Score:</span>
                  <span className="text-emerald-700 font-bold">{plan.options.min_confidence_score}%</span>
                </div>
              </div>
            </div>

            <div className="p-3 bg-emerald-50/60 border border-emerald-200 rounded text-xs text-emerald-800 space-y-1">
              <div className="font-semibold flex items-center gap-1">
                <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600" />
                Evidential Guarantee: Strictly Non-Destructive
              </div>
              <p className="text-[11px] text-emerald-700">
                The evidence image is accessed strictly read-only with streaming SHA-256 hash checks.
                No physical devices or source image sectors will be modified. Recovered files are written exclusively into the designated output folder.
              </p>
            </div>
          </div>

          <div className="flex justify-between pt-2">
            <button
              onClick={() => setCurrentStep(2)}
              className="px-4 py-2 border border-slate-200 text-slate-600 hover:bg-slate-50 text-xs font-medium rounded flex items-center gap-1.5"
            >
              <ArrowLeft className="w-4 h-4" />
              Back
            </button>
            <button
              onClick={handleStartExecution}
              className="px-5 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-semibold rounded flex items-center gap-1.5 transition-colors shadow-xs"
            >
              <Play className="w-4 h-4 fill-white" />
              Start Forensic Recovery
            </button>
          </div>
        </div>
      )}

      {/* Step 4: Execution / Results */}
      {currentStep === 4 && (
        <div className="space-y-4">
          {isScanning && <RecoveryProgress progress={progress} onCancel={cancelCurrent} />}
          {activeResult && (
            <RecoveryResults
              result={activeResult}
              onExport={exportFiles}
              onNewJob={handleReset}
            />
          )}
        </div>
      )}
    </div>
  );
};
