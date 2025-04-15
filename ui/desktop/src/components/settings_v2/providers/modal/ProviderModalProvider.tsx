import React, { createContext, useContext, useState } from 'react';
import { ProviderDetails } from '../../../../api/types.gen';

interface FormValues {
  [key: string]: string | number | boolean | null;
}

interface ModalProps {
  onSubmit?: (values: FormValues) => void;
  onCancel?: () => void;
  onDelete?: (values: FormValues) => void;
  formProps?: {
    initialValues?: FormValues;
    validationSchema?: object;
    [key: string]: unknown;
  };
}

interface ProviderModalContextType {
  isOpen: boolean;
  currentProvider: ProviderDetails | null;
  modalProps: ModalProps;
  openModal: (provider: ProviderDetails, additionalProps?: ModalProps) => void;
  closeModal: () => void;
}

const ProviderModalContext = createContext<ProviderModalContextType | undefined>(undefined);

export const ProviderModalProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [currentProvider, setCurrentProvider] = useState<ProviderDetails | null>(null);
  const [modalProps, setModalProps] = useState<ModalProps>({});

  const openModal = (provider: ProviderDetails, additionalProps: ModalProps = {}) => {
    setCurrentProvider(provider);
    setModalProps(additionalProps);
    setIsOpen(true);
  };

  const closeModal = () => {
    setIsOpen(false);
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

export const useProviderModal = () => {
  const context = useContext(ProviderModalContext);
  if (context === undefined) {
    throw new Error('useProviderModal must be used within a ProviderModalProvider');
  }
  return context;
};
