import { defineMessages } from '../../i18n';
import type { ContextCategory } from '../../types/contextReport';

export const categoryMessages = defineMessages({
  system_prompt: {
    id: 'contextXray.category.systemPrompt',
    defaultMessage: 'System prompt',
  },
  turn_context: {
    id: 'contextXray.category.turnContext',
    defaultMessage: 'Turn context',
  },
  extension_instructions: {
    id: 'contextXray.category.extensionInstructions',
    defaultMessage: 'Extension instructions',
  },
  additional_instructions: {
    id: 'contextXray.category.additionalInstructions',
    defaultMessage: 'Instructions & hints',
  },
  tool_definitions: {
    id: 'contextXray.category.toolDefinitions',
    defaultMessage: 'Tool definitions',
  },
  compaction_summary: {
    id: 'contextXray.category.compactionSummary',
    defaultMessage: 'Compaction summary',
  },
  messages: {
    id: 'contextXray.category.messages',
    defaultMessage: 'Conversation',
  },
});

export const categoryColorClass: Record<ContextCategory, string> = {
  messages: 'bg-[#2a78d6] dark:bg-[#3987e5]',
  tool_definitions: 'bg-[#1baf7a] dark:bg-[#199e70]',
  system_prompt: 'bg-[#eda100] dark:bg-[#c98500]',
  extension_instructions: 'bg-[#008300]',
  additional_instructions: 'bg-[#4a3aa7] dark:bg-[#9085e9]',
  turn_context: 'bg-[#e34948] dark:bg-[#e66767]',
  compaction_summary: 'bg-[#b5359b] dark:bg-[#d76bc4]',
};
