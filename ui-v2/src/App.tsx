import React, { Suspense } from 'react';

import { Outlet } from '@tanstack/react-router';

import SuspenseLoader from './components/SuspenseLoader';

const App: React.FC = (): React.ReactElement => {
  return (
    <div>
      <div className="titlebar-drag-region" />
      <div className="relative w-screen h-screen overflow-hidden bg-bgApp flex flex-col">
        <div className="relative flex items-center h-14 border-b border-borderSubtle w-full" />
        <div className="pt-14">
          <Suspense fallback={<SuspenseLoader />}>
            <Outlet />
          </Suspense>
        </div>
      </div>
    </div>
  );
};

export default App;
