import React from 'react';
import GooseLogo from './components/GooseLogo';

export default function SuspenseLoader() {
  return (
    <div className="flex flex-col gap-4 items-center justify-center w-screen h-screen overflow-hidden bg-bgApp text-textProminent">
      <GooseLogo />
      <span className="text-lg">Loading...</span>
    </div>
  );
}
