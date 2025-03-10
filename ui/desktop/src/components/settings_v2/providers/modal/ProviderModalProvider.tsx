import React, { createContext, useContext, useState, useMemo, useCallback } from 'react';
import { ProviderDetails } from '../../../../api';

interface ProviderModalContextType {
  isOpen: boolean;
  currentProvider: ProviderDetails | null;
  modalProps: any;
  openModal: (provider: ProviderDetails, additionalProps: any) => void;
  closeModal: () => void;
}

const defaultContext: ProviderModalContextType = {
  isOpen: false,
  currentProvider: null,
  modalProps: {},
  openModal: () => {},
  closeModal: () => {},
};

const ProviderModalContext = createContext<ProviderModalContextType>(defaultContext);

export const useProviderModal = () => useContext<ProviderModalContextType>(ProviderModalContext);

export const ProviderModalProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [currentProvider, setCurrentProvider] = useState<ProviderDetails | null>(null);
  const [modalProps, setModalProps] = useState({});

  // Use useCallback to prevent function recreation on each render
  const openModal = useCallback((provider: ProviderDetails, additionalProps = {}) => {
    setCurrentProvider(provider);
    setModalProps(additionalProps);
    setIsOpen(true);
  }, []);

  const closeModal = useCallback(() => {
    setIsOpen(false);
    // Use a small timeout to prevent UI flicker
    setTimeout(() => {
      setCurrentProvider(null);
      setModalProps({});
    }, 200);
  }, []);

  // Memoize the context value to prevent unnecessary re-renders
  const contextValue = useMemo(
    () => ({
      isOpen,
      currentProvider,
      modalProps,
      openModal,
      closeModal,
    }),
    [isOpen, currentProvider, modalProps, openModal, closeModal]
  );

  return (
    <ProviderModalContext.Provider value={contextValue}>{children}</ProviderModalContext.Provider>
  );
};
