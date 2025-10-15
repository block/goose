#!/usr/bin/env python3
"""
Integration test for the proxy with new command interface.
"""
import asyncio
import sys
from proxy import ErrorProxy, ErrorMode

async def test_proxy():
    """Test the ErrorProxy class with various command scenarios."""
    proxy = ErrorProxy()
    
    print("Testing ErrorProxy class:")
    print("=" * 70)
    
    # Test 1: Default state
    assert proxy.get_error_mode() == ErrorMode.NO_ERROR
    print("✅ Test 1: Default state is NO_ERROR")
    
    # Test 2: Set context length error with count
    proxy.set_error_mode(ErrorMode.CONTEXT_LENGTH, count=3, percentage=0.0)
    mode, count, pct = proxy.get_error_config()
    assert mode == ErrorMode.CONTEXT_LENGTH
    assert count == 3
    assert pct == 0.0
    print("✅ Test 2: Set context length error with count=3")
    
    # Test 3: Inject errors (count mode)
    assert proxy.should_inject_error() == True  # 3 -> 2
    assert proxy.should_inject_error() == True  # 2 -> 1
    assert proxy.should_inject_error() == True  # 1 -> 0
    assert proxy.should_inject_error() == False  # 0, switches to NO_ERROR
    assert proxy.get_error_mode() == ErrorMode.NO_ERROR
    print("✅ Test 3: Count mode works correctly (3 errors then NO_ERROR)")
    
    # Test 4: Set rate limit error with percentage
    proxy.set_error_mode(ErrorMode.RATE_LIMIT, count=0, percentage=1.0)
    mode, count, pct = proxy.get_error_config()
    assert mode == ErrorMode.RATE_LIMIT
    assert count == 0
    assert pct == 1.0
    # In percentage mode, should always inject when pct=1.0
    assert proxy.should_inject_error() == True
    assert proxy.should_inject_error() == True
    assert proxy.get_error_mode() == ErrorMode.RATE_LIMIT  # Should stay in RATE_LIMIT
    print("✅ Test 4: Percentage mode works correctly (100% = always inject)")
    
    # Test 5: Set back to NO_ERROR (permanent)
    proxy.set_error_mode(ErrorMode.NO_ERROR, count=1, percentage=0.0)
    assert proxy.should_inject_error() == False
    assert proxy.get_error_mode() == ErrorMode.NO_ERROR
    print("✅ Test 5: NO_ERROR mode is permanent")
    
    # Test 6: Server error with count=1 (default)
    proxy.set_error_mode(ErrorMode.SERVER_ERROR, count=1, percentage=0.0)
    assert proxy.should_inject_error() == True  # 1 -> 0
    assert proxy.should_inject_error() == False  # 0, switches to NO_ERROR
    assert proxy.get_error_mode() == ErrorMode.NO_ERROR
    print("✅ Test 6: Default count=1 works correctly (1 error then NO_ERROR)")
    
    print("=" * 70)
    print("✅ All integration tests passed!")

if __name__ == '__main__':
    asyncio.run(test_proxy())
