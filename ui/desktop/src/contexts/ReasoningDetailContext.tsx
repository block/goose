import { createContext, useContext, useState, useCallback, useRef, ReactNode } from 'react';

interface ReasoningDetail {
  title: string;
  content: string;
  messageId?: string;
}

interface ReasoningDetailContextType {
  detail: ReasoningDetail | null;
  isOpen: boolean;
  openDetail: (detail: ReasoningDetail) => void;
  closeDetail: () => void;
  toggleDetail: (detail: ReasoningDetail) => void;
  updateContent: (content: string) => void;
}

const ReasoningDetailContext = createContext<ReasoningDetailContextType | null>(null);

export function useReasoningDetail() {
  const context = useContext(ReasoningDetailContext);
  if (!context) {
    throw new Error('useReasoningDetail must be used within a ReasoningDetailProvider');
  }
  return context;
}

export function ReasoningDetailProvider({ children }: { children: ReactNode }) {
  const [detail, setDetail] = useState<ReasoningDetail | null>(null);
  const [isOpen, setIsOpen] = useState(false);
  const isOpenRef = useRef(false);

  const openDetail = useCallback((newDetail: ReasoningDetail) => {
    setDetail(newDetail);
    setIsOpen(true);
    isOpenRef.current = true;
  }, []);

  const closeDetail = useCallback(() => {
    setIsOpen(false);
    isOpenRef.current = false;
    setTimeout(() => setDetail(null), 300);
  }, []);

  const toggleDetail = useCallback(
    (newDetail: ReasoningDetail) => {
      if (isOpenRef.current && detail?.messageId === newDetail.messageId) {
        closeDetail();
      } else {
        openDetail(newDetail);
      }
    },
    [detail?.messageId, openDetail, closeDetail]
  );

  const updateContent = useCallback((content: string) => {
    setDetail((prev) => (prev ? { ...prev, content } : prev));
  }, []);

  return (
    <ReasoningDetailContext.Provider
      value={{ detail, isOpen, openDetail, closeDetail, toggleDetail, updateContent }}
    >
      {children}
    </ReasoningDetailContext.Provider>
  );
}
