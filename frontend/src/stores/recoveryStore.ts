import { create } from 'zustand';
import { AcquisitionArtifact } from '../types/acquisition';
import {
  RecoveryMode,
  RecoveryOptions,
  RecoveryPlan,
  RecoveryProgress,
  RecoveryResult,
  RecoverySourceSnapshot,
  RecoveryWorkflowState,
} from '../types/recovery';
import {
  cancelRecovery,
  createRecoveryPlan,
  exportRecoveredFiles,
  getRecoveryJob,
  getRecoveryProgress,
  listRecoveryJobs,
  listRecoverySources,
  startRecovery,
  validateRecoverySource,
} from '../services/recovery';

export function extractErrorMessage(err: unknown): string {
  if (!err) return 'An unknown error occurred';
  if (err instanceof Error) return err.message;
  if (typeof err === 'object') {
    if (
      'message' in err &&
      typeof (err as { message: unknown }).message === 'string' &&
      (err as { message: string }).message.trim().length > 0
    ) {
      return (err as { message: string }).message;
    }
    if ('code' in err && typeof (err as { code: unknown }).code === 'string') {
      return (err as { code: string }).code;
    }
  }
  return String(err);
}

export function formatRecoveryFailureReason(reason: unknown): string {
  if (!reason) return 'Unknown recovery failure';
  if (typeof reason === 'string') return reason;
  if (typeof reason === 'object') {
    if ('type' in reason) {
      const details = (reason as any).details;
      switch ((reason as any).type) {
        case 'filesystem_analysis_failed':
          return `Filesystem analysis failed: ${details || 'Unable to parse filesystem metadata'}`;
        case 'unsupported_filesystem':
          return `Unsupported filesystem: ${details || 'No recognized filesystem detected'}`;
        case 'corrupt_filesystem':
          return `Corrupted filesystem structure: ${details || 'Metadata corruption detected'}`;
        case 'image_not_found':
          return `Evidence image not found: ${details}`;
        case 'corrupt_image':
          return `Evidence image corruption detected: ${details}`;
        case 'read_error':
          return `Read I/O failure on evidence image: ${details}`;
        case 'cancelled':
          return 'Recovery operation was cancelled by operator';
        default:
          return details || (reason as any).type;
      }
    }
  }
  return String(reason);
}

const DEFAULT_FILE_TYPES = [
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
];

interface RecoveryStoreState {
  // Wizard Navigation
  currentStep: number;
  setCurrentStep: (step: number) => void;

  // Configuration & Options
  recoveryMode: RecoveryMode;
  setRecoveryMode: (mode: RecoveryMode) => void;
  outputDir: string;
  setOutputDir: (dir: string) => void;
  minConfidence: number;
  setMinConfidence: (score: number) => void;
  enableFragmentation: boolean;
  setEnableFragmentation: (enabled: boolean) => void;
  selectedTypes: string[];
  setSelectedTypes: (types: string[]) => void;
  toggleType: (id: string) => void;
  step2Error: string | null;
  setStep2Error: (err: string | null) => void;

  // Evidence Sources & Planning
  sources: RecoverySourceSnapshot[];
  loadingSources: boolean;
  selectedArtifact: AcquisitionArtifact | null;
  validatedSource: RecoverySourceSnapshot | null;
  selectedSourceSnapshot: RecoverySourceSnapshot | null;
  setSelectedSourceSnapshot: (src: RecoverySourceSnapshot | null) => void;
  plan: RecoveryPlan | null;
  isPlanning: boolean;

  // Active Execution Telemetry
  activeResult: RecoveryResult | null;
  progress: RecoveryProgress | null;
  isScanning: boolean;
  error: string | null;
  workflowState: RecoveryWorkflowState;
  activeOpId: string | null;

  // Historical Jobs
  jobs: RecoveryResult[];
  loadingJobs: boolean;

  // Actions
  fetchSources: () => Promise<void>;
  selectArtifact: (artifact: AcquisitionArtifact) => Promise<void>;
  preparePlan: (options: RecoveryOptions, sessionToken?: string | null) => Promise<RecoveryPlan>;
  runRecovery: (sessionToken?: string | null) => Promise<void>;
  cancelCurrent: (sessionToken?: string | null) => Promise<void>;
  exportFiles: (jobId: string, exportDir: string) => Promise<number>;
  fetchJobs: () => Promise<void>;
  resetWizard: () => void;
}

