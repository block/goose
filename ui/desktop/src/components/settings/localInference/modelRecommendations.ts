import type { HfModelInfo, HfModelVariant } from '../../../acp/local-inference';

type Quality = HfModelInfo['quality'];
const benchmarks = [
  'TIGER-Lab/MMLU-Pro',
  'LiquidAI/ifstruct-v1.0',
  'harborframework/terminal-bench-2.0',
];

export function compareQuality(a: Quality, b: Quality): number {
  const key = (quality: Quality): [number, number] => {
    for (const [index, benchmark] of benchmarks.entries()) {
      const evidence = quality?.evidence.find((entry) => entry.benchmark === benchmark);
      if (evidence) return [index, evidence.rank];
    }
    return [benchmarks.length, Number.MAX_SAFE_INTEGER];
  };
  const left = key(a);
  const right = key(b);
  return left[0] - right[0] || left[1] - right[1];
}

export function fitRank(variants: HfModelVariant[]): number {
  if (variants.some((variant) => variant.supported && variant.assessment?.fit === 'likelyFit'))
    return 0;
  if (
    !variants.length ||
    variants.some((variant) => !variant.assessment || variant.assessment.fit === 'unknown')
  )
    return 1;
  return 2;
}
