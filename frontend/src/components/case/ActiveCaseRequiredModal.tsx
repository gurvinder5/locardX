import React, { useState, useEffect } from 'react';
import { AlertTriangle, FolderPlus, CheckCircle2 } from 'lucide-react';
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  Button,
  Select,
  SelectItem,
  Chip,
  Divider,
} from '@heroui/react';
import { useCaseStore } from '../../stores/caseStore';
import { useAuthStore } from '../../stores/authStore';
import { CreateCaseModal } from './CreateCaseModal';

interface ActiveCaseRequiredModalProps {
  isOpen: boolean;
  onClose?: () => void;
  operationName?: string;
  onCaseActivated?: () => void;
}

export const ActiveCaseRequiredModal: React.FC<ActiveCaseRequiredModalProps> = ({
  isOpen,
  onClose,
  operationName = 'Forensic Operation',
  onCaseActivated,
}) => {
  const { activeCase, cases, loadCases, setActiveCase, createCase, isLoading } = useCaseStore();
  const { sessionToken } = useAuthStore();
  const [selectedCaseId, setSelectedCaseId] = useState<string>('');
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadCases();
    }
  }, [isOpen, loadCases]);

  const openCases = cases.filter(
    (c) => c.status === 'open' || c.status === 'in_progress'
  );

  useEffect(() => {
    if (openCases.length > 0 && !selectedCaseId) {
      setSelectedCaseId(openCases[0].case_id);
    }
  }, [openCases, selectedCaseId]);

  const handleSelectCase = async () => {
    if (!selectedCaseId) {
      setError('Please select a valid open case.');
      return;
    }
    try {
      setError(null);
      await setActiveCase(selectedCaseId);
      if (onCaseActivated) onCaseActivated();
      if (onClose) onClose();
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Failed to set active case';
      setError(msg);
    }
  };

  const isCaseClosed = activeCase && (activeCase.status === 'completed' || activeCase.status === 'archived');

  return (
    <>
      <Modal
        isOpen={isOpen}
        onClose={onClose}
        size="lg"
        placement="center"
        classNames={{
          base: 'bg-white dark:bg-[#151C24] border border-amber-300 dark:border-amber-700/80 shadow-2xl',
          header: 'border-b border-amber-100 dark:border-amber-900/40 bg-amber-50/70 dark:bg-amber-950/20 py-4 px-6',
          body: 'py-5 px-6 space-y-4',
          footer: 'border-t border-slate-200 dark:border-[#26313D] bg-slate-50 dark:bg-[#10161D] py-3 px-6',
        }}
      >
        <ModalContent>
          {() => (
            <>
              <ModalHeader className="flex items-center gap-3">
                <div className="p-2 rounded-lg bg-amber-100 dark:bg-amber-900/50 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-700">
                  <AlertTriangle className="w-5 h-5" />
                </div>
                <div>
                  <h3 className="text-sm font-bold text-slate-900 dark:text-white">
                    {isCaseClosed ? 'Active Case is Closed / Archived' : 'Active Forensic Case Required'}
                  </h3>
                  <p className="text-xs text-slate-500 dark:text-slate-400 font-normal">
                    Chain of Custody & Forensic Policy Enforcement
                  </p>
                </div>
              </ModalHeader>

              <ModalBody>
                <div className="p-3.5 bg-slate-50 dark:bg-[#1B2530] rounded-lg border border-slate-200 dark:border-[#26313D] text-xs space-y-1.5">
                  <p className="font-semibold text-slate-900 dark:text-white">
                    {operationName} requires an active, open case context.
                  </p>
                  <p className="text-slate-600 dark:text-slate-400 leading-relaxed">
                    To guarantee forensic defensibility, all bitstream acquisitions, filesystem recoveries,
                    deep carving operations, and certificates must be strictly bound to an open investigation case.
                  </p>
                </div>

                {error && (
                  <div className="p-3 bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900 rounded-lg text-xs text-rose-700 dark:text-rose-300 flex items-center gap-2">
                    <AlertTriangle className="w-4 h-4 shrink-0 text-rose-500" />
                    <span>{error}</span>
                  </div>
                )}

                {/* Case Selection Form */}
                <div className="space-y-2">
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300">
                    Select an Existing Open Case
                  </label>
                  {openCases.length > 0 ? (
                    <div className="flex gap-2 items-center">
                      <Select
                        size="sm"
                        variant="bordered"
                        selectedKeys={selectedCaseId ? [selectedCaseId] : []}
                        onChange={(e) => setSelectedCaseId(e.target.value)}
                        placeholder="Choose open forensic case"
                        aria-label="Select Open Forensic Case"
                        className="flex-1"
                      >
                        {openCases.map((c) => (
                          <SelectItem key={c.case_id} textValue={`${c.case_reference} — ${c.title}`}>
                            <div className="flex items-center justify-between">
                              <span className="font-mono text-xs">{c.case_reference}</span>
                              <span className="text-xs text-slate-500 truncate max-w-[200px]">{c.title}</span>
                              <Chip size="sm" variant="flat" className="text-[10px] h-4">
                                {c.status}
                              </Chip>
                            </div>
                          </SelectItem>
                        ))}
                      </Select>
                      <Button
                        size="sm"
                        color="primary"
                        onPress={handleSelectCase}
                        isLoading={isLoading}
                        isDisabled={!selectedCaseId}
                        startContent={!isLoading && <CheckCircle2 className="w-4 h-4" />}
                        className="bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-semibold h-9 px-4 shrink-0"
                      >
                        Set Active
                      </Button>
                    </div>
                  ) : (
                    <p className="text-xs text-slate-500 italic">
                      No active or in-progress cases found. Please create a new case to proceed.
                    </p>
                  )}
                </div>

                <div className="relative flex py-1 items-center">
                  <Divider className="flex-1 bg-slate-200 dark:bg-[#26313D]" />
                  <span className="shrink mx-3 text-xs text-slate-400 font-mono uppercase">OR</span>
                  <Divider className="flex-1 bg-slate-200 dark:bg-[#26313D]" />
                </div>

                {/* Create New Case Button */}
                <div>
                  <Button
                    size="sm"
                    variant="bordered"
                    onPress={() => setShowCreateModal(true)}
                    startContent={<FolderPlus className="w-4 h-4 text-slate-600 dark:text-slate-400" />}
                    className="w-full h-10 border-slate-300 dark:border-[#26313D] text-slate-900 dark:text-white font-semibold"
                  >
                    Create New Investigation Case
                  </Button>
                </div>
              </ModalBody>

              <ModalFooter>
                {onClose && (
                  <Button
                    size="sm"
                    variant="light"
                    onPress={onClose}
                    className="text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white"
                  >
                    Dismiss
                  </Button>
                )}
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>

      {/* Embedded CreateCaseModal */}
      {showCreateModal && (
        <CreateCaseModal
          isOpen={showCreateModal}
          onClose={() => setShowCreateModal(false)}
          onSubmit={async (req) => {
            const token = sessionToken || 'dev-session-token';
            const newCase = await createCase(req, token);
            setShowCreateModal(false);
            if (onCaseActivated) onCaseActivated();
            if (onClose) onClose();
            return newCase;
          }}
        />
      )}
    </>
  );
};
