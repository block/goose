import React, { useEffect, useState } from 'react';
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
    <div className="fixed inset-0 flex items-center justify-center z-50 bg-white">
      <div className="flex flex-col items-center max-w-3xl w-full px-6 pt-10">
        <div className="w-12 h-12 bg-white rounded-full flex items-center justify-center mb-4">
          <Geese className="w-5 h-5" />
        </div>
        <h1 className="text-2xl font-medium text-center mb-2">Laying an egg{dots}</h1>
        <p className="text-gray-500 text-center text-sm">
          Please wait while we process your request
        </p>
      </div>
    </div>
  );
}
