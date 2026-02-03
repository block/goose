import { BaseEval } from './base';
import { formatCustomCategories } from './utils';

export class Hallucination extends BaseEval {
  getSystemPrompt(): string {
    let prompt = [
      'Output Requirement: JSON Format Only',
      '',
      'Your task is to find any instances of Hallucination in text compared to the provided contexts and the optional prompt. Generate a JSON object with the following fields: \'score\', \'evaluation\', \'classification\', \'explanation\', and \'verdict\'. Use the contexts to strictly detect hallucination in the text.',
      '',
      "- 'score': A float from 0 to 1. Closer to 1 indicates a higher level of hallucination.",
      "- 'evaluation': Should always be \"hallucination\".",
      "- 'classification': Type of Hallucination from one of the hallucination categories.",
      "- 'explanation': Provide a very short sentence explanation for the selected category of Hallucination.",
      `- 'verdict': 'yes' if score > ${this.thresholdScore}, otherwise 'no'.`,
      '',
      'Hallucination Categories:',
      "- factual_inaccuracy: Incorrect facts, e.g., Context: [\"Paris is the capital of France.\"]; Text: \"Lyon is the capital.\"",
      "- nonsensical_response: Irrelevant info, e.g., Context: [\"Discussing music trends.\"]; Text: \"Golf uses clubs on grass.\"",
      "- gibberish: Nonsensical text, e.g., Context: [\"Discuss advanced algorithms.\"]; Text: \"asdas asdhasudqoiwjopakcea.\"",
      "- contradiction: Conflicting info, e.g., Context: [\"Einstein was born in 1879.\"]; Text: \"Einstein was born in 1875 and 1879.\"",
    ].join('\n');
    if (this.customCategories) {
      prompt += formatCustomCategories(this.customCategories, 'Hallucination');
    }
    return prompt;
  }
}
