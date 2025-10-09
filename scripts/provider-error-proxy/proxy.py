#!/usr/bin/env python3
"""
Provider Error Proxy - Simulates provider errors for testing Goose error handling.

This proxy intercepts HTTP traffic to AI providers and can inject errors at specified intervals.
It supports the major providers: OpenAI, Anthropic, Google, OpenRouter, Tetrate, and Databricks.

Usage:
    uv run proxy.py [--port PORT] [--error-interval N]

The proxy will:
1. Listen on localhost:PORT (default 8888)
2. Forward requests to the actual provider endpoints
3. Inject errors every N requests (default 5)

To use with Goose, set the provider host environment variables:
    export OPENAI_HOST=http://localhost:8888
    export ANTHROPIC_HOST=http://localhost:8888
    # etc.
"""

import asyncio
import logging
import sys
from argparse import ArgumentParser
from typing import Optional
from urllib.parse import urlparse

from aiohttp import web, ClientSession, ClientTimeout, ClientResponse
from aiohttp.web import Request, Response, StreamResponse

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Provider endpoint mappings
PROVIDER_HOSTS = {
    'openai': 'https://api.openai.com',
    'anthropic': 'https://api.anthropic.com',
    'google': 'https://generativelanguage.googleapis.com',
    'openrouter': 'https://openrouter.ai',
    'tetrate': 'https://api.tetrate.io',
    'databricks': 'https://api.databricks.com',
}

# Error responses for different providers
ERROR_RESPONSES = {
    'openai': {
        'status': 429,
        'body': {
            'error': {
                'message': 'Rate limit exceeded. Please try again later.',
                'type': 'rate_limit_error',
                'code': 'rate_limit_exceeded'
            }
        }
    },
    'anthropic': {
        'status': 529,
        'body': {
            'error': {
                'type': 'overloaded_error',
                'message': 'The API is temporarily overloaded. Please try again shortly.'
            }
        }
    },
    'google': {
        'status': 503,
        'body': {
            'error': {
                'code': 503,
                'message': 'Service temporarily unavailable',
                'status': 'UNAVAILABLE'
            }
        }
    },
    'openrouter': {
        'status': 429,
        'body': {
            'error': {
                'message': 'Rate limit exceeded',
                'code': 429
            }
        }
    },
    'tetrate': {
        'status': 503,
        'body': {
            'error': {
                'message': 'Service unavailable',
                'code': 'service_unavailable'
            }
        }
    },
    'databricks': {
        'status': 429,
        'body': {
            'error_code': 'RATE_LIMIT_EXCEEDED',
            'message': 'Rate limit exceeded'
        }
    },
}


