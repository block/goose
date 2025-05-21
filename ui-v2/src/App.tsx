import React, { Suspense } from 'react';

import { Outlet } from '@tanstack/react-router';

import SuspenseLoader from './components/SuspenseLoader';

const App: React.FC = (): React.ReactElement => {
  return (
    <Suspense fallback={<SuspenseLoader />}>
      <Outlet />
    </Suspense>
  );
};

export default App;
