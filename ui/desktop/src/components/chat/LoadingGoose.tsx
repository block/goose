import GooseLogo from '../branding/GooseLogo';
import AnimatedIcons from '../branding/AnimatedIcons';
import FlyingBird from '../branding/FlyingBird';
import { ChatState } from '../../types/chatState';
import type { RoutingInfo } from '../../types/message';

interface LoadingGooseProps {
  message?: string;
  chatState?: ChatState;
  routingInfo?: RoutingInfo;
}

const STATE_MESSAGES: Record<ChatState, string> = {
  [ChatState.LoadingConversation]: 'loading conversation...',
  [ChatState.Thinking]: 'goose is thinking…',
  [ChatState.Streaming]: 'goose is working on it…',
  [ChatState.WaitingForUserInput]: 'goose is waiting…',
  [ChatState.Compacting]: 'goose is compacting the conversation...',
  [ChatState.Idle]: 'goose is working on it…',
  [ChatState.RestartingAgent]: 'restarting session...',
};

const STATE_ICONS: Record<ChatState, React.ReactNode> = {
  [ChatState.LoadingConversation]: <AnimatedIcons className="flex-shrink-0" cycleInterval={600} />,
  [ChatState.Thinking]: <AnimatedIcons className="flex-shrink-0" cycleInterval={600} />,
  [ChatState.Streaming]: <FlyingBird className="flex-shrink-0" cycleInterval={150} />,
  [ChatState.WaitingForUserInput]: (
    <AnimatedIcons className="flex-shrink-0" cycleInterval={600} variant="waiting" />
  ),
  [ChatState.Compacting]: <AnimatedIcons className="flex-shrink-0" cycleInterval={600} />,
  [ChatState.Idle]: <GooseLogo size="small" hover={false} />,
  [ChatState.RestartingAgent]: <AnimatedIcons className="flex-shrink-0" cycleInterval={600} />,
};

const LoadingGoose = ({ message, chatState = ChatState.Idle, routingInfo }: LoadingGooseProps) => {
  const displayMessage = message || STATE_MESSAGES[chatState];
  const icon = STATE_ICONS[chatState];

  const agentLabel =
    routingInfo && routingInfo.agentName !== 'Goose Agent'
      ? `${routingInfo.agentName}${routingInfo.modeSlug ? ` · ${routingInfo.modeSlug}` : ''}`
      : null;

  return (
    <div className="w-full animate-fade-slide-up">
      <div
        data-testid="loading-indicator"
        className="flex items-center gap-2 text-xs text-text-default py-2"
      >
        {icon}
        <span>
          {displayMessage}
          {agentLabel && (
            <span className="ml-1 text-text-muted/70 italic">({agentLabel})</span>
          )}
        </span>
      </div>
    </div>
  );
};

export default LoadingGoose;
