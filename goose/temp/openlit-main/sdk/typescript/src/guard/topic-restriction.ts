import { BaseGuard } from './base';
import { GuardConfig, GuardResult } from './types';
import { customRuleDetection, toGuardResult, applyThresholdScore, guardMetrics, guardMetricAttributes } from './utils';
import { formatPrompt } from '../evals/utils';
import { parseLlmResponse } from '../llm';

export class TopicRestriction extends BaseGuard {
  constructor(config: GuardConfig = {}) {
    super(config);
  }

  protected getSystemPrompt(): string {
    return [
      'Output Requirement: JSON Format Only',
      '',
      'Your task is to carefully analyze the provided text and identify any instances of restricted topics, and categorize the classification when found. Apply strict criteria in your assessment.',
      '',
      'Restricted topics include information or requests that are not allowed or are outside the permitted scope for this system.',
      '',
      'Presented Text:',
      '{{prompt}}',
      '',
      'Expected JSON Structure:',
      '',
      '{',
      '    "score": "float",',
      '    "verdict": "yes or no",',
      '    "guard": "topic_restriction",',
      '    "classification": "CATEGORY_OF_TOPIC_RESTRICTION",',
      '    "explanation": "Very short one-sentence reason"',
      '}',
      '',
      'Scoring System:',
      '- The score ranges from 0 to 1. Scores should be 0 if the classification is "none". Any other classification must be accompanied by a valid category and a score reflecting the restriction.',
      '',
      'Important: Use only the categories below to classify restricted topics. If the text does not fit any listed category, classify it as "none" with a score of 0.',
      '',
      'Categories of Restricted Topics (use only these categories):',
      '- politics: Requests or discussions about political figures, parties, or policies that are not allowed.',
      '- violence: Requests or discussions involving physical harm, aggression, or violent acts.',
      '- personal_information: Requests for or mentions of sensitive personal data (e.g., credit card, address, email, phone number).',
      '- confidential: Requests for or mentions of confidential or proprietary information.',
      '- illegal: Requests or discussions about illegal activities.',
      '',
      'Final Steps:',
      '- If no restricted topics are detected, or if the topic does not fit into one of the above categories, the return must be: {"score": 0, "verdict": "no", "guard": "none", "classification": "none", "explanation": "none"}.',
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
      // Use correct template variable for prompt and satisfy EvalsInput type
      const prompt = formatPrompt(this.getSystemPrompt(), { prompt: text, text });
      const response = await this.llmResponse(prompt);
      llmResult = toGuardResult(parseLlmResponse(response), 'topic_restriction');
    }
    let result = customRuleResult.score >= llmResult.score ? customRuleResult : llmResult;
    result = applyThresholdScore(result, this.thresholdScore);

    // Metrics collection if enabled
    if (this.collectMetrics) {
      const validator = this.provider || 'custom';
      guardMetrics().add(1, guardMetricAttributes(result.verdict, result.score, validator, result.classification, result.explanation));
    }
    return result;
  }
}
