import { GuardConfig, GuardResult } from './types';
import { PromptInjection } from './prompt-injection';
import { SensitiveTopic } from './sensitive-topic';
import { TopicRestriction } from './topic-restriction';

export class All {
  private promptInjection: PromptInjection;
  private sensitiveTopic: SensitiveTopic;
  private topicRestriction: TopicRestriction;

  constructor(config: GuardConfig = {}) {
    // Split customRules by guard type
    const allRules = config.customRules || [];
    const piRules = allRules.filter(r => r.guard === 'prompt_injection');
    const stRules = allRules.filter(r => r.guard === 'sensitive_topic');
    const trRules = allRules.filter(r => r.guard === 'topic_restriction');

    this.promptInjection = new PromptInjection({ ...config, customRules: piRules });
    this.sensitiveTopic = new SensitiveTopic({ ...config, customRules: stRules });
    this.topicRestriction = new TopicRestriction({ ...config, customRules: trRules });
  }

  async detect(text: string): Promise<GuardResult[]> {
    const [pi, st, tr] = await Promise.all([
      this.promptInjection.detect(text),
      this.sensitiveTopic.detect(text),
      this.topicRestriction.detect(text)
    ]);
    return [pi, st, tr];
  }
}
