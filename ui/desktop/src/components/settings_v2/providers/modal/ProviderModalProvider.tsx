import React, { createContext, useContext, useState } from 'react';
import ProviderState from '../interfaces/ProviderState';

interface ProviderModalContextType {
  isOpen: boolean;
  currentProvider: ProviderState | null;
  modalProps: any;
  openModal: (provider: ProviderState, additionalProps: any) => void;
  closeModal: () => void;
}

const ProviderModalContext = createContext({
  isOpen: false,
  currentProvider: null,
  modalProps: {},
  openModal: (provider, additionalProps) => {},
  closeModal: () => {},
});

export const useProviderModal = () => useContext<ProviderModalContextType>(ProviderModalContext);

export const ProviderModalProvider = ({ children }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [currentProvider, setCurrentProvider] = useState(null);
  const [modalProps, setModalProps] = useState({});

  const openModal = (provider, additionalProps = {}) => {
    setCurrentProvider(provider);
    setModalProps(additionalProps);
    setIsOpen(true);
  };

  const closeModal = () => {
    setIsOpen(false);
    // Use a small timeout to prevent UI flicker
    setTimeout(() => {
      setCurrentProvider(null);
      setModalProps({});
    }, 200);
  };

  return (
    <ProviderModalContext.Provider
      value={{
        isOpen,
        currentProvider,
        modalProps,
        openModal,
        closeModal,
      }}
    >
      {children}
    </ProviderModalContext.Provider>
  );
};
