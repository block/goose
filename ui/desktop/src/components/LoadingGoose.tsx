import GooseLogo from './GooseLogo';
import ThinkingIcons from './ThinkingIcons';
import FlyingBird from './FlyingBird';

interface LoadingGooseProps {
  message?: string;
  isWaiting?: boolean;
  isStreaming?: boolean;
}

const LoadingGoose = ({ 
  message, 
  isWaiting = false, 
  isStreaming = false 
}: LoadingGooseProps) => {
  // Determine the appropriate message based on state
  const getLoadingMessage = () => {
    if (message) return message; // Custom message takes priority
    
    if (isWaiting) return 'goose is thinking…';
    if (isStreaming) return 'goose is working on it…';
    
    // Default fallback
    return 'goose is working on it…';
  };

  return (
    <div className="w-full animate-fade-slide-up">
      <div
        data-testid="loading-indicator"
        className="flex items-center gap-2 text-xs text-textStandard py-2"
      >
        {isWaiting ? (
          <ThinkingIcons className="flex-shrink-0" cycleInterval={600} />
        ) : isStreaming ? (
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
