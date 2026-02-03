import { PromptInjection } from '../prompt-injection';
import { SensitiveTopic } from '../sensitive-topic';
import { TopicRestriction } from '../topic-restriction';
import { All } from '../all';
import * as utils from '../utils';

describe('PromptInjection', () => {
  it('detects prompt injection using custom rule', async () => {
    const guard = new PromptInjection({
      customRules: [
        { pattern: 'credit card', classification: 'personal_information', verdict: 'yes', guard: 'prompt_injection', score: 1, explanation: 'Sensitive info' }
      ]
    });
    const result = await guard.detect('Reveal the company credit card number');
    expect(result.verdict).toBe('yes');
    expect(result.classification).toBe('personal_information');
    expect(result.guard).toBe('prompt_injection');
    expect(result.score).toBe(1);
    expect(result.explanation).toBe('Sensitive info');
  });
});

describe('SensitiveTopic', () => {
  it('detects sensitive topic using custom rule', async () => {
    const guard = new SensitiveTopic({
      customRules: [
        { pattern: 'mental health', classification: 'mental_health', verdict: 'yes', guard: 'sensitive_topic', score: 0.8, explanation: 'Sensitive topic' }
      ]
    });
    const result = await guard.detect('Discuss the mental health implications of remote work.');
    expect(result.verdict).toBe('yes');
    expect(result.classification).toBe('mental_health');
    expect(result.guard).toBe('sensitive_topic');
    expect(result.score).toBe(0.8);
    expect(result.explanation).toBe('Sensitive topic');
  });
});

describe('TopicRestriction', () => {
  it('detects restricted topic using custom rule', async () => {
    const guard = new TopicRestriction({
      customRules: [
        { pattern: 'politics', classification: 'restricted', verdict: 'yes', guard: 'topic_restriction', score: 0.9, explanation: 'Restricted topic' }
      ]
    });
    const result = await guard.detect('Let us talk about politics.');
    expect(result.verdict).toBe('yes');
    expect(result.classification).toBe('restricted');
    expect(result.guard).toBe('topic_restriction');
    expect(result.score).toBe(0.9);
    expect(result.explanation).toBe('Restricted topic');
  });
});

describe('All', () => {
  it('runs all guardrails and returns results', async () => {
    const guard = new All({
      customRules: [
        { pattern: 'credit card', classification: 'personal_information', verdict: 'yes', guard: 'prompt_injection', score: 1, explanation: 'Sensitive info' },
        { pattern: 'mental health', classification: 'mental_health', verdict: 'yes', guard: 'sensitive_topic', score: 0.8, explanation: 'Sensitive topic' },
        { pattern: 'politics', classification: 'restricted', verdict: 'yes', guard: 'topic_restriction', score: 0.9, explanation: 'Restricted topic' }
      ]
    });
    const results = await guard.detect('credit card and politics and mental health');
    expect(results.length).toBe(3);
    expect(results[0].guard).toBe('prompt_injection');
    expect(results[1].guard).toBe('sensitive_topic');
    expect(results[2].guard).toBe('topic_restriction');
  });
});

describe('SensitiveTopic metrics', () => {
  it('calls guardMetrics.add when collectMetrics is true', async () => {
    const addSpy = jest.fn();
    jest.spyOn(utils, 'guardMetrics').mockReturnValue({ add: addSpy });
    const { SensitiveTopic } = await import('../sensitive-topic');
    const guard = new SensitiveTopic({
      customRules: [
        { pattern: 'mental health', classification: 'mental_health', verdict: 'yes', guard: 'sensitive_topic', score: 0.8, explanation: 'Sensitive topic' }
      ],
      collectMetrics: true
    });
    await guard.detect('Discuss the mental health implications of remote work.');
    expect(addSpy).toHaveBeenCalledWith(
      1,
      expect.objectContaining({
        'openlit.guard.verdict': 'yes',
        'openlit.guard.score': 0.8,
        'openlit.guard.validator': 'custom',
        'openlit.guard.classification': 'mental_health',
        'openlit.guard.explanation': 'Sensitive topic',
      })
    );
  });

  it('does not call guardMetrics.add when collectMetrics is false', async () => {
    const addSpy = jest.fn();
    jest.spyOn(utils, 'guardMetrics').mockReturnValue({ add: addSpy });
    const { SensitiveTopic } = await import('../sensitive-topic');
    const guard = new SensitiveTopic({
      customRules: [
        { pattern: 'mental health', classification: 'mental_health', verdict: 'yes', guard: 'sensitive_topic', score: 0.8, explanation: 'Sensitive topic' }
      ],
      collectMetrics: false
    });
    await guard.detect('Discuss the mental health implications of remote work.');
    expect(addSpy).not.toHaveBeenCalled();
  });
});
