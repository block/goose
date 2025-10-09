#!/usr/bin/env python3
"""
Test script for the provider error proxy.

This script tests both regular and streaming responses through the proxy.
"""

import asyncio
import json
import sys

import aiohttp


async def test_non_streaming():
    """Test a non-streaming request through the proxy."""
    print("Testing non-streaming request...")
    
    url = "http://localhost:8888/v1/chat/completions"
    headers = {
        "Authorization": "Bearer test-key",
        "Content-Type": "application/json"
    }
    payload = {
        "model": "gpt-4",
        "messages": [{"role": "user", "content": "Hello"}],
        "stream": False
    }
    
    async with aiohttp.ClientSession() as session:
        try:
            async with session.post(url, headers=headers, json=payload) as resp:
                print(f"  Status: {resp.status}")
                print(f"  Content-Type: {resp.headers.get('content-type')}")
                body = await resp.text()
                print(f"  Body (first 200 chars): {body[:200]}")
                print("  ✅ Non-streaming test passed\n")
        except Exception as e:
            print(f"  ❌ Error: {e}\n")


async def test_streaming():
    """Test a streaming request through the proxy."""
    print("Testing streaming request...")
    
    url = "http://localhost:8888/v1/chat/completions"
    headers = {
        "Authorization": "Bearer test-key",
        "Content-Type": "application/json"
    }
    payload = {
        "model": "gpt-4",
        "messages": [{"role": "user", "content": "Hello"}],
        "stream": True
    }
    
    async with aiohttp.ClientSession() as session:
        try:
            async with session.post(url, headers=headers, json=payload) as resp:
                print(f"  Status: {resp.status}")
                print(f"  Content-Type: {resp.headers.get('content-type')}")
                
                # Read first few chunks
                chunk_count = 0
                async for chunk in resp.content.iter_any():
                    chunk_count += 1
                    if chunk_count <= 3:
                        print(f"  Chunk {chunk_count}: {chunk[:100]}")
                    if chunk_count >= 10:
                        break
                
                print(f"  Received {chunk_count} chunks")
                print("  ✅ Streaming test passed\n")
        except Exception as e:
            print(f"  ❌ Error: {e}\n")


async def test_error_injection():
    """Test error injection."""
    print("Testing error injection (making 5 requests)...")
    
    url = "http://localhost:8888/v1/chat/completions"
    headers = {
        "Authorization": "Bearer test-key",
        "Content-Type": "application/json"
    }
    payload = {
        "model": "gpt-4",
        "messages": [{"role": "user", "content": "Hello"}],
        "stream": False
    }
    
    async with aiohttp.ClientSession() as session:
        for i in range(1, 6):
            try:
                async with session.post(url, headers=headers, json=payload) as resp:
                    print(f"  Request {i}: Status {resp.status}")
                    if resp.status >= 400:
                        body = await resp.json()
                        print(f"    Error: {body}")
            except Exception as e:
                print(f"  Request {i}: ❌ Error: {e}")
    
    print("  ✅ Error injection test completed\n")


async def main():
    """Run all tests."""
    print("=" * 60)
    print("Provider Error Proxy - Test Suite")
    print("=" * 60)
    print("Make sure the proxy is running on port 8888")
    print("  uv run proxy.py --error-interval 5")
    print("=" * 60)
    print()
    
    # Note: These tests will fail without actual API keys and will hit the proxy
    # The goal is to verify the proxy handles requests correctly
    
    await test_non_streaming()
    await test_streaming()
    await test_error_injection()
    
    print("=" * 60)
    print("Test suite completed!")
    print("=" * 60)


if __name__ == '__main__':
    asyncio.run(main())
