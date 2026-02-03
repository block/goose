import { BaseGuard } from './base';
import { GuardConfig, GuardResult } from './types';
import { customRuleDetection, toGuardResult, applyThresholdScore, guardMetrics, guardMetricAttributes } from './utils';
import { formatPrompt } from '../evals/utils';
import { parseLlmResponse } from '../llm';

export class SensitiveTopic extends BaseGuard {
  constructor(config: GuardConfig = {}) {
    super(config);
  }

  protected getSystemPrompt(): string {
    return [
      'Output Requirement: JSON Format Only',
      '',
      'Your task is to carefully analyze the provided text and identify any instances of sensitive topics, and categorize the classification when found. Apply strict criteria in your assessment.',
      '',
      'Sensitive topics include information or requests that may not be publicly acceptable or could be considered controversial or distressing.',
      '',
      'Presented Text:',
      '{{prompt}}',
      '',
      'Expected JSON Structure:',
      '',
      '{',
      '    "score": "float",',
      '    "verdict": "yes or no",',
      '    "guard": "sensitive_topic",',
      '    "classification": "CATEGORY_OF_SENSITIVE_TOPIC",',
      '    "explanation": "Very short one-sentence reason"',
      '}',
      '',
      'Scoring System:',
      '- The score ranges from 0 to 1. Scores should be 0 if the classification is "none". Any other classification must be accompanied by a valid category and a score reflecting the sensitivity.',
      '',
      'Important: Use only the categories below to classify sensitive topics. If the text does not fit any listed category, classify it as "none" with a score of 0.',
      '',
      'Categories of Sensitive Topics (use only these categories):',
      '- politics: Discussions or opinions about political figures, parties, or policies.',
      '- breakup: Conversations or advice related to relationship breakups or emotional distress.',
      '- violence: References to physical harm, aggression, or violent acts.',
      '- guns: Mentions of firearms, gun control, or related topics.',
      '- mental_health: Topics related to mental health issues, therapy, or emotional well-being.',
      '- discrimination: Language or topics that could be perceived as discriminatory or biased.',
      '- substance_use: Discussions about drugs, alcohol, or substance abuse.',
      '',
      'Final Steps:',
      '- If no sensitive topics are detected, or if the topic does not fit into one of the above categories, the return must be: {"score": 0, "verdict": "no", "guard": "none", "classification": "none", "explanation": "none"}.',
    ].join('\n');
  }

  async detect(text: string): Promise<GuardResult> {
    const customRuleResult = customRuleDetection(text, this.customRules);
    let llmResult: GuardResult = {
      score: 0,
      verdict: 'no',
      guard: 'none',
      classification: 'none',
      explanation: 'none'
    };
    if (this.provider) {
      const prompt = formatPrompt(this.getSystemPrompt(), { prompt: text, text });
      const response = await this.llmResponse(prompt);
      llmResult = toGuardResult(parseLlmResponse(response), 'sensitive_topic');
    }
    let result = customRuleResult.score >= llmResult.score ? customRuleResult : llmResult;
    result = applyThresholdScore(result, this.thresholdScore);
    // Metrics collection if enabled
    if (this.collectMetrics) {
      guardMetrics().add(1, guardMetricAttributes(result.verdict, result.score, this.provider || 'custom', result.classification, result.explanation));
    }
    return result;
  }
}
