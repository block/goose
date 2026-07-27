export type ContextCategory =
  | 'system_prompt'
  | 'turn_context'
  | 'extension_instructions'
  | 'additional_instructions'
  | 'tool_definitions'
  | 'compaction_summary'
  | 'messages';

export type ContextPart = {
  label: string;
  source?: string | null;
  tokenCount: number;
  charCount: number;
  contentPreview?: string | null;
};

export type ContextSegment = {
  category: ContextCategory;
  label: string;
  source?: string | null;
  tokenCount: number;
  charCount: number;
  contentPreview?: string | null;
  parts?: ContextPart[];
};

export type ContextReportModel = {
  provider?: string | null;
  modelName: string;
  contextLimit: number;
};

export type ContextReport = {
  model: ContextReportModel;
  estimatedTotalTokens: number;
  wireTotalTokens: number;
  liveTotalTokens?: number | null;
  segments: ContextSegment[];
};
