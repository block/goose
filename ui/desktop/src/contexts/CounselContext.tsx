import React, { createContext, useContext, useState, ReactNode } from 'react';

interface CounselContextType {
  isCounselModalOpen: boolean;
  openCounselModal: () => void;
  closeCounselModal: () => void;
}

const CounselContext = createContext<CounselContextType | undefined>(undefined);

export const CounselProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [isCounselModalOpen, setIsCounselModalOpen] = useState(false);

  const openCounselModal = () => setIsCounselModalOpen(true);
  const closeCounselModal = () => setIsCounselModalOpen(false);

  return (
    <CounselContext.Provider
      value={{
        isCounselModalOpen,
        openCounselModal,
        closeCounselModal,
      }}
    >
      {children}
    </CounselContext.Provider>
  );
};

export const useCounsel = (): CounselContextType => {
  const context = useContext(CounselContext);
  if (!context) {
    throw new Error('useCounsel must be used within a CounselProvider');
  }
  return context;
};
