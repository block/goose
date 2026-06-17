#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const imagesDir = path.dirname(fileURLToPath(import.meta.url));
const brandMarkPath = path.join(imagesDir, 'brand-mark.svg');
const glyphPath = path.join(imagesDir, 'glyph.svg');
const iconPath = path.join(imagesDir, 'icon.svg');

const brandMark = fs.readFileSync(brandMarkPath, 'utf8');
const svgMatch = brandMark.match(/<svg[^>]*viewBox="([^"]+)"[^>]*>([\s\S]*?)<\/svg>/i);

if (!svgMatch) {
  throw new Error(`Unable to parse canonical brand mark: ${brandMarkPath}`);
}

const [, markViewBox, rawMarkInner] = svgMatch;
const markInner = rawMarkInner.replace(/<title>[\s\S]*?<\/title>\s*/i, '').trim();
const darkMarkInner = markInner.replaceAll('currentColor', '#161616');

const generatedBanner = [
  '<?xml version="1.0" encoding="UTF-8"?>',
  '<!-- Generated from brand-mark.svg via generate-brand-assets.mjs. Do not edit by hand. -->',
].join('\n');

const glyphSvg = `${generatedBanner}
<svg xmlns="http://www.w3.org/2000/svg" viewBox="${markViewBox}" fill="none" data-brand-source="brand-mark.svg">
${darkMarkInner}
</svg>
`;

const iconSvg = `${generatedBanner}
<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024" fill="none" data-brand-source="brand-mark.svg">
  <title>收到 app icon</title>
  <rect x="64" y="64" width="896" height="896" rx="224" fill="#ffffff" />
  <g transform="translate(160 160) scale(22)">
${darkMarkInner}
  </g>
</svg>
`;

fs.writeFileSync(glyphPath, glyphSvg);
fs.writeFileSync(iconPath, iconSvg);
