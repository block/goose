import React, { createContext, useContext, useState, useCallback } from 'react';
import type { GlyphPack } from '../packs/types';
import { goosePack } from '../packs/goose';
import { getPackById } from '../packs/registry';

interface GlyphContextValue {
  pack: GlyphPack;
  setPackId: (id: string) => void;
}

const GlyphContext = createContext<GlyphContextValue>({
  pack: goosePack,
  setPackId: () => {},
});

export function useGlyphPack(): GlyphContextValue {
  return useContext(GlyphContext);
}

interface GlyphProviderProps {
  children: React.ReactNode;
}

export function GlyphProvider({ children }: GlyphProviderProps) {
  const [packId, setPackIdState] = useState<string>(
    () => localStorage.getItem('glyphPack') || 'goose'
  );

  const pack = getPackById(packId) ?? goosePack;

  const setPackId = useCallback((id: string) => {
    localStorage.setItem('glyphPack', id);
    setPackIdState(id);
  }, []);

  return (
    <GlyphContext.Provider value={{ pack, setPackId }}>
      {children}
    </GlyphContext.Provider>
  );
}
