import { Goose } from '../components/icons/Goose';
import { Bird1 } from '../components/icons/Bird1';
import { Bird2 } from '../components/icons/Bird2';
import { Bird3 } from '../components/icons/Bird3';
import { Bird4 } from '../components/icons/Bird4';
import { Bird5 } from '../components/icons/Bird5';
import { Bird6 } from '../components/icons/Bird6';
import type { GlyphPack } from './types';

export const goosePack: GlyphPack = {
  id: 'goose',
  name: 'Goose',
  emoji: '🪿',
  description: 'The original.',
  StaticGlyph: Goose,
  AnimationFrames: [Bird1, Bird2, Bird3, Bird4, Bird5, Bird6],
};
