import type { ComponentType } from 'react';

/** Props accepted by all glyph components — matches existing icon conventions */
export interface GlyphProps {
  className?: string;
}

export interface GlyphPack {
  id: string;
  name: string;
  emoji: string;
  description: string;
  /** Static icon used in sidebar logo, welcome screen, etc. */
  StaticGlyph: ComponentType<GlyphProps>;
  /** Frame-based animation components (goose uses Bird1–6). If absent, StaticGlyph is used. */
  AnimationFrames?: ComponentType<GlyphProps>[];
}
