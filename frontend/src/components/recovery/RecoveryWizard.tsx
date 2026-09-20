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
  Loader2,
  Briefcase,
} from 'lucide-react';
import { useRecovery } from '../../hooks/useRecovery';
import { RecoveryMode, RecoveryOptions, RecoverySourceSnapshot } from '../../types/recovery';
import { RecoverySourceSelector } from './RecoverySourceSelector';
import { RecoveryProgress } from './RecoveryProgress';
import { RecoveryResults } from './RecoveryResults';
import { useCaseStore } from '../../stores/caseStore';
import { ActiveCaseRequiredModal } from '../case/ActiveCaseRequiredModal';

interface RecoveryWizardProps {
  sessionToken?: string | null;
}

export const RecoveryWizard: React.FC<RecoveryWizardProps> = ({ sessionToken }) => {
  const { activeCase } = useCaseStore();
  const [showCaseModal, setShowCaseModal] = useState(false);
  const isCaseActive = activeCase && (activeCase.status === 'open' || activeCase.status === 'in_progress');

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
    workflowState,
    selectArtifact,
    preparePlan,
    runRecovery,
    cancelCurrent,
    exportFiles,
    resetState,
  } = useRecovery(sessionToken);

  const [currentStep, setCurrentStep] = useState<number>(1);
  const [selectedSourceSnapshot, setSelectedSourceSnapshot] = useState<RecoverySourceSnapshot | null>(null);
  const [step2Error, setStep2Error] = useState<string | null>(null);

  // Recovery Options State
  const [recoveryMode, setRecoveryMode] = useState<RecoveryMode>('all');
  const [outputDir, setOutputDir] = useState<string>('C:\\RecoveredFiles');
  const [minConfidence, setMinConfidence] = useState<number>(40);
  const [enableFragmentation, setEnableFragmentation] = useState<boolean>(true);
  const [selectedTypes, setSelectedTypes] = useState<string[]>([
    'jpeg',
    'png',
    'gif',
    'bmp',
    'tiff',
    'webp',
    'pdf',
    'doc',
    'docx',
    'xls',
    'xlsx',
    'ppt',
    'pptx',
    'rtf',
    'zip',
    '7z',
    'rar',
    'mp3',
    'wav',
    'mp4',
    'avi',
    'mkv',
    'sqlite',
    'text',
  ]);

  const fileTypeOptions = [
    { id: 'jpeg', label: 'JPEG Images (.jpg, .jpeg)' },
    { id: 'png', label: 'PNG Images (.png)' },
    { id: 'gif', label: 'GIF Images (.gif)' },
    { id: 'bmp', label: 'Bitmap Images (.bmp)' },
    { id: 'tiff', label: 'TIFF Images (.tiff, .tif)' },
    { id: 'webp', label: 'WebP Images (.webp)' },
    { id: 'pdf', label: 'PDF Documents (.pdf)' },
    { id: 'doc', label: 'Word Legacy CFBF (.doc)' },
    { id: 'docx', label: 'Word OpenXML (.docx)' },
    { id: 'xls', label: 'Excel Legacy CFBF (.xls)' },
    { id: 'xlsx', label: 'Excel OpenXML (.xlsx)' },
    { id: 'ppt', label: 'PowerPoint Legacy (.ppt)' },
    { id: 'pptx', label: 'PowerPoint OpenXML (.pptx)' },
    { id: 'rtf', label: 'Rich Text Format (.rtf)' },
    { id: 'zip', label: 'ZIP Archives (.zip)' },
    { id: '7z', label: '7-Zip Archives (.7z)' },
    { id: 'rar', label: 'RAR Archives (.rar)' },
    { id: 'mp3', label: 'MP3 Audio (.mp3)' },
    { id: 'wav', label: 'WAV Audio (.wav)' },
    { id: 'mp4', label: 'MP4 Video (.mp4)' },
    { id: 'avi', label: 'AVI Video (.avi)' },
    { id: 'mkv', label: 'MKV Video (.mkv)' },
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
    setStep2Error(null);

    if (!isCaseActive) {
      setShowCaseModal(true);
      return;
    }

    if (!selectedArtifact) {
      setStep2Error('Please select a verified forensic evidence source on Step 1 before continuing.');
      return;
    }

    const trimmedOutDir = outputDir.trim();
    if (!trimmedOutDir) {
      setStep2Error('Output Export Directory is required. Please specify a destination folder for recovered files.');
      return;
    }

    if (recoveryMode === 'carving_only' && selectedTypes.length === 0) {
      setStep2Error('Raw bitstream carving requires at least one target file type to be selected.');
      return;
    }

    const options: RecoveryOptions = {
      recovery_mode: recoveryMode,
      target_file_types: selectedTypes.length > 0 ? selectedTypes : null,
      output_directory: trimmedOutDir,
      enable_fragment_reconstruction: enableFragmentation,
      min_confidence_score: minConfidence,
      chunk_size_bytes: 1024 * 1024,
    };

    try {
      const newPlan = await preparePlan(options);
      if (newPlan) {
        setStep2Error(null);
        setCurrentStep(3);
      }
    } catch (err) {
      setStep2Error(err instanceof Error ? err.message : String(err));
    }
  };

  const handleStartExecution = async () => {
    if (!isCaseActive) {
      setShowCaseModal(true);
      return;
    }
    setCurrentStep(4);
    await runRecovery();
  };

  const handleReset = () => {
    resetState();
    setSelectedSourceSnapshot(null);
    setStep2Error(null);
    setCurrentStep(1);
  };

  return (
    <div className="space-y-6">
      {/* Active Case Warning Banner if not active */}
      {!isCaseActive && (
        <div className="bg-amber-50 border border-amber-300 rounded-lg p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3 shadow-2xs">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-amber-100 text-amber-800 border border-amber-200 shrink-0">
              <Briefcase className="w-5 h-5" />
            </div>
            <div>
              <h4 className="text-xs font-bold text-amber-950 uppercase tracking-wider">
                Active Investigation Case Required
              </h4>
              <p className="text-xs text-amber-800">
                Forensic file recovery and evidential carving must be bound to an open investigation case to record evidentiary chain of custody.
              </p>
            </div>
          </div>
          <button
            onClick={() => setShowCaseModal(true)}
            className="px-3.5 py-2 text-xs font-bold text-amber-900 bg-amber-200 hover:bg-amber-300 border border-amber-300 rounded-lg shadow-xs transition-colors shrink-0"
          >
            Select or Create Case
          </button>
        </div>
      )}

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
              <div className="flex items-center justify-between">
                <h4 className="text-xs font-semibold text-slate-800 flex items-center gap-1.5">
                  <FileCheck className="w-4 h-4 text-indigo-600" />
                  Target File Types ({selectedTypes.length}/{fileTypeOptions.length})
                </h4>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={() => setSelectedTypes(fileTypeOptions.map((o) => o.id))}
                    className="text-[10px] text-indigo-600 hover:text-indigo-800 font-medium cursor-pointer"
                  >
                    Select All
                  </button>
                  <span className="text-slate-300">|</span>
                  <button
                    type="button"
                    onClick={() => setSelectedTypes([])}
                    className="text-[10px] text-slate-500 hover:text-slate-700 cursor-pointer"
                  >
                    Clear
                  </button>
                </div>
              </div>
              <div className="space-y-1.5 max-h-72 overflow-y-auto pr-1">
                {fileTypeOptions.map((opt) => (
                  <label
                    key={opt.id}
                    className="flex items-center gap-2 p-1.5 rounded hover:bg-slate-50 cursor-pointer text-xs text-slate-700"
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

          {(step2Error || error) && (
            <div className="p-3 bg-rose-50 border border-rose-200 rounded-md text-xs text-rose-700 flex items-center gap-2">
              <AlertCircle className="w-4 h-4 shrink-0 text-rose-600" />
              <span>{step2Error || error}</span>
            </div>
          )}

          <div className="flex justify-between pt-2">
            <button
              onClick={() => {
                setStep2Error(null);
                setCurrentStep(1);
              }}
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
              {isPlanning ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Generating Plan...
                </>
              ) : (
                <>
                  Review Pre-Flight Plan
                  <ArrowRight className="w-4 h-4" />
                </>
              )}
            </button>
          </div>
        </div>
      )}

      {/* Step 3: Pre-Flight Review */}
      {currentStep === 3 && (
        !plan ? (
          <div className="p-6 bg-amber-50 border border-amber-200 rounded-md text-center space-y-3">
            <AlertCircle className="w-8 h-8 text-amber-600 mx-auto" />
            <p className="text-xs text-amber-800 font-medium">
              No active recovery plan found. Please return to options to configure and generate a pre-flight plan.
            </p>
            <button
              onClick={() => setCurrentStep(2)}
              className="px-3 py-1.5 bg-amber-600 hover:bg-amber-700 text-white text-xs font-semibold rounded inline-flex items-center gap-1.5"
            >
              <ArrowLeft className="w-3.5 h-3.5" />
              Back to Carving Options
            </button>
          </div>
        ) : (
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
        )
      )}

      {/* Step 4: Execution / Results */}
      {currentStep === 4 && (
        <div className="space-y-4">
          {/* Active progress phase */}
          {(isScanning ||
            workflowState === 'STARTING' ||
            workflowState === 'ANALYZING' ||
            workflowState === 'RECOVERING' ||
            workflowState === 'VALIDATING') && (
            <RecoveryProgress
              progress={progress}
              sourceName={selectedSourceSnapshot?.image_path}
              recoveryMode={recoveryMode}
              onCancel={cancelCurrent}
            />
          )}

          {/* Failure state */}
          {workflowState === 'FAILED' && (
            <div className="bg-white border border-rose-200 rounded-md p-6 space-y-4 shadow-xs">
              <div className="flex items-start gap-3">
                <div className="p-2.5 rounded-full bg-rose-50 text-rose-600 shrink-0">
                  <AlertCircle className="w-6 h-6" />
                </div>
                <div className="space-y-1">
                  <h4 className="text-sm font-bold text-slate-900">
                    Filesystem Recovery Failed
                  </h4>
                  <p className="text-xs text-slate-600">
                    The filesystem analysis engine encountered an unrecoverable condition and halted to preserve evidentiary integrity.
                  </p>
                  <div className="mt-2 p-3 bg-rose-50 border border-rose-100 rounded text-xs font-mono text-rose-800 break-words">
                    Reason: {error || 'An unexpected error halted filesystem recovery.'}
                  </div>
                </div>
              </div>

              <div className="flex justify-end pt-2 border-t border-slate-100">
                <button
                  onClick={() => setCurrentStep(2)}
                  className="px-4 py-2 bg-slate-800 hover:bg-slate-900 text-white text-xs font-semibold rounded flex items-center gap-1.5 transition-colors"
                >
                  <ArrowLeft className="w-4 h-4" />
                  Back to Recovery Options
                </button>
              </div>
            </div>
          )}

          {/* Cancellation state */}
          {workflowState === 'CANCELLED' && (
            <div className="bg-white border border-amber-200 rounded-md p-6 space-y-4 shadow-xs">
              <div className="flex items-start gap-3">
                <div className="p-2.5 rounded-full bg-amber-50 text-amber-600 shrink-0">
                  <AlertCircle className="w-6 h-6" />
                </div>
                <div className="space-y-1">
                  <h4 className="text-sm font-bold text-slate-900">
                    Recovery Cancelled
                  </h4>
                  <p className="text-xs text-slate-600">
                    The recovery operation was safely cancelled by the operator.
                  </p>
                </div>
              </div>

              <div className="flex justify-end pt-2 border-t border-slate-100">
                <button
                  onClick={() => setCurrentStep(2)}
                  className="px-4 py-2 bg-slate-800 hover:bg-slate-900 text-white text-xs font-semibold rounded flex items-center gap-1.5 transition-colors"
                >
                  <ArrowLeft className="w-4 h-4" />
                  Back to Recovery Options
                </button>
              </div>
            </div>
          )}

          {/* Completed results display */}
          {(workflowState === 'COMPLETED' || workflowState === 'RESULTS') && activeResult && (
            <RecoveryResults
              result={activeResult}
              onExport={exportFiles}
              onNewJob={handleReset}
            />
          )}
        </div>
      )}

      <ActiveCaseRequiredModal
        isOpen={showCaseModal}
        onClose={() => setShowCaseModal(false)}
        operationName="Forensic File Recovery"
      />
    </div>
  );
};
