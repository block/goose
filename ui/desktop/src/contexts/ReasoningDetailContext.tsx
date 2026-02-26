import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
} from 'react';
import type { Message } from '@/api';

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
  showAgentBadge?: boolean;
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
  // Ref mirrors panelDetail so callbacks stay stable (no panelDetail in deps)
  const panelDetailRef = useRef<PanelDetail | null>(null);

  const openDetail = useCallback((newDetail: ReasoningDetail) => {
    setDetail(newDetail);
    const pd: PanelDetail = { type: 'reasoning', data: newDetail };
    setPanelDetail(pd);
    panelDetailRef.current = pd;
    setIsOpen(true);
    isOpenRef.current = true;
  }, []);

  const closeDetail = useCallback(() => {
    setIsOpen(false);
    isOpenRef.current = false;
    setTimeout(() => {
      setDetail(null);
      setPanelDetail(null);
      panelDetailRef.current = null;
    }, 300);
  }, []);

  const toggleDetail = useCallback(
    (newDetail: ReasoningDetail) => {
      if (isOpenRef.current && panelDetailRef.current?.data.messageId === newDetail.messageId) {
        closeDetail();
      } else {
        openDetail(newDetail);
      }
    },
    [openDetail, closeDetail]
  );

  const toggleWorkBlock = useCallback(
    (workBlock: WorkBlockDetail) => {
      const cur = panelDetailRef.current;
      if (
        isOpenRef.current &&
        cur?.type === 'workblock' &&
        cur.data.messageId === workBlock.messageId
      ) {
        closeDetail();
      } else {
        setDetail(null);
        const pd: PanelDetail = { type: 'workblock', data: workBlock };
        setPanelDetail(pd);
        panelDetailRef.current = pd;
        setIsOpen(true);
        isOpenRef.current = true;
      }
    },
    [closeDetail]
  );

  const updateContent = useCallback((content: string) => {
    setDetail((prev) => (prev ? { ...prev, content } : prev));
  }, []);

  const updateWorkBlock = useCallback((workBlock: WorkBlockDetail) => {
    setPanelDetail((prev) => {
      if (prev?.type === 'workblock' && prev.data.messageId === workBlock.messageId) {
        // Compare by value — messages array is recreated every render by the parent
        // (.map() in ProgressiveMessageList), so reference equality always fails.
        // Length + toolCount + isStreaming are sufficient change detectors during streaming.
        const d = prev.data;
        if (
          d.messages.length === workBlock.messages.length &&
          d.toolCount === workBlock.toolCount &&
          d.isStreaming === workBlock.isStreaming
        ) {
          return prev; // same content → no re-render
        }
        const pd: PanelDetail = { type: 'workblock', data: workBlock };
        panelDetailRef.current = pd;
        return pd;
      }
      return prev;
    });
  }, []);

  const value = useMemo(
    () => ({
      detail,
      panelDetail,
      isOpen,
      openDetail,
      closeDetail,
      toggleDetail,
      toggleWorkBlock,
      updateContent,
      updateWorkBlock,
    }),
    [
      detail,
      panelDetail,
      isOpen,
      openDetail,
      closeDetail,
      toggleDetail,
      toggleWorkBlock,
      updateContent,
      updateWorkBlock,
    ]
  );

  return (
    <ReasoningDetailContext.Provider value={value}>{children}</ReasoningDetailContext.Provider>
  );
}
