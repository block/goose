import { useEffect, useState } from 'react';
import { Geese } from './icons/Geese';

export default function LayingEggLoader() {
  const [dots, setDots] = useState('');

  useEffect(() => {
    const interval = setInterval(() => {
      setDots((prev) => (prev.length >= 3 ? '' : prev + '.'));
    }, 500);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="fixed inset-0 flex items-center justify-center z-50 bg-background-default">
      <div className="flex flex-col items-center max-w-3xl w-full px-6 pt-10">
        <div className="w-16 h-16 bg-background-default rounded-full flex items-center justify-center mb-4">
          <Geese className="w-12 h-12 text-iconProminent" />
        </div>
        <h1 className="text-2xl font-medium text-center mb-2 text-textProminent">
          Laying an egg{dots}
        </h1>
        <p className="text-textSubtle text-center text-sm">
          Please wait while we process your request
        </p>
      </div>
    </div>
  );
}
