import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import App from '../App'
import { BrowserRouter } from 'react-router-dom'
import { GoosedProvider } from '../contexts/GoosedContext'

describe('App', () => {
    it('renders without crashing', () => {
        render(
            <BrowserRouter>
                <GoosedProvider>
                    <App />
                </GoosedProvider>
            </BrowserRouter>
        )
        // Sidebar should always be present
        expect(screen.getByText('Goose')).toBeInTheDocument()
        expect(screen.getByText('Home')).toBeInTheDocument()
        expect(screen.getByText('History')).toBeInTheDocument()
    })
})