class ErrorProxy:
    """HTTP proxy that can inject errors into provider responses."""
    
    def __init__(self, error_interval: int = 5):
        """
        Initialize the error proxy.
        
        Args:
            error_interval: Inject an error every N requests (default: 5)
        """
        self.error_interval = error_interval
        self.request_count = 0
        self.session: Optional[ClientSession] = None
        
    async def start_session(self):
        """Start the aiohttp client session."""
        timeout = ClientTimeout(total=600)  # Match provider timeout
        self.session = ClientSession(timeout=timeout)
        
    async def close_session(self):
        """Close the aiohttp client session."""
        if self.session:
            await self.session.close()
            
    def should_inject_error(self) -> bool:
        """Determine if we should inject an error for this request."""
        self.request_count += 1
        should_error = self.request_count % self.error_interval == 0
        if should_error:
            logger.warning(f"🔴 Injecting error on request #{self.request_count}")
        else:
            logger.info(f"✅ Forwarding request #{self.request_count}")
        return should_error
        
    def detect_provider(self, request: Request) -> Optional[str]:
        """
        Detect which provider this request is for based on headers and path.
        
        Args:
            request: The incoming HTTP request
            
        Returns:
            Provider name or None if not detected
        """
        # Check for provider-specific headers
        if 'x-api-key' in request.headers:
            return 'anthropic'
        if 'authorization' in request.headers:
            auth = request.headers['authorization'].lower()
            if 'bearer' in auth:
                # Most providers use bearer tokens, check path for hints
                path = request.path.lower()
                if 'openai' in path or 'chat/completions' in path:
                    return 'openai'
                if 'anthropic' in path or 'messages' in path:
                    return 'anthropic'
                if 'google' in path or 'generativelanguage' in path:
                    return 'google'
                if 'openrouter' in path:
                    return 'openrouter'
                if 'tetrate' in path:
                    return 'tetrate'
                if 'databricks' in path:
                    return 'databricks'
                    
        # Default to openai if we can't determine
        return 'openai'
        
    def get_target_url(self, request: Request, provider: str) -> str:
        """
        Construct the target URL for the provider.
        
        Args:
            request: The incoming HTTP request
            provider: The detected provider name
            
        Returns:
            Full target URL
        """
        base_host = PROVIDER_HOSTS.get(provider, PROVIDER_HOSTS['openai'])
        path = request.path
        query = request.query_string
        
        url = f"{base_host}{path}"
        if query:
            url = f"{url}?{query}"
            
        return url
        
    async def handle_request(self, request: Request) -> Response:
        """
        Handle an incoming HTTP request.
        
        Args:
            request: The incoming HTTP request
            
        Returns:
            HTTP response (either proxied or error)
        """
        provider = self.detect_provider(request)
        logger.info(f"📨 {request.method} {request.path} -> {provider}")
        
        # Check if we should inject an error
        if self.should_inject_error():
            error_config = ERROR_RESPONSES.get(provider, ERROR_RESPONSES['openai'])
            logger.warning(f"💥 Returning {error_config['status']} error for {provider}")
            return web.json_response(
                error_config['body'],
                status=error_config['status']
            )
        
        # Forward the request to the actual provider
        target_url = self.get_target_url(request, provider)
        
        try:
            # Read request body
            body = await request.read()
            
            # Copy headers, excluding hop-by-hop headers
            headers = {k: v for k, v in request.headers.items() 
                      if k.lower() not in ('host', 'connection', 'keep-alive', 
                                           'proxy-authenticate', 'proxy-authorization',
                                           'te', 'trailers', 'transfer-encoding', 'upgrade')}
            
            # Make the proxied request
            async with self.session.request(
                method=request.method,
                url=target_url,
                headers=headers,
                data=body,
                allow_redirects=False
            ) as resp:
                # Copy response headers
                # For non-streaming responses, we need to exclude content-encoding and content-length
                # because aiohttp.Response.read() automatically decompresses the body
                response_headers = {k: v for k, v in resp.headers.items()
                                   if k.lower() not in ('connection', 'keep-alive',
                                                        'transfer-encoding', 'content-encoding',
                                                        'content-length')}
                
                # Check if this is a streaming response (SSE or chunked)
                # Only consider it streaming if it's actually SSE, not just chunked encoding
                content_type = resp.headers.get('content-type', '').lower()
                is_streaming = 'text/event-stream' in content_type
                
                if is_streaming:
                    # Stream the response (Server-Sent Events)
                    logger.info(f"🌊 Streaming response: {resp.status}")
                    response = StreamResponse(
                        status=resp.status,
                        headers=response_headers
                    )
                    await response.prepare(request)
                    
                    # Stream chunks from provider to client
                    try:
                        async for chunk in resp.content.iter_any():
                            await response.write(chunk)
                        await response.write_eof()
                    except Exception as stream_error:
                        logger.warning(f"Stream write error (client may have disconnected): {stream_error}")
                    return response
                else:
                    # Non-streaming response - read entire body
                    response_body = await resp.read()
                    logger.info(f"✅ Proxied response: {resp.status}")
                    
                    return Response(
                        body=response_body,
                        status=resp.status,
                        headers=response_headers
                    )
                
        except Exception as e:
            logger.error(f"❌ Error proxying request: {e}", exc_info=True)
            return web.json_response(
                {'error': {'message': f'Proxy error: {str(e)}'}},
                status=500
            )


async def create_app(error_interval: int) -> web.Application:
    """
    Create the aiohttp application.
    
    Args:
        error_interval: Inject an error every N requests
        
    Returns:
        Configured aiohttp application
    """
    app = web.Application()
    proxy = ErrorProxy(error_interval=error_interval)
    
    # Setup and teardown
    async def on_startup(app):
        await proxy.start_session()
        logger.info("🚀 Proxy session started")
        
    async def on_cleanup(app):
        await proxy.close_session()
        logger.info("🛑 Proxy session closed")
        
    app.on_startup.append(on_startup)
    app.on_cleanup.append(on_cleanup)
    
    # Route all requests through the proxy
    app.router.add_route('*', '/{path:.*}', proxy.handle_request)
    
    return app


def main():
    """Main entry point."""
    parser = ArgumentParser(description='Provider Error Proxy for Goose testing')
    parser.add_argument(
        '--port',
        type=int,
        default=8888,
        help='Port to listen on (default: 8888)'
    )
    parser.add_argument(
        '--error-interval',
        type=int,
        default=5,
        help='Inject an error every N requests (default: 5)'
    )
    
    args = parser.parse_args()
    
    logger.info("=" * 60)
    logger.info("🔧 Provider Error Proxy")
    logger.info("=" * 60)
    logger.info(f"Port: {args.port}")
    logger.info(f"Error interval: every {args.error_interval} requests")
    logger.info("")
    logger.info("To use with Goose, set these environment variables:")
    logger.info(f"  export OPENAI_HOST=http://localhost:{args.port}")
    logger.info(f"  export ANTHROPIC_HOST=http://localhost:{args.port}")
    logger.info(f"  export GOOGLE_HOST=http://localhost:{args.port}")
    logger.info(f"  export OPENROUTER_HOST=http://localhost:{args.port}")
    logger.info(f"  export TETRATE_HOST=http://localhost:{args.port}")
    logger.info(f"  export DATABRICKS_HOST=http://localhost:{args.port}")
    logger.info("=" * 60)
    logger.info("")
    
    # Create and run the app
    app = asyncio.run(create_app(args.error_interval))
    web.run_app(app, host='localhost', port=args.port)


if __name__ == '__main__':
    main()
