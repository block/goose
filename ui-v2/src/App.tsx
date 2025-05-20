import React, { Suspense } from 'react';

import { Outlet } from '@tanstack/react-router';

import GooseLogo from './components/GooseLogo';
import SuspenseLoader from './components/SuspenseLoader';

const App: React.FC = (): React.ReactElement => {
  return (
    <Suspense fallback={<SuspenseLoader />}>
      <div className="p-5 max-w-3xl mx-auto">
        <div className="flex items-center gap-4 mb-4">
          <GooseLogo />
          <h1 className="text-2xl font-bold text-textProminent">Goose v2</h1>
        </div>
        <Outlet />
      </div>
    </Suspense>
  );
};

export default App;
