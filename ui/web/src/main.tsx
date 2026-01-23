import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import App from './App'
import { GoosedProvider } from './contexts/GoosedContext'
import ErrorBoundary from './components/ErrorBoundary'
import './App.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
        <ErrorBoundary>
            <BrowserRouter>
                <GoosedProvider>
                    <App />
                </GoosedProvider>
            </BrowserRouter>
        </ErrorBoundary>
    </React.StrictMode>,
)

