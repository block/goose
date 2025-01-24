import React from 'react';
import { Routes, Route, Navigate } from 'react-router-dom';
import { ChatContent } from '../../ChatWindow';
import Settings from '../settings/Settings';
import MoreModelsSettings from '../settings/models/MoreModels';
import ConfigureProviders from '../settings/providers/ConfigureProviders';
import { WelcomeScreen } from '../welcome_screen/WelcomeScreen';

export const ChatRoutes = ({
  chats,
  setChats,
  selectedChatId,
  setSelectedChatId,
  setProgressMessage,
  setWorking,
}) => (
  <Routes>
    <Route
      path="/chat/:id"
      element={
        <ChatContent
          chats={chats}
          setChats={setChats}
          selectedChatId={selectedChatId}
          setSelectedChatId={setSelectedChatId}
          initialQuery={null}
          setProgressMessage={setProgressMessage}
          setWorking={setWorking}
        />
      }
    />
    <Route path="/settings" element={<Settings />} />
    <Route path="/settings/more-models" element={<MoreModelsSettings />} />
    <Route path="/settings/configure-providers" element={<ConfigureProviders />} />
    <Route path="/welcome" element={<WelcomeScreen />} />
    <Route path="*" element={<Navigate to="/chat/1" replace />} />
  </Routes>
);
