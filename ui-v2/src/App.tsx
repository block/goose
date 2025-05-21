import React, { Suspense } from 'react';

import { Outlet } from '@tanstack/react-router';

import SuspenseLoader from './components/SuspenseLoader';

const App: React.FC = (): React.ReactElement => {
  return (
    <div className="w-screen">
      <div className="titlebar-drag-region" />
      <div className="">
        <div className="h-15 border-b-1 w-full" />
        <div className="">
          <Suspense fallback={<SuspenseLoader />}>
            <Outlet />
          </Suspense>
        </div>
      </div>
    </div>
  );
};

export default App;
