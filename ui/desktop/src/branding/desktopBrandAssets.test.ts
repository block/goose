import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

const imagesDir = path.resolve(process.cwd(), 'src', 'images');

function readImageAsset(name: string): string {
  return fs.readFileSync(path.join(imagesDir, name), 'utf8');
}

describe('desktop brand assets', () => {
  it('keeps a single canonical Carbon brand mark and generated svg outputs', () => {
    const brandMark = readImageAsset('brand-mark.svg');
    const glyphSvg = readImageAsset('glyph.svg');
    const iconSvg = readImageAsset('icon.svg');
    const prepareScript = readImageAsset('prepare.sh');

    expect(brandMark).toContain('IBM Carbon AiEnabledEdt');
    expect(glyphSvg).toContain('Generated from brand-mark.svg via generate-brand-assets.mjs');
    expect(iconSvg).toContain('Generated from brand-mark.svg via generate-brand-assets.mjs');
    expect(prepareScript).toContain('generate-brand-assets.mjs');
  });
});
