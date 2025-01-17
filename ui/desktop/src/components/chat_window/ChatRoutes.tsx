import React from "react";
import { Routes, Route, Navigate } from "react-router-dom";
import { ChatContent } from "../../ChatWindow"
import Settings  from "../settings/Settings"
import Keys from "../settings/Keys"

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
                    key={selectedChatId}
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
        <Route path="/keys" element={<Keys />} />
        <Route path="*" element={<Navigate to="/chat/1" replace />} />
    </Routes>
);
