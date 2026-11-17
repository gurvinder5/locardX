import React, { useState } from 'react';
import { Briefcase, Plus, AlertCircle } from 'lucide-react';
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  Button,
  Input,
  Textarea,
  Select,
  SelectItem,
} from '@heroui/react';
import { CreateCaseRequest } from '../../types/case';

interface CreateCaseModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (request: CreateCaseRequest) => Promise<any>;
}

export const CreateCaseModal: React.FC<CreateCaseModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
}) => {
  const [reference, setReference] = useState('');
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [priority, setPriority] = useState('Standard');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!reference.trim() || !title.trim()) {
      setError('Please provide both case reference and title.');
      return;
    }

    try {
      setSubmitting(true);
      setError(null);
      await onSubmit({
        case_reference: reference.trim(),
        title: title.trim(),
        description: description.trim(),
        metadata_json: JSON.stringify({ priority }),
      });
      onClose();
    } catch (err: any) {
      setError(err?.message || 'Failed to create case');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      size="lg"
      placement="center"
      classNames={{
        base: 'bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] shadow-2xl',
        header: 'border-b border-slate-200 dark:border-[#26313D] bg-slate-50/70 dark:bg-[#10161D] py-4 px-6',
        body: 'py-5 px-6 space-y-4',
        footer: 'border-t border-slate-200 dark:border-[#26313D] bg-slate-50/50 dark:bg-[#10161D] py-3 px-6',
      }}
    >
      <ModalContent>
        {() => (
          <form onSubmit={handleSubmit}>
            <ModalHeader className="flex items-center gap-2">
              <div className="p-1.5 rounded bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-[#26313D]">
                <Briefcase className="w-4 h-4" />
              </div>
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">
                Initiate Investigation Case
              </h3>
            </ModalHeader>

            <ModalBody className="space-y-4">
              {error && (
                <div className="p-3 bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900 rounded text-xs text-rose-700 dark:text-rose-300 flex items-center gap-2">
                  <AlertCircle className="w-4 h-4 shrink-0 text-rose-500" />
                  <span>{error}</span>
                </div>
              )}

              <div className="grid grid-cols-2 gap-3">
                <Input
                  size="sm"
                  variant="bordered"
                  label="Case Reference"
                  placeholder="e.g. INV-2026-004"
                  value={reference}
                  onValueChange={setReference}
                  isRequired
                  classNames={{
                    input: 'font-mono text-xs',
                  }}
                />

                <Select
                  size="sm"
                  variant="bordered"
                  label="Priority"
                  selectedKeys={[priority]}
                  onChange={(e) => setPriority(e.target.value)}
                >
                  <SelectItem key="Urgent">Urgent / Active Incident</SelectItem>
                  <SelectItem key="High">High</SelectItem>
                  <SelectItem key="Standard">Standard</SelectItem>
                  <SelectItem key="Low">Low / Archival</SelectItem>
                </Select>
              </div>

              <Input
                size="sm"
                variant="bordered"
                label="Case Title"
                placeholder="e.g. Unauthorized Intrusion & Data Extraction - Server Node 3"
                value={title}
                onValueChange={setTitle}
                isRequired
              />

              <Textarea
                size="sm"
                variant="bordered"
                label="Case Description & Scope"
                placeholder="Provide context, evidence scope, authority/warrant number, and targets..."
                minRows={3}
                value={description}
                onValueChange={setDescription}
              />
            </ModalBody>

            <ModalFooter className="flex justify-end gap-2">
              <Button
                size="sm"
                variant="bordered"
                onPress={onClose}
                className="border-slate-300 dark:border-[#26313D] text-slate-700 dark:text-slate-300"
              >
                Cancel
              </Button>
              <Button
                size="sm"
                color="primary"
                type="submit"
                isLoading={submitting}
                isDisabled={submitting || !reference.trim() || !title.trim()}
                startContent={!submitting && <Plus className="w-4 h-4" />}
                className="bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-semibold shadow-xs"
              >
                Create Case
              </Button>
            </ModalFooter>
          </form>
        )}
      </ModalContent>
    </Modal>
  );
};
