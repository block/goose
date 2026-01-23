import { createContext, useContext, useState, useEffect, ReactNode } from 'react'
import { GoosedClient } from '@goosed/sdk'

interface GoosedContextType {
    client: GoosedClient
    isConnected: boolean
    error: string | null
}

const GoosedContext = createContext<GoosedContextType | null>(null)

interface GoosedProviderProps {
    children: ReactNode
}

// Default configuration - can be customized via environment variables or UI
const DEFAULT_BASE_URL = import.meta.env.VITE_GOOSED_BASE_URL || 'http://127.0.0.1:3000'
const DEFAULT_SECRET_KEY = import.meta.env.VITE_GOOSED_SECRET_KEY || 'test'

export function GoosedProvider({ children }: GoosedProviderProps) {
    const [isConnected, setIsConnected] = useState(false)
    const [error, setError] = useState<string | null>(null)

    // Create client instance
    const [client] = useState(() => new GoosedClient({
        baseUrl: DEFAULT_BASE_URL,
        secretKey: DEFAULT_SECRET_KEY,
        timeout: 30000,
    }))

    // Check connection on mount
    useEffect(() => {
        const checkConnection = async () => {
            try {
                const status = await client.status()
                if (status === 'ok') {
                    setIsConnected(true)
                    setError(null)
                }
            } catch (err) {
                setIsConnected(false)
                setError(err instanceof Error ? err.message : 'Failed to connect to goosed server')
                console.error('Goosed connection error:', err)
            }
        }

        checkConnection()

        // Periodically check connection
        const interval = setInterval(checkConnection, 30000)
        return () => clearInterval(interval)
    }, [client])

    return (
        <GoosedContext.Provider value={{ client, isConnected, error }}>
            {children}
        </GoosedContext.Provider>
    )
}

export function useGoosed(): GoosedContextType {
    const context = useContext(GoosedContext)
    if (!context) {
        throw new Error('useGoosed must be used within a GoosedProvider')
    }
    return context
}
