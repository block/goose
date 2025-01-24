import React from 'react';
import { useNavigate } from 'react-router-dom';
import { FaArrowLeft } from 'react-icons/fa';

export default function Header() {
  const navigate = useNavigate();

  return (
    <div className="flex items-center mb-6">
      <button
        onClick={() => navigate(-1)}
        className="mr-4 p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-colors"
        title="Go back"
      >
        <FaArrowLeft className="text-gray-500 dark:text-gray-400" />
      </button>
      <h1 className="text-2xl font-semibold dark:text-white flex-1">Providers</h1>
    </div>
  );
}
