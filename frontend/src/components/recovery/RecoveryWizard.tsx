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
  Briefcase,
} from 'lucide-react';
import {
  Button,
  Input,
  Checkbox,
  RadioGroup,
  Radio,
  Slider,
  Chip,
  Divider,
} from '@heroui/react';
import { useRecoveryStore } from '../../stores/recoveryStore';
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
    fetchSources,
    selectedArtifact,
    selectedSourceSnapshot,
    setSelectedSourceSnapshot,
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
    resetWizard: resetState,
    currentStep,
    setCurrentStep,
    step2Error,
    setStep2Error,
    recoveryMode,
    setRecoveryMode,
    outputDir,
    setOutputDir,
    minConfidence,
    setMinConfidence,
    enableFragmentation,
    setEnableFragmentation,
    selectedTypes,
    setSelectedTypes,
    toggleType,
  } = useRecoveryStore();

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
    toggleType(id);
  };

  const handleSelectSource = (src: RecoverySourceSnapshot) => {
    setSelectedSourceSnapshot(src);
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
        <div className="border-l-2 border-amber-500 bg-amber-500/10 p-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs">
          <div className="flex items-center gap-3">
            <Briefcase className="w-5 h-5 text-amber-600 dark:text-amber-400 shrink-0" />
            <div>
              <h4 className="text-xs font-bold text-amber-900 dark:text-amber-200 uppercase tracking-wider">
                Active Investigation Case Required
              </h4>
              <p className="text-xs text-amber-800 dark:text-amber-300">
                Forensic file recovery and evidential carving must be bound to an open investigation case to record evidentiary chain of custody.
              </p>
            </div>
          </div>
          <Button
            size="sm"
            onPress={() => setShowCaseModal(true)}
            className="px-3 py-1.5 text-xs font-semibold text-amber-900 dark:text-amber-100 bg-amber-200 hover:bg-amber-300 dark:bg-amber-900/60 dark:hover:bg-amber-900/80 border border-amber-400 dark:border-amber-700 rounded-sm shrink-0"
          >
            Select or Create Case
          </Button>
        </div>
      )}

      {/* Stepper Header */}
      <div className="flex items-center justify-between border-b border-slate-200 dark:border-workstation-border pb-3">
        <div className="flex items-center space-x-6 text-xs">
          {[
            { step: 1, label: 'Source Image' },
            { step: 2, label: 'Carving Options' },
            { step: 3, label: 'Pre-Flight Review' },
            { step: 4, label: 'Execution & Results' },
          ].map((item) => (
            <span
              key={item.step}
              className={`font-semibold flex items-center gap-1.5 ${
                currentStep === item.step
                  ? 'text-slate-900 dark:text-slate-100 font-bold'
                  : 'text-slate-400 dark:text-slate-500'
              }`}
            >
              <span
                className={`w-5 h-5 rounded-full border flex items-center justify-center text-[10px] ${
                  currentStep === item.step
                    ? 'border-slate-900 dark:border-slate-100 bg-slate-900 dark:bg-slate-100 text-white dark:text-slate-900'
                    : 'border-slate-300 dark:border-slate-700'
                }`}
              >
                {item.step}
              </span>
              {item.label}
            </span>
          ))}
        </div>

        {currentStep > 1 && !isScanning && (
          <Button
            size="sm"
            variant="light"
            onPress={handleReset}
            startContent={<RotateCcw className="w-3.5 h-3.5" />}
            className="text-xs text-slate-500 hover:text-slate-700 dark:hover:text-slate-300"
          >
            Reset
          </Button>
        )}
      </div>

      {error && (
        <div className="p-3 bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 rounded-md text-xs text-rose-700 dark:text-rose-400 flex items-center gap-2">
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
            onRefreshSources={fetchSources}
            sessionToken={sessionToken}
          />

          <div className="flex justify-end pt-2">
            <Button
              onPress={() => setCurrentStep(2)}
              isDisabled={!selectedArtifact}
              endContent={<ArrowRight className="w-4 h-4" />}
              className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 font-semibold text-xs px-4"
            >
              Continue to Options
            </Button>
          </div>
        </div>
      )}

      {/* Step 2: Configure Options */}
      {currentStep === 2 && (
        <div className="space-y-5">
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {/* Left Column: Mode & Output */}
            <div className="border border-slate-200 dark:border-[#26313D] p-4 rounded-xs bg-white dark:bg-[#151C24] space-y-4">
              <h4 className="text-xs font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5 uppercase tracking-wider">
                  <Sliders className="w-4 h-4 text-slate-700 dark:text-slate-300" />
                  Recovery Strategy & Modes
                </h4>

                <div className="space-y-2">
                  <label className="block text-xs font-medium text-slate-700 dark:text-slate-300">Recovery Mode</label>
                  <RadioGroup
                    value={recoveryMode}
                    onValueChange={(val) => setRecoveryMode(val as RecoveryMode)}
                    classNames={{
                      wrapper: 'space-y-2',
                    }}
                  >
                    {[
                      { key: 'all', title: 'All (Filesystem Metadata + Sliding-Window Carving)' },
                      { key: 'filesystem_only', title: 'Filesystem Metadata Only (NTFS MFT, FAT32 deleted entries)' },
                      { key: 'carving_only', title: 'Raw Bitstream Carving Only (Structure & Signature search)' },
                    ].map((m) => (
                      <Radio
                        key={m.key}
                        value={m.key}
                        classNames={{
                          base: `p-2.5 rounded-lg border w-full max-w-full m-0 cursor-pointer transition-all ${
                            recoveryMode === m.key
                              ? 'border-slate-900 dark:border-slate-400 bg-slate-50 dark:bg-workstation-surface'
                              : 'border-slate-200 dark:border-workstation-border hover:bg-slate-50 dark:hover:bg-workstation-surface/50'
                          }`,
                          label: 'text-xs text-slate-800 dark:text-slate-200',
                        }}
                      >
                        {m.title}
                      </Radio>
                    ))}
                  </RadioGroup>
                </div>

                <Divider />

                <div className="space-y-1">
                  <label className="block text-xs font-medium text-slate-700 dark:text-slate-300">Output Export Directory</label>
                  <Input
                    size="sm"
                    value={outputDir}
                    onValueChange={setOutputDir}
                    placeholder="C:\RecoveredFiles"
                    className="font-mono text-xs"
                    classNames={{
                      inputWrapper: 'bg-white dark:bg-workstation-panel border border-slate-300 dark:border-workstation-border',
                    }}
                  />
                  <p className="text-[10px] text-slate-400">
                    Recovered files will be written here strictly isolated from the source image.
                  </p>
                </div>

                <Divider />

                <div className="space-y-2">
                  <div className="flex justify-between text-xs font-medium text-slate-700 dark:text-slate-300">
                    <span>Minimum Confidence Score Threshold</span>
                    <Chip size="sm" variant="flat" className="font-mono font-bold bg-slate-100 dark:bg-workstation-surface text-slate-900 dark:text-slate-100">
                      {minConfidence}%
                    </Chip>
                  </div>
                  <Slider
                    size="sm"
                    step={5}
                    maxValue={95}
                    minValue={0}
                    value={minConfidence}
                    onChange={(val) => setMinConfidence(Number(val))}
                    aria-label="Minimum confidence score"
                    classNames={{
                      filler: 'bg-slate-900 dark:bg-slate-100',
                      thumb: 'bg-slate-900 dark:bg-slate-100',
                    }}
                  />
                  <div className="flex justify-between text-[10px] text-slate-400">
                    <span>Permissive (0%)</span>
                    <span>Standard (40%)</span>
                    <span>Strict High Only (85%)</span>
                  </div>
                </div>
            </div>

            {/* Right Column: File Types */}
            <div className="border border-slate-200 dark:border-[#26313D] p-4 rounded-xs bg-white dark:bg-[#151C24] space-y-4">
                <div className="flex items-center justify-between">
                  <h4 className="text-xs font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5 uppercase tracking-wider">
                    <FileCheck className="w-4 h-4 text-slate-700 dark:text-slate-300" />
                    Target File Types ({selectedTypes.length}/{fileTypeOptions.length})
                  </h4>
                  <div className="flex items-center gap-2">
                    <Button
                      size="sm"
                      variant="light"
                      onPress={() => setSelectedTypes(fileTypeOptions.map((o) => o.id))}
                      className="text-[10px] text-slate-700 dark:text-slate-300 font-medium h-6 px-2"
                    >
                      Select All
                    </Button>
                    <span className="text-slate-300 dark:text-slate-600">|</span>
                    <Button
                      size="sm"
                      variant="light"
                      onPress={() => setSelectedTypes([])}
                      className="text-[10px] text-slate-500 dark:text-slate-400 h-6 px-2"
                    >
                      Clear
                    </Button>
                  </div>
                </div>
                <div className="space-y-1.5 max-h-72 overflow-y-auto pr-1">
                  {fileTypeOptions.map((opt) => (
                    <div
                      key={opt.id}
                      onClick={() => handleToggleType(opt.id)}
                      className="flex items-center gap-2.5 p-1.5 rounded hover:bg-slate-50 dark:hover:bg-workstation-surface/60 cursor-pointer text-xs text-slate-700 dark:text-slate-300 transition-colors"
                    >
                      <Checkbox
                        size="sm"
                        isSelected={selectedTypes.includes(opt.id)}
                        onValueChange={() => handleToggleType(opt.id)}
                      />
                      <span>{opt.label}</span>
                    </div>
                  ))}
                </div>

                <Divider />

                <div className="space-y-1">
                  <Checkbox
                    size="sm"
                    isSelected={enableFragmentation}
                    onValueChange={setEnableFragmentation}
                    classNames={{
                      label: 'text-xs text-slate-700 dark:text-slate-300',
                    }}
                  >
                    Enable cluster-aligned fragment reconstruction
                  </Checkbox>
                  <p className="text-[10px] text-slate-400 pl-6">
                    Evaluates discontinuities without hallucinating or fabricating missing data.
                  </p>
                </div>
            </div>
          </div>

          {(step2Error || error) && (
            <div className="p-3 bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 rounded-md text-xs text-rose-700 dark:text-rose-400 flex items-center gap-2">
              <AlertCircle className="w-4 h-4 shrink-0 text-rose-600" />
              <span>{step2Error || error}</span>
            </div>
          )}

          <div className="flex justify-between pt-2">
            <Button
              variant="flat"
              onPress={() => {
                setStep2Error(null);
                setCurrentStep(1);
              }}
              startContent={<ArrowLeft className="w-4 h-4" />}
              className="text-xs font-medium border border-slate-200 dark:border-workstation-border bg-white dark:bg-workstation-panel text-slate-700 dark:text-slate-300"
            >
              Back
            </Button>
            <Button
              onPress={handleGeneratePlan}
              isDisabled={isPlanning}
              isLoading={isPlanning}
              endContent={!isPlanning && <ArrowRight className="w-4 h-4" />}
              className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 text-xs font-semibold px-4"
            >
              {isPlanning ? 'Generating Plan...' : 'Review Pre-Flight Plan'}
            </Button>
          </div>
        </div>
      )}

      {/* Step 3: Pre-Flight Review */}
      {currentStep === 3 && (
        !plan ? (
          <div className="border border-amber-500/30 bg-amber-500/10 p-6 text-center space-y-3 rounded-xs">
            <AlertCircle className="w-8 h-8 text-amber-600 dark:text-amber-400 mx-auto" />
            <p className="text-xs text-amber-800 dark:text-amber-300 font-medium">
              No active recovery plan found. Please return to options to configure and generate a pre-flight plan.
            </p>
            <Button
              size="sm"
              onPress={() => setCurrentStep(2)}
              startContent={<ArrowLeft className="w-3.5 h-3.5" />}
              className="bg-amber-600 hover:bg-amber-700 text-white text-xs font-semibold"
            >
              Back to Carving Options
            </Button>
          </div>
        ) : (
          <div className="space-y-4">
            <div className="border border-slate-200 dark:border-[#26313D] p-5 space-y-4 rounded-xs bg-white dark:bg-[#151C24]">
              <div className="flex items-center gap-2 border-b border-slate-100 dark:border-[#26313D] pb-3">
                <ShieldCheck className="w-5 h-5 text-slate-700 dark:text-slate-300" />
                <div>
                  <h4 className="text-xs font-semibold text-slate-800 dark:text-slate-200 uppercase tracking-wider">
                    Pre-Flight Evidential Safety Verification
                  </h4>
                  <p className="text-[11px] text-slate-500 dark:text-slate-400">
                    Plan generated and cryptographically cross-verified against forensic source
                  </p>
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs">
                <div className="space-y-2">
                  <div>
                    <span className="text-slate-400 text-[11px] block">Source Image:</span>
                    <span className="font-mono text-slate-800 dark:text-slate-200 font-semibold">{plan.source_image_path}</span>
                  </div>
                  <div>
                    <span className="text-slate-400 text-[11px] block">Verified Image SHA-256:</span>
                    <span className="font-mono text-[11px] text-slate-700 dark:text-slate-300 break-all bg-slate-50 dark:bg-workstation-surface p-1.5 rounded border border-slate-100 dark:border-[#26313D] block">
                      {plan.source_image_sha256}
                    </span>
                  </div>
                  <div>
                    <span className="text-slate-400 text-[11px] block">Recovery Mode:</span>
                    <span className="text-slate-800 dark:text-slate-200 font-semibold">{plan.options.recovery_mode.toUpperCase()}</span>
                  </div>
                </div>

                <div className="space-y-2">
                  <div>
                    <span className="text-slate-400 text-[11px] block">Output Directory:</span>
                    <span className="font-mono text-slate-800 dark:text-slate-200 font-semibold">{plan.options.output_directory}</span>
                  </div>
                  <div>
                    <span className="text-slate-400 text-[11px] block">Target Types:</span>
                    <span className="text-slate-700 dark:text-slate-300">
                      {plan.options.target_file_types ? plan.options.target_file_types.join(', ') : 'All registered types'}
                    </span>
                  </div>
                  <div>
                    <span className="text-slate-400 text-[11px] block">Minimum Confidence Score:</span>
                    <span className="text-slate-900 dark:text-slate-100 font-bold">{plan.options.min_confidence_score}%</span>
                  </div>
                </div>
              </div>

              <div className="p-3 bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-[#26313D] rounded text-xs text-slate-900 dark:text-slate-100 space-y-1">
                <div className="font-semibold flex items-center gap-1.5">
                  <CheckCircle2 className="w-3.5 h-3.5 text-slate-700 dark:text-slate-300" />
                  Evidential Guarantee: Strictly Non-Destructive
                </div>
                <p className="text-[11px] text-slate-600 dark:text-slate-400">
                  The evidence image is accessed strictly read-only with streaming SHA-256 hash checks.
                  No physical devices or source image sectors will be modified. Recovered files are written exclusively into the designated output folder.
                </p>
              </div>
            </div>

            <div className="flex justify-between pt-2">
              <Button
                variant="flat"
                onPress={() => setCurrentStep(2)}
                startContent={<ArrowLeft className="w-4 h-4" />}
                className="text-xs font-medium border border-slate-200 dark:border-[#26313D] bg-white dark:bg-[#151C24] text-slate-700 dark:text-slate-300"
              >
                Back
              </Button>
              <Button
                onPress={handleStartExecution}
                startContent={<Play className="w-4 h-4 fill-current" />}
                className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 text-xs font-semibold px-5 shadow-xs"
              >
                Start Forensic Recovery
              </Button>
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
            <div className="border border-rose-200 dark:border-rose-900/40 p-5 space-y-4 rounded-xs bg-white dark:bg-[#151C24]">
              <div className="flex items-start gap-3">
                <div className="p-2.5 rounded-full bg-rose-50 dark:bg-rose-950/40 text-rose-600 dark:text-rose-400 shrink-0">
                  <AlertCircle className="w-6 h-6" />
                </div>
                <div className="space-y-1">
                  <h4 className="text-sm font-bold text-slate-900 dark:text-slate-100">
                    Filesystem Recovery Failed
                  </h4>
                  <p className="text-xs text-slate-600 dark:text-slate-400">
                    The filesystem analysis engine encountered an unrecoverable condition and halted to preserve evidentiary integrity.
                  </p>
                  <div className="mt-2 p-3 bg-rose-50 dark:bg-rose-950/30 border border-rose-100 dark:border-rose-900/30 rounded text-xs font-mono text-rose-800 dark:text-rose-300 break-words">
                    Reason: {error || 'An unexpected error halted filesystem recovery.'}
                  </div>
                </div>
              </div>

              <div className="flex justify-end pt-2 border-t border-slate-100 dark:border-[#26313D]">
                <Button
                  size="sm"
                  onPress={() => setCurrentStep(2)}
                  startContent={<ArrowLeft className="w-4 h-4" />}
                  className="bg-slate-800 hover:bg-slate-900 text-white dark:bg-slate-200 dark:text-slate-900 text-xs font-semibold px-4"
                >
                  Back to Recovery Options
                </Button>
              </div>
            </div>
          )}

          {/* Cancellation state */}
          {workflowState === 'CANCELLED' && (
            <div className="border border-amber-200 dark:border-amber-900/40 p-5 space-y-4 rounded-xs bg-white dark:bg-[#151C24]">
              <div className="flex items-start gap-3">
                <div className="p-2.5 rounded-full bg-amber-50 dark:bg-amber-950/40 text-amber-600 dark:text-amber-400 shrink-0">
                  <AlertCircle className="w-6 h-6" />
                </div>
                <div className="space-y-1">
                  <h4 className="text-sm font-bold text-slate-900 dark:text-slate-100">
                    Recovery Cancelled
                  </h4>
                  <p className="text-xs text-slate-600 dark:text-slate-400">
                    The recovery operation was safely cancelled by the operator.
                  </p>
                </div>
              </div>

              <div className="flex justify-end pt-2 border-t border-slate-100 dark:border-[#26313D]">
                <Button
                  size="sm"
                  onPress={() => setCurrentStep(2)}
                  startContent={<ArrowLeft className="w-4 h-4" />}
                  className="bg-slate-800 hover:bg-slate-900 text-white dark:bg-slate-200 dark:text-slate-900 text-xs font-semibold px-4"
                >
                  Back to Recovery Options
                </Button>
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
