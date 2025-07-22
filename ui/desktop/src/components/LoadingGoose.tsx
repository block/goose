import GooseLogo from './GooseLogo';
import ThinkingIcons from './ThinkingIcons';
import FlyingBird from './FlyingBird';
import { ChatState } from '../types/chatState';

interface LoadingGooseProps {
  message?: string;
  chatState?: ChatState;
}

const LoadingGoose = ({ message, chatState = ChatState.Idle }: LoadingGooseProps) => {
  // Determine the appropriate message based on state
  const getLoadingMessage = () => {
    if (message) return message; // Custom message takes priority

    if (chatState === ChatState.Waiting) return 'goose is thinking…';
    if (chatState === ChatState.Streaming) return 'goose is working on it…';

    // Default fallback
    return 'goose is working on it…';
  };

  return (
    <div className="w-full animate-fade-slide-up">
      <div
        data-testid="loading-indicator"
        className="flex items-center gap-2 text-xs text-textStandard py-2"
      >
        {chatState === ChatState.Waiting ? (
          <ThinkingIcons className="flex-shrink-0" cycleInterval={600} />
        ) : chatState === ChatState.Streaming ? (
          <FlyingBird className="flex-shrink-0" cycleInterval={150} />
        ) : (
          <GooseLogo size="small" hover={false} />
        )}
        {getLoadingMessage()}
      </div>
    </div>
  );
};

export default LoadingGoose;
