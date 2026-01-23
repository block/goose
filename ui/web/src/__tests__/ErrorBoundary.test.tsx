import { render, screen } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import ErrorBoundary from '../components/ErrorBoundary'
import React from 'react'

// Component that throws an error
const ThrowError = () => {
    throw new Error('Test error')
}

describe('ErrorBoundary', () => {
    it('renders children when no error occurs', () => {
        render(
            <ErrorBoundary>
                <div>Safe Content</div>
            </ErrorBoundary>
        )
        expect(screen.getByText('Safe Content')).toBeInTheDocument()
    })

    it('renders error UI when an error occurs', () => {
        // Suppress console.error for this test as we expect an error
        const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => { })

        render(
            <ErrorBoundary>
                <ThrowError />
            </ErrorBoundary>
        )

        expect(screen.getByText('Something went wrong')).toBeInTheDocument()
        expect(screen.getByText('Test error')).toBeInTheDocument()

        consoleSpy.mockRestore()
    })
})
