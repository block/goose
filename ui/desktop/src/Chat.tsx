import React, { useEffect } from 'react'
import { useChat } from 'ai/react'
import { useNavigate } from 'react-router-dom'
import { getApiUrl } from './config'
import { X, Plus, Send } from 'lucide-react'
import { Button } from './components/ui/button'
import { Card } from './components/ui/card'
import { Input } from './components/ui/input'
import { ScrollArea } from './components/ui/scroll-area'
import GooseSplashLogo from './components/GooseSplashLogo'
import GooseMessage from './components/GooseMessage'
import UserMessage from './components/UserMessage'

export interface Chat {
  id: number;
  title: string;
  messages: Array<{ id: string; role: any; content: string }>;
}

export default function Chat({ chats, setChats, selectedChatId, setSelectedChatId } : { chats: Chat[], setChats: any, selectedChatId: number, setSelectedChatId: any }) {
  const navigate = useNavigate()
  const chat = chats.find((c: Chat) => c.id === selectedChatId);

  const { messages, input, handleInputChange, handleSubmit } = useChat({
    api: getApiUrl("/reply"),
    initialMessages: chat.messages
  })

  const useChatData = { messages, input, handleInputChange, handleSubmit };

  useEffect(() => {
    const updatedChats = [...chats]
    updatedChats.find((c) => c.id === selectedChatId).messages = messages
    setChats(updatedChats)
  }, [messages, selectedChatId])

  const navigateChat = (chatId: number) => {
    setSelectedChatId(chatId)
    navigate(`/chat/${chatId}`)
  }

  const addChat = () => {
    const newChatId = chats[chats.length-1].id + 1;
    const newChat = {
      id: newChatId,
      title: `Chat ${newChatId}`,
      messages: [],
    };
    setChats([...chats, newChat]);
    navigateChat(newChatId);
  };

  const removeChat = (chatId: number) => {
    const updatedChats = chats.filter((chat: any) => chat.id !== chatId);
    setChats(updatedChats);
    navigateChat(updatedChats[0].id);
  };

  return (
    <div className="min-h-screen w-full bg-window-gradient flex flex-col items-center justify-center p-0">
      <div className="flex items-center justify-start overflow-x-auto relative p-[10px] pb-0 w-full">
        {chats.map((chat) => (
          <div
            key={chat.id}
            className={`flex bg-tab items-center min-w-[140px] max-w-[240px] h-8 px-3 mr-1 rounded-t-lg cursor-pointer transition-all`}
            onClick={() => navigateChat(chat.id)}
            onKeyDown={(e) => e.key === "Enter" && navigateChat(chat.id)}
            tabIndex={0}
            role="tab"
            aria-selected={selectedChatId === chat.id}
          >
            <span className="flex-grow truncate text-sm font-medium">{chat.title}</span>
            {chats.length > 1 && (
              <button
                className="ml-2 p-1 rounded-full hover:bg-sky-100 focus:outline-none focus:ring-2 focus:ring-sky-400"
                onClick={(e) => {
                  e.stopPropagation();
                  removeChat(chat.id);
                }}
                aria-label={`Close ${chat.title} chat`}
              >
                <X className="w-3 h-3" />
              </button>
            )}
          </div>
        ))}
        <button
          className="flex items-center justify-center w-8 h-8 rounded-full bg-transparent hover:bg-sky-200 focus:outline-none focus:ring-2 focus:ring-sky-400"
          onClick={addChat}
          aria-label="New chat"
        >
          <Plus className="w-5 h-5 text-sky-600" />
        </button>
      </div>

      <Card className="w-[calc(100%-20px)] h-[calc(100vh-20px)] m-[10px] bg-card-gradient mt-0 backdrop-blur-sm border-none shadow-xl rounded-2xl rounded-tl-none overflow-hidden">
        <div className="flex flex-col h-full">
          <ScrollArea className="flex-1 p-4">
            <div className="space-y-4">
              {messages.length === 0 ? (
                <div className="flex items-center justify-center h-screen">
                  <div className="flex items-center space-x-4">
                    <GooseSplashLogo />
                    <span className="ask-goose-type ml-[8px]">ask<br />goose</span>
                  </div>
                </div>
              ) : (
                messages.map((message) => (
                  <div key={message.id}>
                    {message.role === 'user' ? (
                      <UserMessage message={message} />
                    ) : (
                      <GooseMessage message={message} useChatData={useChatData} />
                    )}
                  </div>
                ))
              )}
            </div>
          </ScrollArea>
          <form onSubmit={handleSubmit} className="relative bg-white mb-0 h-{57px}">
            <Input
              placeholder="What should goose do?"
              value={input}
              onChange={handleInputChange}
              className="pr-12 rounded-full border-none focus:outline-none focus:ring-0"
            />
            <Button
              type="submit"
              size="icon"
              variant="ghost"
              className="absolute right-2 top-1/2 -translate-y-1/2 text-indigo-600 hover:text-indigo-700 hover:bg-indigo-100"
            >
              <Send className="h-5 w-5" />
            </Button>
          </form>
        </div>
      </Card>
    </div>
  )
}