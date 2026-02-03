import { Hallucination } from '../hallucination';
import { Bias } from '../bias';
import { Toxicity } from '../toxicity';
import { All } from '../all';

describe('evals core logic', () => {
  it('Hallucination system prompt includes custom categories', () => {
    const evaler = new Hallucination({ customCategories: { foo: 'desc' } });
    const prompt = evaler.getSystemPrompt();
    expect(prompt).toContain('Additional Hallucination Categories:');
    expect(prompt).toContain('- foo: desc');
  });

  it('BiasDetector system prompt includes custom categories', () => {
    const evaler = new Bias({ customCategories: { bar: 'desc2' } });
    const prompt = evaler.getSystemPrompt();
    expect(prompt).toContain('Additional Bias Categories:');
    expect(prompt).toContain('- bar: desc2');
  });

  it('ToxicityDetector system prompt includes custom categories', () => {
    const evaler = new Toxicity({ customCategories: { baz: 'desc3' } });
    const prompt = evaler.getSystemPrompt();
    expect(prompt).toContain('Additional Toxicity Categories:');
    expect(prompt).toContain('- baz: desc3');
  });

  it('All system prompt includes custom categories', () => {
    const evaler = new All({ customCategories: { qux: 'desc4' } });
    const prompt = evaler.getSystemPrompt();
    expect(prompt).toContain('Additional Evaluation Categories:');
    expect(prompt).toContain('- qux: desc4');
  });
});
