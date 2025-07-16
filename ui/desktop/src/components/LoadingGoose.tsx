import GooseLogo from './GooseLogo';

interface LoadingGooseProps {
  message?: string;
}

const LoadingGoose = ({ message = 'goose is working on it…' }: LoadingGooseProps) => {
  return (
    <div className="w-full animate-fade-slide-up">
      <div
        data-testid="loading-indicator"
        className="flex items-center gap-2 text-xs text-textStandard py-2"
      >
        <GooseLogo size="small" hover={false} />
        {message}
      </div>
    </div>
  );
};

export default LoadingGoose;
