import 'react-toastify/dist/ReactToastify.css';

import { ChatType } from '../types/chat';
import BaseChat2 from './BaseChat2';

interface PairProps {
  setChat: (chat: ChatType) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
  sessionId: string;
  initialMessage?: string;
  // Matrix integration props
  showParticipantsBar?: boolean;
  matrixRoomId?: string;
  showPendingInvites?: boolean;
  // Additional props for flexibility
  loadingChat?: boolean;
  showPopularTopics?: boolean;
}

export default function Pair({
  setChat,
  setIsGoosehintsModalOpen,
  sessionId,
  initialMessage,
  showParticipantsBar = false,
  matrixRoomId,
  showPendingInvites = false,
  loadingChat = false,
  showPopularTopics = true,
}: PairProps) {
  return (
    <BaseChat2
      setChat={setChat}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
      sessionId={sessionId}
      initialMessage={initialMessage}
      suppressEmptyState={false}
      showParticipantsBar={showParticipantsBar}
      matrixRoomId={matrixRoomId}
      showPendingInvites={showPendingInvites}
      loadingChat={loadingChat}
      showPopularTopics={showPopularTopics}
    />
  );
}
