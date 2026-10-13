import type { HfModelInfo, HfModelVariant } from '../../../acp/local-inference';

type Quality = HfModelInfo['quality'];
export function catalogueOrder(quality: Quality): number | undefined {
  const order = quality?.catalogueOrder;
  return order != null && order > 0 ? order : undefined;
}

export function compareQuality(a: Quality, b: Quality): number {
  const left = catalogueOrder(a) ?? Number.MAX_SAFE_INTEGER;
  const right = catalogueOrder(b) ?? Number.MAX_SAFE_INTEGER;
  return left - right;
}

export function fitRank(variants: HfModelVariant[]): number {
  if (variants.some((variant) => isEligibleVariant(variant))) return 0;
  if (
    !variants.length ||
    variants.some((variant) => !variant.assessment || variant.assessment.fit === 'unknown')
  )
    return 1;
  return 2;
}

export function isEligibleVariant(variant: HfModelVariant): boolean {
  const assessment = variant.assessment;
  return (
    variant.supported &&
    assessment != null &&
    (assessment.fit === 'likelyFit' ||
      (assessment.fit === 'unknown' &&
        assessment.upperBytes == null &&
        assessment.lowerBytes != null &&
        assessment.availableMemoryBytes > 0 &&
        assessment.lowerBytes <= assessment.availableMemoryBytes - assessment.reserveBytes)) &&
    assessment.compatibility === 'compatible' &&
    assessment.suitability === 'suitable'
  );
}

export function recommendedVariant(
  variants: HfModelVariant[],
  quality: Quality
): HfModelVariant | undefined {
  if (catalogueOrder(quality) === undefined) return undefined;
  return variants
    .filter(isEligibleVariant)
    .sort(
      (a, b) =>
        Number(b.assessment?.fit === 'likelyFit') - Number(a.assessment?.fit === 'likelyFit') ||
        b.qualityRank - a.qualityRank
    )[0];
}
