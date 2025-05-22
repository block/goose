import React, { Suspense } from 'react';

import { Outlet } from '@tanstack/react-router';

import SuspenseLoader from './components/SuspenseLoader';

const App: React.FC = (): React.ReactElement => {
  return (
    <div className="">
      <div className="titlebar-drag-region" />
      <div className="h-10 w-full" />
      <div className="">
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
