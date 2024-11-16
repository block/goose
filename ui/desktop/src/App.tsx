import React, { useState } from 'react';
import { Route, Routes, Navigate } from 'react-router-dom';
import Chat from './Chat';

export default function App() {
  const initialChats = [
    { id: 1, title: 'Chat 1', messages: [] },
  ]

  const [chats, setChats] = useState(initialChats)
  const [selectedChatId, setSelectedChatId] = useState(1)

  return (
    <div className="relative w-screen h-screen overflow-hidden bg-gradient-to-b from-white to-gray-50 flex flex-col">
      <Routes>
        <Route
          path="/chat/:id"
          element={<Chat key={selectedChatId} chats={chats} setChats={setChats} selectedChatId={selectedChatId} setSelectedChatId={setSelectedChatId} />}
        />
        <Route path="*" element={<Navigate to="/chat/1" replace />} />
      </Routes>
    </div>
  );
}