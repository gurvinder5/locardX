import React from 'react';
import { Briefcase, AlertTriangle } from 'lucide-react';
import {
  Button,
  Chip,
  Checkbox,
} from '@heroui/react';
import { useCaseStore } from '../../stores/caseStore';

interface ActiveCaseBannerProps {
  associateWithCase: boolean;
  onToggleAssociate: (associate: boolean) => void;
  onOpenCaseSelector?: () => void;
}

export const ActiveCaseBanner: React.FC<ActiveCaseBannerProps> = ({
  associateWithCase,
  onToggleAssociate,
  onOpenCaseSelector,
}) => {
  const { activeCase } = useCaseStore();
  const isClosed = activeCase?.status === 'completed' || activeCase?.status === 'archived';

  if (!activeCase) {
    return (
      <div className="bg-slate-50 dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] rounded-xs p-3 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 text-xs">
        <div className="flex items-center gap-2 text-slate-700 dark:text-slate-300">
          <Briefcase className="w-4 h-4 text-slate-400 shrink-0" />
          <div>
            <span className="font-semibold text-slate-900 dark:text-white">Mode: Standalone Operation</span>
            <span className="text-slate-500 dark:text-slate-400 ml-1.5">
              (No active case selected. Sanitization records and certificates will not be bound to a forensic case.)
            </span>
          </div>
        </div>

        {onOpenCaseSelector && (
          <Button
            size="sm"
            variant="bordered"
            onPress={onOpenCaseSelector}
            className="h-7 text-xs font-semibold bg-white dark:bg-[#1B2530] border-slate-300 dark:border-[#26313D] text-slate-700 dark:text-slate-300 shrink-0"
          >
            Select Active Case
          </Button>
        )}
      </div>
    );
  }

  return (
    <div
      className={`border rounded-xs p-3 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 text-xs ${
        isClosed
          ? 'bg-rose-50/70 dark:bg-rose-950/20 border-rose-200 dark:border-rose-900 text-rose-900 dark:text-rose-200'
          : associateWithCase
          ? 'bg-slate-100 dark:bg-[#1B2530] border-slate-300 dark:border-[#26313D] text-slate-900 dark:text-white'
          : 'bg-slate-50 dark:bg-[#151C24] border-slate-200 dark:border-[#26313D] text-slate-700 dark:text-slate-300'
      }`}
    >
      <div className="flex items-start sm:items-center gap-2.5">
        <Briefcase
          className={`w-4 h-4 shrink-0 mt-0.5 sm:mt-0 ${
            isClosed ? 'text-rose-600 dark:text-rose-400' : associateWithCase ? 'text-slate-900 dark:text-white' : 'text-slate-400'
          }`}
        />
        <div>
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-semibold uppercase tracking-wider text-[10px] text-slate-500 dark:text-slate-400">
              Active Investigation Case:
            </span>
            <span className="font-mono font-bold text-slate-900 dark:text-white">{activeCase.case_reference}</span>
            <span className="text-slate-400">—</span>
            <span className="font-medium text-slate-800 dark:text-slate-200">{activeCase.title}</span>
            <Chip
              size="sm"
              variant="flat"
              className={`text-[9px] uppercase font-bold h-4 ${
                isClosed
                  ? 'bg-rose-100 dark:bg-rose-900/60 text-rose-800 dark:text-rose-200 border border-rose-300 dark:border-rose-700'
                  : 'bg-slate-200 dark:bg-[#26313D] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-600'
              }`}
            >
              {activeCase.status}
            </Chip>
          </div>

          {isClosed && (
            <p className="text-[11px] text-rose-700 dark:text-rose-300 mt-1 flex items-center gap-1 font-medium">
              <AlertTriangle className="w-3 h-3 text-rose-600 dark:text-rose-400 shrink-0" />
              <span>
                Case is closed. Forensic operations cannot be linked to this case until it is reopened.
              </span>
            </p>
          )}
        </div>
      </div>

      <div className="flex items-center gap-3 shrink-0">
        {!isClosed ? (
          <Checkbox
            size="sm"
            isSelected={associateWithCase}
            onValueChange={onToggleAssociate}
            classNames={{
              label: 'text-xs font-semibold text-slate-800 dark:text-slate-200',
            }}
          >
            Bind Execution Record to Case
          </Checkbox>
        ) : null}

        {onOpenCaseSelector && (
          <Button
            size="sm"
            variant="bordered"
            onPress={onOpenCaseSelector}
            className="h-7 text-xs font-semibold border-slate-300 dark:border-[#26313D] text-slate-700 dark:text-slate-300"
          >
            Switch Case
          </Button>
        )}
      </div>
    </div>
  );
};