export const useRecoveryStore = create<RecoveryStoreState>((set, get) => ({
  currentStep: 1,
  setCurrentStep: (step) => set({ currentStep: step }),

  recoveryMode: 'all',
  setRecoveryMode: (recoveryMode) => set({ recoveryMode }),
  outputDir: 'C:\\RecoveredFiles',
  setOutputDir: (outputDir) => set({ outputDir }),
  minConfidence: 40,
  setMinConfidence: (minConfidence) => set({ minConfidence }),
  enableFragmentation: true,
  setEnableFragmentation: (enableFragmentation) => set({ enableFragmentation }),
  selectedTypes: [...DEFAULT_FILE_TYPES],
  setSelectedTypes: (selectedTypes) => set({ selectedTypes }),
  toggleType: (id) => {
    const { selectedTypes } = get();
    if (selectedTypes.includes(id)) {
      set({ selectedTypes: selectedTypes.filter((t) => t !== id) });
    } else {
      set({ selectedTypes: [...selectedTypes, id] });
    }
  },
  step2Error: null,
  setStep2Error: (step2Error) => set({ step2Error }),

  sources: [],
  loadingSources: false,
  selectedArtifact: null,
  validatedSource: null,
  selectedSourceSnapshot: null,
  setSelectedSourceSnapshot: (selectedSourceSnapshot) => set({ selectedSourceSnapshot }),
  plan: null,
  isPlanning: false,

  activeResult: null,
  progress: null,
  isScanning: false,
  error: null,
  workflowState: 'IDLE',
  activeOpId: null,

  jobs: [],
  loadingJobs: false,

  fetchSources: async () => {
    set({ loadingSources: true, error: null });
    try {
      const srcList = await listRecoverySources();
      set({ sources: srcList, loadingSources: false });
    } catch (err) {
      set({ error: extractErrorMessage(err), loadingSources: false });
    }
  },

  selectArtifact: async (artifact: AcquisitionArtifact) => {
    set({ selectedArtifact: artifact, error: null });
    try {
      const validated = await validateRecoverySource(artifact);
      set({ validatedSource: validated, workflowState: 'CONFIGURED' });
    } catch (err) {
      set({
        error: extractErrorMessage(err),
        validatedSource: null,
        workflowState: 'IDLE',
      });
    }
  },

  preparePlan: async (options: RecoveryOptions, sessionToken?: string | null) => {
    const { selectedArtifact } = get();
    if (!selectedArtifact) {
      const msg = 'Please select an evidence acquisition artifact first.';
      set({ error: msg });
      throw new Error(msg);
    }
    set({ isPlanning: true, error: null });
    try {
      const newPlan = await createRecoveryPlan({
        artifact: selectedArtifact,
        options,
        session_token: sessionToken,
      });
      set({ plan: newPlan, workflowState: 'CONFIGURED', isPlanning: false });
      return newPlan;
    } catch (err) {
      const msg = extractErrorMessage(err);
      set({ error: msg, isPlanning: false });
      throw new Error(msg);
    }
  },

  runRecovery: async (sessionToken?: string | null) => {
    const { plan } = get();
    if (!plan) {
      set({ error: 'No approved recovery plan ready for execution.' });
      return;
    }

    const opId = `op-rec-${Date.now()}`;
    set({
      isScanning: true,
      currentStep: 4,
      progress: null,
      error: null,
      activeResult: null,
      workflowState: 'STARTING',
      activeOpId: opId,
    });

    // Start background telemetry polling (runs independently of component lifecycle)
    const pollTimer = setInterval(async () => {
      const currentOp = get().activeOpId;
      if (!currentOp || !get().isScanning) {
        clearInterval(pollTimer);
        return;
      }
      try {
        const prog = await getRecoveryProgress(currentOp);
        if (prog) {
          set({ progress: prog });
          if (prog.phase) {
            const upper = prog.phase.toUpperCase();
            if (upper.includes('ANALYZ') || upper.includes('MFT') || upper.includes('DIRECTORY')) {
              set({ workflowState: 'ANALYZING' });
            } else if (upper.includes('RECOVER') || upper.includes('CARV')) {
              set({ workflowState: 'RECOVERING' });
            } else if (upper.includes('VALIDAT')) {
              set({ workflowState: 'VALIDATING' });
            }
          }
        }
      } catch {
        // ignore transient polling errors
      }
    }, 150);

    try {
      const res = await startRecovery({
        plan,
        session_token: sessionToken,
        operation_id: opId,
      });

      clearInterval(pollTimer);

      if (res.status === 'Completed') {
        const scopedJob = res.job_id ? await getRecoveryJob(res.job_id).catch(() => null) : null;
        const finalResult = scopedJob || res;
        set((state) => ({
          activeResult: finalResult,
          workflowState: 'COMPLETED',
          isScanning: false,
          activeOpId: null,
          jobs: [finalResult, ...state.jobs.filter((j) => j.job_id !== finalResult.job_id)],
        }));
      } else if (res.status === 'Cancelled') {
        set((state) => ({
          workflowState: 'CANCELLED',
          isScanning: false,
          activeOpId: null,
          error: 'Recovery operation was cancelled by operator.',
          jobs: [res, ...state.jobs.filter((j) => j.job_id !== res.job_id)],
        }));
      } else {
        const formattedErr = formatRecoveryFailureReason(res.failure_reason);
        set({
          workflowState: 'FAILED',
          isScanning: false,
          activeOpId: null,
          error: formattedErr,
        });
      }
    } catch (err) {
      clearInterval(pollTimer);
      set({
        workflowState: 'FAILED',
        isScanning: false,
        activeOpId: null,
        error: extractErrorMessage(err),
      });
    }
  },

  cancelCurrent: async (sessionToken?: string | null) => {
    const { activeOpId } = get();
    if (activeOpId) {
      try {
        await cancelRecovery(activeOpId, sessionToken);
        set({ workflowState: 'CANCELLED', isScanning: false });
      } catch (err) {
        set({ error: extractErrorMessage(err) });
      }
    }
  },

  exportFiles: async (jobId: string, exportDir: string) => {
    try {
      const count = await exportRecoveredFiles({ job_id: jobId, export_dir: exportDir });
      return count;
    } catch (err) {
      const msg = extractErrorMessage(err);
      set({ error: msg });
      throw new Error(msg);
    }
  },

  fetchJobs: async () => {
    set({ loadingJobs: true });
    try {
      const jobList = await listRecoveryJobs();
      set({ jobs: jobList, loadingJobs: false });
    } catch {
      set({ loadingJobs: false });
    }
  },

  resetWizard: () => {
    set({
      currentStep: 1,
      selectedArtifact: null,
      validatedSource: null,
      selectedSourceSnapshot: null,
      plan: null,
      activeResult: null,
      progress: null,
      error: null,
      step2Error: null,
      workflowState: 'IDLE',
      activeOpId: null,
      isScanning: false,
    });
  },
}));
