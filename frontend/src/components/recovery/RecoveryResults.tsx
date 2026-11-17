import React, { useState, useMemo, useEffect } from 'react';
import {
  Download,
  Search,
  CheckCircle2,
  FileText,
  ShieldCheck,
  FolderDown,
  FolderOpen,
  ExternalLink,
  Image as ImageIcon,
  Archive,
  Database,
  FileCode,
  File,
  Film,
} from 'lucide-react';
import {
  Button,
  Input,
  Chip,
  Table,
  TableHeader,
  TableColumn,
  TableBody,
  TableRow,
  TableCell,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  Divider,
} from '@heroui/react';
import { RecoveryReport, RecoveryResult, RecoveredFile } from '../../types/recovery';
import { getRecoveryReport, openRecoveredFile, revealRecoveredFile } from '../../services/recovery';

interface RecoveryResultsProps {
  result: RecoveryResult;
  onExport: (jobId: string, exportDir: string) => Promise<number>;
  onNewJob: () => void;
}

export const RecoveryResults: React.FC<RecoveryResultsProps> = ({
  result,
  onExport,
  onNewJob,
}) => {
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [exportDir, setExportDir] = useState<string>('C:\\RecoveredFiles');
  const [isExporting, setIsExporting] = useState<boolean>(false);
  const [exportedCount, setExportedCount] = useState<number | null>(null);
  const [report, setReport] = useState<RecoveryReport | null>(null);
  const [showReportModal, setShowReportModal] = useState<boolean>(false);
  const [exportError, setExportError] = useState<string | null>(null);

  // Selected file for contextual detail pane
  const [selectedFileId, setSelectedFileId] = useState<string | null>(
    result.recovered_files.length > 0 ? result.recovered_files[0].file_id : null
  );

  const [actionState, setActionState] = useState<{
    loading: boolean;
    type?: 'open' | 'reveal';
    error?: string | null;
  }>({ loading: false });

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  };

  const categories = [
    { key: 'all', label: 'All Artifacts' },
    { key: 'images', label: 'Images' },
    { key: 'documents', label: 'Documents' },
    { key: 'archives', label: 'Archives' },
    { key: 'media', label: 'Media' },
    { key: 'databases', label: 'Databases' },
    { key: 'text_files', label: 'Text / Code' },
  ];

  const filteredFiles = result.recovered_files.filter((file) => {
    const matchesCategory =
      selectedCategory === 'all' ||
      file.category.toLowerCase() === selectedCategory.toLowerCase();
    const matchesSearch =
      searchQuery.trim() === '' ||
      file.suggested_filename.toLowerCase().includes(searchQuery.toLowerCase()) ||
      file.file_type.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  const [currentPage, setCurrentPage] = useState(1);
  const pageSize = 50;

  // Reset to page 1 whenever category or search filter changes
  useEffect(() => {
    setCurrentPage(1);
  }, [selectedCategory, searchQuery]);

  const totalPages = Math.max(1, Math.ceil(filteredFiles.length / pageSize));
  const safePage = Math.min(currentPage, totalPages);

  const paginatedFiles = useMemo(() => {
    const start = (safePage - 1) * pageSize;
    return filteredFiles.slice(start, start + pageSize);
  }, [filteredFiles, safePage, pageSize]);

  const selectedFile: RecoveredFile | undefined =
    result.recovered_files.find((f) => f.file_id === selectedFileId) || filteredFiles[0];

  const avgConfidence =
    result.recovered_files.length > 0
      ? Math.round(
          result.recovered_files.reduce((acc, f) => acc + f.confidence_score, 0) /
            result.recovered_files.length
        )
      : 0;

  const handleExport = async () => {
    if (!exportDir.trim()) return;
    setIsExporting(true);
    setExportError(null);
    try {
      const count = await onExport(result.job_id, exportDir);
      setExportedCount(count);
    } catch (err) {
      setExportError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsExporting(false);
    }
  };

  const handleViewReport = async () => {
    try {
      const rep = await getRecoveryReport(result.job_id);
      if (rep) {
        setReport(rep);
        setShowReportModal(true);
      }
    } catch {
      // ignore
    }
  };

  const handleOpenFile = async (file: RecoveredFile) => {
    if (!file.output_relative_path) return;
    setActionState({ loading: true, type: 'open' });
    try {
      await openRecoveredFile(file.job_id, file.file_id);
      setActionState({ loading: false });
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Failed to launch file.';
      setActionState({ loading: false, error: msg });
    }
  };

  const handleRevealFile = async (file: RecoveredFile) => {
    if (!file.output_relative_path) return;
    setActionState({ loading: true, type: 'reveal' });
    try {
      await revealRecoveredFile(file.job_id, file.file_id);
      setActionState({ loading: false });
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Failed to reveal file in folder.';
      setActionState({ loading: false, error: msg });
    }
  };

  const getCategoryIcon = (cat: string) => {
    switch (cat.toLowerCase()) {
      case 'images':
        return <ImageIcon className="w-3.5 h-3.5 text-slate-500" />;
      case 'documents':
        return <FileText className="w-3.5 h-3.5 text-slate-500" />;
      case 'archives':
        return <Archive className="w-3.5 h-3.5 text-slate-500" />;
      case 'media':
        return <Film className="w-3.5 h-3.5 text-slate-500" />;
      case 'databases':
        return <Database className="w-3.5 h-3.5 text-slate-500" />;
      case 'text_files':
        return <FileCode className="w-3.5 h-3.5 text-slate-500" />;
      default:
        return <File className="w-3.5 h-3.5 text-slate-500" />;
    }
  };

  return (
    <div className="space-y-4">
      {/* 1. Header Telemetry Ribbon (No Cards) */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 py-3 border-y border-slate-200 dark:border-[#26313D] text-xs">
        <div>
          <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
            Files Recovered
          </span>
          <span className="text-xl font-bold font-mono text-slate-900 dark:text-slate-100 mt-0.5 block">
            {result.files_recovered}
          </span>
          <span className="text-[11px] text-slate-500">Extracted evidence files</span>
        </div>

        <div className="md:border-l md:border-slate-200 md:dark:border-[#26313D] md:pl-4">
          <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
            Candidates Evaluated
          </span>
          <span className="text-xl font-bold font-mono text-slate-900 dark:text-slate-100 mt-0.5 block">
            {result.candidates_evaluated}
          </span>
          <span className="text-[11px] text-slate-500">Heuristic signature hits</span>
        </div>

        <div className="md:border-l md:border-slate-200 md:dark:border-[#26313D] md:pl-4">
          <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
            Mean Carving Confidence
          </span>
          <span className="text-xl font-bold font-mono text-slate-900 dark:text-slate-100 mt-0.5 block">
            {avgConfidence}%
          </span>
          <span className="text-[11px] text-slate-500">Multi-point validation score</span>
        </div>

        <div className="md:border-l md:border-slate-200 md:dark:border-[#26313D] md:pl-4">
          <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
            Carving Duration
          </span>
          <span className="text-xl font-bold font-mono text-slate-900 dark:text-slate-100 mt-0.5 block">
            {result.elapsed_seconds.toFixed(2)}s
          </span>
          <span className="text-[11px] text-slate-500">
            {result.bytes_scanned > 0 ? `${formatBytes(result.bytes_scanned)} scanned` : 'Complete'}
          </span>
        </div>
      </div>

      {/* 2. Export Destination & Action Strip */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 p-3 bg-slate-50 dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] rounded-sm text-xs">
        <div className="flex items-center gap-2 flex-1 w-full sm:w-auto">
          <FolderDown className="w-4 h-4 text-slate-600 dark:text-slate-400 shrink-0" />
          <span className="font-semibold text-slate-700 dark:text-slate-300 shrink-0">Export Target:</span>
          <Input
            size="sm"
            value={exportDir}
            onValueChange={setExportDir}
            placeholder="Destination directory path..."
            aria-label="Export Destination"
            className="flex-1 font-mono text-xs max-w-md"
            classNames={{
              inputWrapper: 'bg-white dark:bg-[#1B2530] border border-slate-300 dark:border-[#26313D] h-7',
            }}
          />
          <Button
            size="sm"
            onPress={handleExport}
            isDisabled={isExporting || !exportDir.trim() || result.files_recovered === 0}
            isLoading={isExporting}
            startContent={!isExporting && <Download className="w-3.5 h-3.5" />}
            className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 font-semibold px-3 text-xs h-7 shrink-0 shadow-xs"
          >
            Export All Files
          </Button>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <Button
            size="sm"
            variant="flat"
            onPress={handleViewReport}
            startContent={<FileText className="w-3.5 h-3.5" />}
            className="text-xs font-semibold bg-white dark:bg-[#1B2530] border border-slate-300 dark:border-[#26313D] text-slate-700 dark:text-slate-300 h-7"
          >
            Forensic Report
          </Button>
          <Button
            size="sm"
            variant="flat"
            onPress={onNewJob}
            className="text-xs font-semibold bg-white dark:bg-[#1B2530] border border-slate-300 dark:border-[#26313D] text-slate-700 dark:text-slate-300 h-7"
          >
            New Carve Job
          </Button>
        </div>
      </div>

      {exportedCount !== null && (
        <div className="text-[11px] text-slate-800 dark:text-slate-200 font-medium flex items-center gap-1.5 p-2 bg-slate-100 dark:bg-[#1B2530] border border-slate-200 dark:border-[#26313D] rounded-xs">
          <CheckCircle2 className="w-3.5 h-3.5 text-slate-700 dark:text-slate-300" />
          Successfully isolated and exported {exportedCount} files into <span className="font-mono font-semibold">{exportDir}</span>.
        </div>
      )}

      {exportError && (
        <div className="text-[11px] text-rose-700 dark:text-rose-300 font-medium p-2 bg-rose-50 dark:bg-rose-950/20 border border-rose-200 dark:border-rose-900 rounded-xs">
          Export Error: {exportError}
        </div>
      )}

      {/* 3. Professional File-Explorer Workspace (Left Categories, Center Files, Right Inspector) */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-4 pt-1">
        {/* Left Pane: Categories Tree & Search Filter */}
        <div className="lg:col-span-3 space-y-3">
          <Input
            size="sm"
            value={searchQuery}
            onValueChange={setSearchQuery}
            placeholder="Search artifacts..."
            startContent={<Search className="w-3.5 h-3.5 text-slate-400 shrink-0" />}
            className="text-xs w-full"
            classNames={{
              inputWrapper: 'bg-white dark:bg-[#151C24] border border-slate-300 dark:border-[#26313D] h-8',
            }}
          />

          <div className="border border-slate-200 dark:border-[#26313D] rounded-xs p-1 bg-white dark:bg-[#151C24] divide-y divide-slate-100 dark:divide-[#26313D]">
            <div className="px-2.5 py-1 text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500">
              Artifact Classifications
            </div>
            <div className="space-y-0.5 pt-1">
              {categories.map((cat) => {
                const count =
                  cat.key === 'all'
                    ? result.recovered_files.length
                    : result.recovered_files.filter((f) => f.category.toLowerCase() === cat.key.toLowerCase()).length;
                const isSelected = selectedCategory === cat.key;

                return (
                  <button
                    key={cat.key}
                    type="button"
                    onClick={() => setSelectedCategory(cat.key)}
                    className={`w-full flex items-center justify-between px-2.5 py-1.5 text-xs rounded-xs font-medium transition-colors cursor-pointer text-left ${
                      isSelected
                        ? 'bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 font-bold'
                        : 'text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-[#1B2530]'
                    }`}
                  >
                    <span>{cat.label}</span>
                    <span className={`font-mono text-[10px] ${isSelected ? 'text-slate-300 dark:text-slate-600' : 'text-slate-400'}`}>
                      {count}
                    </span>
                  </button>
                );
              })}
            </div>
          </div>
        </div>

        {/* Center Pane: Structured Files Table */}
        <div className="lg:col-span-6 space-y-2">
          {filteredFiles.length === 0 ? (
            <div className="p-8 text-center text-slate-400 text-xs border border-dashed border-slate-300 dark:border-[#26313D] rounded-sm">
              {searchQuery ? 'No artifacts match the filter.' : 'Zero files recovered for this category.'}
            </div>
          ) : (
            <div className="border border-slate-200 dark:border-[#26313D] rounded-sm overflow-hidden bg-white dark:bg-[#151C24]">
              <Table
                aria-label="Recovered files list"
                classNames={{
                  base: 'min-w-full',
                  table: 'min-w-full text-xs',
                  thead: '[&>tr]:first:rounded-none',
                  th: 'bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 font-semibold text-[10px] uppercase py-2 px-3 border-b border-slate-200 dark:border-[#26313D]',
                  td: 'py-2 px-3 border-b border-slate-100 dark:border-[#26313D] text-slate-800 dark:text-slate-200',
                  wrapper: 'bg-transparent shadow-none p-0',
                }}
              >
                <TableHeader>
                  <TableColumn>NAME & TYPE</TableColumn>
                  <TableColumn>SIZE</TableColumn>
                  <TableColumn>SECTOR OFFSET</TableColumn>
                  <TableColumn align="end">CONFIDENCE</TableColumn>
                </TableHeader>
                <TableBody>
                  {paginatedFiles.map((file) => {
                    const isSelected = selectedFile?.file_id === file.file_id;

                    return (
                      <TableRow
                        key={file.file_id}
                        onClick={() => setSelectedFileId(file.file_id)}
                        className={`cursor-pointer transition-colors ${
                          isSelected
                            ? 'bg-slate-100 dark:bg-[#1B2530] font-semibold'
                            : 'hover:bg-slate-50 dark:hover:bg-[#1B2530]/50'
                        }`}
                      >
                        <TableCell>
                          <div className="flex items-center gap-2">
                            {getCategoryIcon(file.category)}
                            <div>
                              <div className="font-mono text-xs font-semibold text-slate-900 dark:text-slate-100 truncate max-w-[180px]">
                                {file.suggested_filename}
                              </div>
                              <div className="text-[10px] text-slate-400 uppercase">
                                {file.file_type}
                              </div>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell className="font-mono text-[11px]">
                          {formatBytes(file.size_bytes)}
                        </TableCell>
                        <TableCell className="font-mono text-[11px] text-slate-500">
                          0x{file.source_offset.toString(16).toUpperCase()}
                        </TableCell>
                        <TableCell>
                          <Chip
                            size="sm"
                            variant="flat"
                            className={`font-mono text-[10px] font-bold h-5 ${
                              file.confidence_grade === 'high'
                                ? 'bg-slate-200 dark:bg-[#26313D] text-slate-800 dark:text-slate-200'
                                : file.confidence_grade === 'medium'
                                ? 'bg-amber-100 dark:bg-amber-950/40 text-amber-800 dark:text-amber-300'
                                : 'bg-slate-100 text-slate-500'
                            }`}
                          >
                            {file.confidence_score}%
                          </Chip>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>

              {filteredFiles.length > pageSize && (
                <div className="flex items-center justify-between px-3 py-2 text-xs border-t border-slate-200 dark:border-[#26313D] bg-slate-50/70 dark:bg-[#10161D] text-slate-600 dark:text-slate-400">
                  <div className="text-[11px] font-mono">
                    Showing {(safePage - 1) * pageSize + 1}–{Math.min(safePage * pageSize, filteredFiles.length)} of {filteredFiles.length}
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-[11px] font-mono">
                      Page {safePage} / {totalPages}
                    </span>
                    <div className="flex items-center gap-1">
                      <Button
                        size="sm"
                        variant="bordered"
                        isDisabled={safePage <= 1}
                        onPress={() => setCurrentPage((p) => Math.max(1, p - 1))}
                        className="h-6 min-w-12 px-2 text-[11px] border-slate-300 dark:border-[#26313D] text-slate-700 dark:text-slate-300"
                      >
                        Prev
                      </Button>
                      <Button
                        size="sm"
                        variant="bordered"
                        isDisabled={safePage >= totalPages}
                        onPress={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
                        className="h-6 min-w-12 px-2 text-[11px] border-slate-300 dark:border-[#26313D] text-slate-700 dark:text-slate-300"
                      >
                        Next
                      </Button>
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Right Pane: Contextual File Details Inspector */}
        <div className="lg:col-span-3">
          {selectedFile ? (
            <div className="border border-slate-200 dark:border-[#26313D] rounded-sm p-3.5 space-y-3 bg-white dark:bg-[#151C24] text-xs">
              <div className="border-b border-slate-200 dark:border-[#26313D] pb-2">
                <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
                  Artifact Inspector
                </span>
                <div className="font-mono font-bold text-slate-900 dark:text-slate-100 text-xs mt-1 truncate">
                  {selectedFile.suggested_filename}
                </div>
                <div className="flex items-center gap-2 mt-1">
                  <Chip size="sm" variant="flat" className="text-[9px] uppercase font-bold h-4">
                    {selectedFile.category}
                  </Chip>
                  <span className="text-slate-400">·</span>
                  <span className="font-mono text-[10px] text-slate-500">
                    {formatBytes(selectedFile.size_bytes)}
                  </span>
                </div>
              </div>

              {/* Offset & Heuristic Telemetry */}
              <div className="space-y-1.5 text-[11px]">
                <div className="flex justify-between py-0.5 border-b border-slate-100 dark:border-[#26313D]">
                  <span className="text-slate-400">Carving Method:</span>
                  <span className="font-mono text-slate-800 dark:text-slate-200 font-semibold">{selectedFile.recovery_method}</span>
                </div>
                <div className="flex justify-between py-0.5 border-b border-slate-100 dark:border-[#26313D]">
                  <span className="text-slate-400">Start LBA Offset:</span>
                  <span className="font-mono text-slate-800 dark:text-slate-200">0x{selectedFile.source_offset.toString(16).toUpperCase()}</span>
                </div>
                <div className="flex justify-between py-0.5 border-b border-slate-100 dark:border-[#26313D]">
                  <span className="text-slate-400">End LBA Offset:</span>
                  <span className="font-mono text-slate-800 dark:text-slate-200">0x{(selectedFile.source_offset + selectedFile.size_bytes).toString(16).toUpperCase()}</span>
                </div>
                <div className="flex justify-between py-0.5 border-b border-slate-100 dark:border-[#26313D]">
                  <span className="text-slate-400">Exact Bytes:</span>
                  <span className="font-mono text-slate-800 dark:text-slate-200">{selectedFile.size_bytes.toLocaleString()} B</span>
                </div>
                <div className="flex justify-between py-0.5 border-b border-slate-100 dark:border-[#26313D]">
                  <span className="text-slate-400">Validation Status:</span>
                  <span className="font-semibold text-slate-900 dark:text-slate-100 uppercase text-[10px]">{selectedFile.validation_status}</span>
                </div>
              </div>

              {/* Score breakdown chips */}
              <div className="space-y-1 pt-1">
                <span className="text-[10px] font-bold text-slate-400 uppercase block">Validation Breakdown</span>
                <div className="grid grid-cols-2 gap-1 text-[10px] font-mono">
                  <div className="p-1 bg-slate-50 dark:bg-[#1B2530] rounded-xs border border-slate-200 dark:border-[#26313D]">
                    Confidence: {selectedFile.confidence_score}%
                  </div>
                  <div className="p-1 bg-slate-50 dark:bg-[#1B2530] rounded-xs border border-slate-200 dark:border-[#26313D]">
                    Grade: {selectedFile.confidence_grade.toUpperCase()}
                  </div>
                </div>
              </div>

              {/* Actions for File */}
              {selectedFile.output_relative_path ? (
                <div className="pt-2 space-y-1.5 border-t border-slate-200 dark:border-[#26313D]">
                  <span className="text-[10px] font-bold text-slate-400 uppercase block">Disk Export Actions</span>
                  <div className="flex gap-2">
                    <Button
                      size="sm"
                      variant="flat"
                      onPress={() => handleOpenFile(selectedFile)}
                      isDisabled={actionState.loading}
                      startContent={<ExternalLink className="w-3 h-3" />}
                      className="flex-1 text-xs h-7 bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 border border-slate-300 dark:border-[#26313D]"
                    >
                      Open
                    </Button>
                    <Button
                      size="sm"
                      variant="flat"
                      onPress={() => handleRevealFile(selectedFile)}
                      isDisabled={actionState.loading}
                      startContent={<FolderOpen className="w-3 h-3" />}
                      className="flex-1 text-xs h-7 bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 border border-slate-300 dark:border-[#26313D]"
                    >
                      Reveal
                    </Button>
                  </div>
                </div>
              ) : (
                <div className="p-2 text-[10px] text-slate-400 bg-slate-50 dark:bg-[#1B2530] rounded-xs border border-slate-200 dark:border-[#26313D] italic">
                  Run &quot;Export All Files&quot; above to isolate and write artifacts to disk.
                </div>
              )}
            </div>
          ) : (
            <div className="border border-dashed border-slate-300 dark:border-[#26313D] rounded-sm p-6 text-center text-slate-400 text-xs">
              Select an artifact from the table to inspect metadata.
            </div>
          )}
        </div>
      </div>

      {/* Report Modal */}
      <Modal
        isOpen={showReportModal && !!report}
        onClose={() => setShowReportModal(false)}
        size="lg"
        classNames={{
          base: 'bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D]',
          header: 'border-b border-slate-200 dark:border-[#26313D]',
          footer: 'border-t border-slate-200 dark:border-[#26313D]',
        }}
      >
        <ModalContent>
          {report && (
            <>
              <ModalHeader className="flex items-center gap-2">
                <ShieldCheck className="w-5 h-5 text-slate-700 dark:text-slate-300" />
                <span className="text-sm font-bold text-slate-900 dark:text-slate-100">
                  Forensic Recovery Report
                </span>
              </ModalHeader>
              <ModalBody className="py-4 space-y-3 text-xs text-slate-600 dark:text-slate-300">
                <div className="flex justify-between py-1 border-b border-slate-100 dark:border-[#26313D]">
                  <span className="text-slate-500">Report ID:</span>
                  <span className="font-mono text-slate-800 dark:text-slate-200">{report.report_id}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-100 dark:border-[#26313D]">
                  <span className="text-slate-500">Job ID:</span>
                  <span className="font-mono text-slate-800 dark:text-slate-200">{report.job_id}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-100 dark:border-[#26313D]">
                  <span className="text-slate-500">Total Files Recovered:</span>
                  <span className="font-bold text-slate-900 dark:text-slate-100">{report.total_files_recovered}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-100 dark:border-[#26313D]">
                  <span className="text-slate-500">Average Confidence:</span>
                  <span className="font-bold text-slate-900 dark:text-slate-100">{report.average_confidence.toFixed(1)}%</span>
                </div>
                <Divider className="bg-slate-200 dark:bg-[#26313D]" />
                <div className="space-y-1">
                  <span className="text-slate-500 block">Report Cryptographic Digest:</span>
                  <span className="font-mono text-[10px] break-all bg-slate-50 dark:bg-[#1B2530] p-2 rounded-xs block border border-slate-200 dark:border-[#26313D] text-slate-800 dark:text-slate-200">
                    {report.report_digest}
                  </span>
                </div>
                <div className="space-y-1">
                  <span className="text-slate-500 block">Source Image SHA-256:</span>
                  <span className="font-mono text-[10px] break-all bg-slate-50 dark:bg-[#1B2530] p-2 rounded-xs block border border-slate-200 dark:border-[#26313D] text-slate-800 dark:text-slate-200">
                    {report.source_image_sha256}
                  </span>
                </div>
              </ModalBody>
              <ModalFooter>
                <Button
                  size="sm"
                  onPress={() => setShowReportModal(false)}
                  className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 font-semibold text-xs px-4"
                >
                  Close Report
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>
    </div>
  );
};

export default RecoveryResults;
