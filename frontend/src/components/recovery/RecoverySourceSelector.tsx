import React, { useState } from 'react';
import {
  HardDrive,
  ShieldCheck,
  CheckCircle2,
  FolderOpen,
  FileCheck,
  AlertCircle,
  Loader2,
  FileDigit,
  RefreshCw,
} from 'lucide-react';
import {
  Button,
  Input,
  Chip,
  Tabs,
  Tab,
} from '@heroui/react';
import { RecoverySourceSnapshot } from '../../types/recovery';
import { importLocalRecoverySource, pickRawImageFile } from '../../services/recovery';

interface RecoverySourceSelectorProps {
  sources: RecoverySourceSnapshot[];
  loading: boolean;
  selectedSourceId: string | null;
  onSelectSource: (source: RecoverySourceSnapshot) => void;
  onRefreshSources?: () => void;
  sessionToken?: string | null;
}

export const RecoverySourceSelector: React.FC<RecoverySourceSelectorProps> = ({
  sources,
  loading,
  selectedSourceId,
  onSelectSource,
  onRefreshSources,
  sessionToken,
}) => {
  const [sourceMode, setSourceMode] = useState<string>(
    sources.length > 0 ? 'library' : 'local'
  );
  const [localPath, setLocalPath] = useState('');
  const [isImporting, setIsImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  const [justImported, setJustImported] = useState<RecoverySourceSnapshot | null>(null);

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  };

  const handleBrowseFile = async () => {
    try {
      const selected = await pickRawImageFile();
      if (selected) {
        setLocalPath(selected);
        setImportError(null);
      }
    } catch (err: unknown) {
      console.error('File picker failed:', err);
    }
  };

  const handleImportAndSelect = async () => {
    const trimmed = localPath.trim();
    if (!trimmed) {
      setImportError('Please enter or browse to a raw disk image file path.');
      return;
    }

    setIsImporting(true);
    setImportError(null);
    try {
      const snapshot = await importLocalRecoverySource(trimmed, sessionToken || undefined);
      setJustImported(snapshot);
      onSelectSource(snapshot);
      if (onRefreshSources) {
        onRefreshSources();
      }
    } catch (err: unknown) {
      const msg =
        err && typeof err === 'object' && 'message' in err
          ? (err as { message: string }).message
          : err instanceof Error
          ? err.message
          : 'Failed to inspect and validate raw image file.';
      setImportError(msg);
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <div className="space-y-4">
      {/* Tab Switcher: Local Computer File vs. Known Image Library */}
      <Tabs
        selectedKey={sourceMode}
        onSelectionChange={(key) => setSourceMode(String(key))}
        variant="underlined"
        classNames={{
          tabList: 'border-b border-slate-200 dark:border-workstation-border gap-6 p-0',
          cursor: 'bg-slate-900 dark:bg-slate-100',
          tab: 'h-10 text-xs font-semibold px-0 data-[selected=true]:text-slate-900 dark:data-[selected=true]:text-slate-100 text-slate-500',
        }}
      >
        <Tab
          key="local"
          title={
            <div className="flex items-center gap-2">
              <FolderOpen className="w-4 h-4" />
              <span>Local Image File (.raw, .dd, .img, .bin, .iso)</span>
            </div>
          }
        />
        <Tab
          key="library"
          title={
            <div className="flex items-center gap-2">
              <HardDrive className="w-4 h-4" />
              <span>Acquired Images Library</span>
              <Chip size="sm" variant="flat" className="font-mono text-[10px] h-5 bg-slate-100 dark:bg-workstation-surface text-slate-700 dark:text-slate-300">
                {sources.length}
              </Chip>
            </div>
          }
        />
      </Tabs>

      {/* Mode 1: Local Computer File Input / Native Browse (Flat Form) */}
      {sourceMode === 'local' && (
        <div className="space-y-4 pt-1">
          <div className="space-y-1">
            <h3 className="text-xs font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2 uppercase tracking-wider">
              <FileDigit className="w-4 h-4 text-slate-700 dark:text-slate-300" />
              Select Forensic Disk Image from Local Computer
            </h3>
            <p className="text-[11px] text-slate-500 dark:text-slate-400">
              Provide a raw bitstream disk image (<code className="text-slate-800 dark:text-slate-200 font-mono font-semibold">.raw</code>,{' '}
              <code className="text-slate-800 dark:text-slate-200 font-mono font-semibold">.dd</code>,{' '}
              <code className="text-slate-800 dark:text-slate-200 font-mono font-semibold">.img</code>,{' '}
              <code className="text-slate-800 dark:text-slate-200 font-mono font-semibold">.bin</code>,{' '}
              <code className="text-slate-800 dark:text-slate-200 font-mono font-semibold">.iso</code>, or{' '}
              <code className="text-slate-800 dark:text-slate-200 font-mono font-semibold">.001</code>). LocardX will compute its streaming
              SHA-256 digest in read-only mode to anchor forensic chain-of-custody before carving.
            </p>
          </div>

          <div className="space-y-2">
            <label className="block text-[11px] font-semibold text-slate-700 dark:text-slate-300">
              Image File Path on Local System
            </label>
            <div className="flex gap-2">
              <Input
                size="sm"
                value={localPath}
                onValueChange={(val) => {
                  setLocalPath(val);
                  setImportError(null);
                }}
                placeholder="e.g. C:\Users\evidence.raw or /path/to/disk.dd"
                isDisabled={isImporting}
                className="flex-1 font-mono text-xs"
                classNames={{
                  inputWrapper: 'bg-white dark:bg-[#151C24] border border-slate-300 dark:border-[#26313D] h-8',
                }}
              />
              <Button
                size="sm"
                variant="flat"
                onPress={handleBrowseFile}
                isDisabled={isImporting}
                startContent={<FolderOpen className="w-3.5 h-3.5 text-slate-600 dark:text-slate-400" />}
                className="bg-slate-100 dark:bg-[#1B2530] border border-slate-300 dark:border-[#26313D] text-slate-700 dark:text-slate-300 text-xs font-semibold px-3.5 shrink-0 h-8"
              >
                Browse File...
              </Button>
              <Button
                size="sm"
                onPress={handleImportAndSelect}
                isDisabled={isImporting || !localPath.trim()}
                isLoading={isImporting}
                startContent={!isImporting && <FileCheck className="w-3.5 h-3.5" />}
                className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 text-xs font-semibold px-4 shrink-0 shadow-xs h-8"
              >
                {isImporting ? 'Verifying...' : 'Load & Verify Image'}
              </Button>
            </div>
          </div>

          {isImporting && (
            <div className="p-3 bg-slate-100 dark:bg-[#1B2530] border border-slate-300 dark:border-[#26313D] rounded-xs flex items-center gap-2.5 text-xs text-slate-800 dark:text-slate-200">
              <Loader2 className="w-4 h-4 animate-spin text-slate-700 dark:text-slate-300 shrink-0" />
              <span>Streaming cryptographic SHA-256 over image bitstream in read-only mode...</span>
            </div>
          )}

          {importError && (
            <div className="p-3 bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 rounded-xs flex items-start gap-2.5 text-xs text-rose-800 dark:text-rose-300">
              <AlertCircle className="w-4 h-4 text-rose-600 shrink-0 mt-0.5" />
              <div className="space-y-0.5">
                <span className="font-semibold block">Failed to load evidence image</span>
                <span>{importError}</span>
              </div>
            </div>
          )}

          {justImported && (
            <div className="p-3.5 bg-slate-100 dark:bg-[#1B2530] border border-slate-300 dark:border-[#26313D] rounded-xs space-y-1.5">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 text-xs font-bold text-slate-900 dark:text-slate-100">
                  <CheckCircle2 className="w-4 h-4 text-slate-700 dark:text-slate-300" />
                  <span>Evidence Image Verified & Selected</span>
                </div>
                <Chip size="sm" variant="flat" className="text-[10px] uppercase font-bold tracking-wider bg-slate-200 dark:bg-[#151C24] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-[#26313D]">
                  Ready for Carving
                </Chip>
              </div>
              <div className="text-[11px] text-slate-800 dark:text-slate-200 font-mono space-y-0.5">
                <div>Path: <strong className="break-all">{justImported.image_path}</strong></div>
                <div>Size: <strong>{formatBytes(justImported.image_size_bytes)}</strong> ({justImported.image_size_bytes} bytes)</div>
                <div className="text-[10px] text-slate-600 dark:text-slate-400 truncate">SHA-256: {justImported.image_sha256}</div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Mode 2: Existing Sources in Library (Structured List) */}
      {sourceMode === 'library' && (
        <div className="space-y-3 pt-1">
          <div className="flex items-center justify-between">
            <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300">
              Verified Evidence Images in Database
            </label>
            {onRefreshSources && (
              <Button
                size="sm"
                variant="light"
                onPress={onRefreshSources}
                isDisabled={loading}
                startContent={<RefreshCw className={`w-3 h-3 ${loading ? 'animate-spin' : ''}`} />}
                className="text-[11px] text-slate-700 dark:text-slate-300 font-medium h-7"
              >
                Refresh
              </Button>
            )}
          </div>

          {loading ? (
            <div className="p-8 text-center text-slate-400 text-xs">
              Loading verified acquisition sources...
            </div>
          ) : sources.length === 0 ? (
            <div className="p-8 text-center space-y-2 border border-dashed border-slate-300 dark:border-[#26313D] rounded-sm">
              <HardDrive className="w-8 h-8 text-slate-400 mx-auto" />
              <p className="text-xs font-medium text-slate-700 dark:text-slate-300">No Acquired Images in Database</p>
              <p className="text-[11px] text-slate-500 dark:text-slate-400 max-w-md mx-auto">
                No images were found in the database. Use the{' '}
                <button
                  type="button"
                  onClick={() => setSourceMode('local')}
                  className="text-slate-900 dark:text-slate-100 font-semibold hover:underline cursor-pointer"
                >
                  Local Image File
                </button>{' '}
                tab to pick any raw file (.raw, .dd, .img) from your computer.
              </p>
            </div>
          ) : (
            <div className="border border-slate-200 dark:border-[#26313D] rounded-sm divide-y divide-slate-200 dark:divide-[#26313D] bg-white dark:bg-[#151C24]">
              {sources.map((src) => {
                const isSelected = selectedSourceId === src.source_id;
                return (
                  <div
                    key={src.source_id}
                    onClick={() => onSelectSource(src)}
                    className={`p-3.5 flex items-start justify-between cursor-pointer transition-colors ${
                      isSelected
                        ? 'bg-slate-100 dark:bg-[#1B2530]'
                        : 'hover:bg-slate-50 dark:hover:bg-[#1B2530]/50'
                    }`}
                  >
                    <div className="flex items-start space-x-3">
                      <div
                        className={`p-1.5 rounded-xs mt-0.5 ${
                          isSelected
                            ? 'bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900'
                            : 'bg-slate-100 text-slate-600 dark:bg-[#10161D] dark:text-slate-400'
                        }`}
                      >
                        <HardDrive className="w-4 h-4" />
                      </div>
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="text-xs font-semibold text-slate-900 dark:text-slate-100 font-mono">
                            {src.image_path}
                          </span>
                          {src.is_trusted && (
                            <Chip
                              size="sm"
                              variant="flat"
                              startContent={<ShieldCheck className="w-3 h-3 text-slate-700 dark:text-slate-300" />}
                              className="text-[10px] font-medium bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-[#26313D]"
                            >
                              Hash Verified
                            </Chip>
                          )}
                        </div>
                        <div className="text-[11px] text-slate-500 dark:text-slate-400 mt-1 flex flex-wrap gap-x-4 gap-y-1">
                          <span>
                            Source: <strong className="font-mono text-slate-700 dark:text-slate-300">{src.original_device_id}</strong>
                          </span>
                          <span>
                            Size: <strong className="text-slate-700 dark:text-slate-300">{formatBytes(src.image_size_bytes)}</strong>
                          </span>
                          {src.original_serial && (
                            <span>
                              Serial: <strong className="font-mono text-slate-700 dark:text-slate-300">{src.original_serial}</strong>
                            </span>
                          )}
                        </div>
                        <div className="text-[10px] text-slate-400 font-mono mt-1 truncate max-w-xl">
                          SHA-256: {src.image_sha256}
                        </div>
                      </div>
                    </div>
                    {isSelected && (
                      <CheckCircle2 className="w-5 h-5 text-slate-900 dark:text-slate-100 shrink-0" />
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
