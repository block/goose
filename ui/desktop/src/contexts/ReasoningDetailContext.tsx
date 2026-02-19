import { createContext, type ReactNode, useCallback, useContext, useRef, useState } from 'react';
import type { Message } from '../api';

export interface ReasoningDetail {
  title: string;
  content: string;
  messageId: string;
}

export interface WorkBlockDetail {
  title: string;
  messageId: string;
  messages: Message[];
  toolCount: number;
  isStreaming?: boolean;
  agentName?: string;
  modeName?: string;
  sessionId?: string;
  toolCallNotifications?: Map<string, unknown[]>;
}

type PanelDetail =
  | { type: 'reasoning'; data: ReasoningDetail }
  | { type: 'workblock'; data: WorkBlockDetail };

interface ReasoningDetailContextType {
  detail: ReasoningDetail | null;
  panelDetail: PanelDetail | null;
  isOpen: boolean;
  openDetail: (detail: ReasoningDetail) => void;
  closeDetail: () => void;
  toggleDetail: (detail: ReasoningDetail) => void;
  toggleWorkBlock: (detail: WorkBlockDetail) => void;
  updateContent: (content: string) => void;
  updateWorkBlock: (detail: WorkBlockDetail) => void;
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
  const [panelDetail, setPanelDetail] = useState<PanelDetail | null>(null);
  const [isOpen, setIsOpen] = useState(false);
  const isOpenRef = useRef(false);

  const openDetail = useCallback((newDetail: ReasoningDetail) => {
    setDetail(newDetail);
    setPanelDetail({ type: 'reasoning', data: newDetail });
    setIsOpen(true);
    isOpenRef.current = true;
  }, []);

  const closeDetail = useCallback(() => {
    setIsOpen(false);
    isOpenRef.current = false;
    setTimeout(() => {
      setDetail(null);
      setPanelDetail(null);
    }, 300);
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

  const toggleWorkBlock = useCallback(
    (workBlock: WorkBlockDetail) => {
      if (
        isOpenRef.current &&
        panelDetail?.type === 'workblock' &&
        panelDetail.data.messageId === workBlock.messageId
      ) {
        closeDetail();
      } else {
        setDetail(null);
        setPanelDetail({ type: 'workblock', data: workBlock });
        setIsOpen(true);
        isOpenRef.current = true;
      }
    },
    [panelDetail, closeDetail]
  );

  const updateContent = useCallback((content: string) => {
    setDetail((prev) => (prev ? { ...prev, content } : prev));
  }, []);

  const updateWorkBlock = useCallback((workBlock: WorkBlockDetail) => {
    setPanelDetail((prev) => {
      if (prev?.type === 'workblock' && prev.data.messageId === workBlock.messageId) {
        return { type: 'workblock', data: workBlock };
      }
      return prev;
    });
  }, []);

  return (
    <ReasoningDetailContext.Provider
      value={{
        detail,
        panelDetail,
        isOpen,
        openDetail,
        closeDetail,
        toggleDetail,
        toggleWorkBlock,
        updateContent,
        updateWorkBlock,
      }}
    >
      {children}
    </ReasoningDetailContext.Provider>
  );
}
