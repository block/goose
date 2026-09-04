export type TextDirection = 'rtl' | 'ltr';

type Range = readonly [start: number, end: number];

// Blocks whose characters have strong RTL direction (bidi class R or AL).
const RTL_RANGES: readonly Range[] = [
  [0x0590, 0x05ff], // Hebrew
  [0x0600, 0x06ff], // Arabic
  [0x0700, 0x074f], // Syriac
  [0x0750, 0x077f], // Arabic supplement
  [0x0780, 0x07bf], // Thaana
  [0x07c0, 0x082f], // NKo, Samaritan
  [0x0840, 0x085f], // Mandaic
  [0x08a0, 0x08ff], // Arabic extended-B
  [0xfb1d, 0xfdff], // Hebrew and Arabic presentation forms
  [0xfe70, 0xfeff], // Arabic presentation forms-B
];

// Digits inside RTL blocks are weak (bidi class AN/EN), not strong RTL, so a
// message made only of digits must not flip direction.
const WEAK_RTL_RANGES: readonly Range[] = [
  [0x0660, 0x0669], // Arabic-Indic digits
  [0x06f0, 0x06f9], // Extended Arabic-Indic digits
];

// Blocks with strong LTR direction. Checked after RTL_RANGES, so overlaps with
// RTL blocks are impossible. Digits, punctuation, whitespace, emoji and scripts
// not listed here count as neutral and don't influence the result.
const LTR_RANGES: readonly Range[] = [
  [0x0041, 0x005a], // A-Z
  [0x0061, 0x007a], // a-z
  [0x00c0, 0x024f], // Latin-1 letters, Latin extended-A/B, IPA
  [0x0370, 0x03ff], // Greek
  [0x0400, 0x052f], // Cyrillic and supplement
  [0x0530, 0x058f], // Armenian
  [0x0900, 0x0dff], // Indic scripts
  [0x0e00, 0x0eff], // Thai, Lao
  [0x10a0, 0x10ff], // Georgian
  [0x1e00, 0x1eff], // Latin extended additional
  [0x1f00, 0x1fff], // Greek extended
  [0x3040, 0x30ff], // Hiragana, Katakana
  [0x3400, 0x4dbf], // CJK ext-A
  [0x4e00, 0x9fff], // CJK unified
  [0xac00, 0xd7af], // Hangul syllables
  [0xf900, 0xfaff], // CJK compatibility ideographs
];

function inRanges(code: number, ranges: readonly Range[]): boolean {
  for (const [start, end] of ranges) {
    if (code >= start && code <= end) return true;
  }
  return false;
}

/**
 * Heuristic direction for a block of text: counts strong RTL characters
 * against strong LTR characters, so a leading English word or number doesn't
 * force an Arabic/Hebrew message to render LTR.
 *
 * Returns null when the text has no strong directional characters at all
 * (digits, punctuation, whitespace, emoji); callers should treat that as
 * "inherit from the surrounding context".
 */
export function getTextDirection(text: string): TextDirection | null {
  if (!text) return null;

  let rtlCount = 0;
  let ltrCount = 0;
  for (let i = 0; i < text.length; i++) {
    const code = text.charCodeAt(i);
    if (inRanges(code, WEAK_RTL_RANGES)) continue;
    if (inRanges(code, RTL_RANGES)) {
      rtlCount++;
    } else if (inRanges(code, LTR_RANGES)) {
      ltrCount++;
    }
  }

  if (rtlCount === 0 && ltrCount === 0) return null;
  return rtlCount > ltrCount ? 'rtl' : 'ltr';
}
