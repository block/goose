import React from 'react';
import GooseLogo from './GooseLogo';

const LoadingGoose = () => {
  return (
    <div className="fixed bottom-[115px] left-0 right-0 w-full z-50">
      <div className="flex items-center text-xs text-textStandard pl-4 animate-[appear_250ms_ease-in_forwards]">
        <GooseLogo className="mr-2" size="small" hover={false} />
        goose is working on it…
      </div>
    </div>
  );
};

export default LoadingGoose;