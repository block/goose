import { useState, useEffect, useRef, useCallback } from 'react'

interface UIResource {
    uri: string
    mimeType?: string
    text?: string
    blob?: string
}

interface UIResourceRendererProps {
    resource: UIResource
}

/**
 * Extract Mermaid code from HTML content
 */
function extractMermaidCode(html: string): string | null {
    // Match content inside <div class="mermaid">...</div>
    const match = html.match(/<div[^>]*class="mermaid"[^>]*>([\s\S]*?)<\/div>/i)
    if (match && match[1]) {
        return match[1].trim()
    }
    return null
}

/**
 * Check if this is a Mermaid visualization
 */
function isMermaidVisualization(uri: string, html: string): boolean {
    return uri.includes('mermaid') || html.includes('class="mermaid"')
}

/**
 * Create a clean HTML page with CDN Mermaid.js
 */
function createCleanMermaidHtml(mermaidCode: string): string {
    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mermaid Diagram</title>
    <script src="https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.min.js"></script>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #ffffff;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            box-sizing: border-box;
        }
        .mermaid {
            max-width: 100%;
        }
        .container {
            text-align: center;
            width: 100%;
        }
        h1 {
            font-size: 1.5em;
            font-weight: 300;
            color: #333;
            margin-bottom: 20px;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>Mermaid Diagram</h1>
        <div class="mermaid">
${mermaidCode}
        </div>
    </div>
    <script>
        mermaid.initialize({
            startOnLoad: true,
            theme: 'default',
            flowchart: { useMaxWidth: true, htmlLabels: true }
        });
    </script>
</body>
</html>`
}

/**
 * Lightweight UI Resource Renderer
 * Renders MCP-UI resources (visualizations) in an iframe
 * No external dependencies - self-contained implementation
 */
export default function UIResourceRenderer({ resource }: UIResourceRendererProps) {
    const [htmlContent, setHtmlContent] = useState<string | null>(null)
    const [error, setError] = useState<string | null>(null)
    const iframeRef = useRef<HTMLIFrameElement>(null)

    useEffect(() => {
        try {
            let decoded = ''

            // Decode the HTML content from base64 blob or text
            if (resource.blob) {
                // Base64 decode
                decoded = atob(resource.blob)
            } else if (resource.text) {
                decoded = resource.text
            } else {
                setError('No content available')
                return
            }

            // Check if this is a Mermaid visualization with broken inline script
            if (isMermaidVisualization(resource.uri, decoded)) {
                const mermaidCode = extractMermaidCode(decoded)
                if (mermaidCode) {
                    // Create clean HTML with CDN Mermaid instead of using broken template
                    console.log('Creating clean Mermaid HTML with CDN...')
                    decoded = createCleanMermaidHtml(mermaidCode)
                }
            }

            setHtmlContent(decoded)
        } catch (e) {
            setError(`Failed to decode content: ${e instanceof Error ? e.message : 'Unknown error'}`)
        }
    }, [resource])

    // Handle iframe load for auto-resize
    const handleIframeLoad = useCallback(() => {
        const iframe = iframeRef.current
        if (!iframe) return

        // Auto-resize after content loads
        const resizeIframe = () => {
            try {
                const doc = iframe.contentDocument || iframe.contentWindow?.document
                if (doc) {
                    const height = doc.documentElement.scrollHeight || doc.body.scrollHeight
                    iframe.style.height = `${Math.min(height + 20, 800)}px`
                }
            } catch {
                // Cross-origin restrictions may prevent access
            }
        }

        // Resize after initial load and after a delay for async rendering
        resizeIframe()
        setTimeout(resizeIframe, 500)
        setTimeout(resizeIframe, 1500)
    }, [])

    if (error) {
        return (
            <div className="ui-resource-error" style={{
                padding: '12px',
                background: 'var(--color-bg-tertiary, #f5f5f5)',
                borderRadius: '8px',
                color: 'var(--color-text-muted, #666)',
                fontSize: '14px'
            }}>
                ⚠️ {error}
            </div>
        )
    }

    if (!htmlContent) {
        return (
            <div className="ui-resource-loading" style={{
                padding: '12px',
                textAlign: 'center',
                color: 'var(--color-text-muted, #666)'
            }}>
                Loading visualization...
            </div>
        )
    }

    return (
        <div className="ui-resource-container" style={{
            marginTop: '12px',
            border: '1px solid var(--color-border, #e0e0e0)',
            borderRadius: '8px',
            overflow: 'hidden',
            background: 'white'
        }}>
            <iframe
                ref={iframeRef}
                srcDoc={htmlContent}
                onLoad={handleIframeLoad}
                sandbox="allow-scripts"
                style={{
                    width: '100%',
                    height: '400px',
                    border: 'none',
                    display: 'block'
                }}
                title="Visualization"
            />
        </div>
    )
}

/**
 * Check if a content item is a UI resource
 * UI resources have uri starting with "ui://"
 */
export function isUIResource(content: unknown): content is { resource: UIResource } {
    if (!content || typeof content !== 'object') return false

    const obj = content as Record<string, unknown>
    if (!obj.resource || typeof obj.resource !== 'object') return false

    const resource = obj.resource as Record<string, unknown>
    if (typeof resource.uri !== 'string') return false

    return resource.uri.startsWith('ui://')
}
