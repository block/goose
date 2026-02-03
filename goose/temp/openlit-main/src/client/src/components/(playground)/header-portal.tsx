"use client";
import React, { createContext, ReactNode, useContext, useState } from 'react';
import { createPortal } from 'react-dom';

const PortalContext = createContext<{ headerRef: HTMLDivElement | null; setHeaderRef: (node: HTMLDivElement | null) => void }>({
	headerRef: null,
	setHeaderRef: () => { },
});

export const PortalProvider = ({ children }: { children: React.ReactNode }) => {
	const [headerRef, setHeaderRef] = useState<HTMLDivElement | null>(null);

	return (
		<PortalContext.Provider value={{ headerRef, setHeaderRef }}>
			{children}
		</PortalContext.Provider>
	);
};

export const usePortal = () => useContext(PortalContext);

export const HeaderPortal = ({ children }: { children: ReactNode }) => {
  const { headerRef } = usePortal();

  if (!headerRef) return null;

  return createPortal(children, headerRef);
};
