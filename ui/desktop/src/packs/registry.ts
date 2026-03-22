import type { GlyphPack } from './types';
import { goosePack } from './goose';

export const allPacks: GlyphPack[] = [goosePack];

const packMap = new Map(allPacks.map((p) => [p.id, p]));

export function getPackById(id: string): GlyphPack | undefined {
  return packMap.get(id);
}
