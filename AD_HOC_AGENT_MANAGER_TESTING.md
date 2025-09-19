Testing Environment Setup

1. Local goosed Instance Configuration

Basic Setup


# Build the latest goosed with AgentManager
cargo build -p goose-server --bin goosed

# Start goosed with explicit configuration (MUST use screen for servers)
screen -dmS goosed_test bash -c "
  RUST_LOG=info \
  GOOSE_PORT=8081 \
  GOOSE_API_KEY=\$(cat ~/keys/oncall_buddy_goose_etc_databricks.txt) \
  GOOSE_DEFAULT_PROVIDER=databricks \
  GOOSE_SERVER__SECRET_KEY=test123 \
  GOOSE_DEFAULT_MODEL=claude-3-5-sonnet-latest \
  ./target/debug/goosed agent
"

# Verify it's running
lsof -i :8081

# Stop the session
screen -r goosed_test -X stuff $'\003' && screen -X -S goosed_test quit

Important Notes


MUST use screen for servers - Background processes don't work properly without it

Secret key environment variable: GOOSE_SERVER__SECRET_KEY (note the double underscore!)

Secret key header: X-Secret-Key (case-sensitive)

No /api prefix - Routes are directly under root (e.g., /reply, /agent/stats)

Default secret key: If not set, defaults to "test"


2. Provider Configuration

Databricks Provider


API Key Location: ~/keys/oncall_buddy_goose_etc_databricks.txt

Environment Variable: GOOSE_API_KEY

Provider Name: databricks

Security: Never cat or expose the key in logs/context


Testing Methodology

Phase 1: Unit Testing (Completed)

Located in crates/goose/tests/agent_manager_test.rs:




Session Isolation



Verify each session gets unique agent

Confirm same session reuses same agent

Test agent pointer equality




Resource Management



Test idle agent cleanup

Verify memory reclamation

Test agent recreation after cleanup




Concurrency



Multiple threads accessing same session

Verify no race conditions

Test mutex contention handling




Metrics



Track agent creation/cleanup

Monitor cache hits/misses

Verify active agent counts




Phase 2: Integration Testing

Test 1: Multi-Session Isolation


# Create test script: test_multi_session.sh
#!/bin/bash

# Session 1: Create and use agent
curl -X POST http://localhost:8080/api/reply \
  -H "X-Secret-Key: test_secret" \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "test_session_1",
    "messages": [{"role": "user", "content": "Hello from session 1"}]
  }' &

# Session 2: Concurrent request
curl -X POST http://localhost:8080/api/reply \
  -H "X-Secret-Key: test_secret" \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "test_session_2",
    "messages": [{"role": "user", "content": "Hello from session 2"}]
  }' &

wait

Test 2: Extension Isolation


# Test that extensions loaded in one session don't affect another
# Session 1: Load extension
curl -X POST http://localhost:8080/api/extension/add \
  -H "X-Secret-Key: test_secret" \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "ext_test_1",
    "extension": {"type": "builtin", "name": "memory"}
  }'

# Session 2: Verify extension not present
curl -X GET http://localhost:8080/api/extension/list \
  -H "X-Secret-Key: test_secret" \
  -H "Content-Type: application/json" \
  -d '{"session_id": "ext_test_2"}'

Test 3: Provider Configuration Per Session


# Test different provider configurations per session
# Session 1: Default provider
# Session 2: Different model/temperature

Phase 3: Performance Testing

Benchmark Metrics



Agent Creation Time



Target: < 10ms per agent

Measure: Time from request to agent ready




Memory Usage



Baseline: Memory with 0 agents

Per-agent overhead: Expected 5-20MB

Test with 10, 50, 100 concurrent sessions




Cleanup Performance



Idle timeout: 5 minutes default

Cleanup execution: < 100ms

No impact on active sessions




Load Testing Script


# load_test.py
import asyncio
import aiohttp
import time
import statistics

async def create_session_and_query(session, session_id):
    url = "http://localhost:8080/api/reply"
    headers = {
        "X-Secret-Key": "test_secret",
        "Content-Type": "application/json"
    }
    data = {
        "session_id": f"load_test_{session_id}",
        "messages": [{"role": "user", "content": f"Test message {session_id}"}]
    }
    
    start = time.time()
    async with session.post(url, json=data, headers=headers) as response:
        await response.text()
    return time.time() - start

async def load_test(num_sessions):
    async with aiohttp.ClientSession() as session:
        tasks = [create_session_and_query(session, i) for i in range(num_sessions)]
        times = await asyncio.gather(*tasks)
    
    print(f"Sessions: {num_sessions}")
    print(f"Average time: {statistics.mean(times):.3f}s")
    print(f"Max time: {max(times):.3f}s")
    print(f"Min time: {min(times):.3f}s")

# Run with different loads
asyncio.run(load_test(10))
asyncio.run(load_test(50))
asyncio.run(load_test(100))

Phase 4: Stress Testing

Test Scenarios



Rapid Session Creation/Destruction



Create 100 sessions rapidly

Immediately trigger cleanup

Verify no memory leaks




Long-Running Sessions



Keep sessions alive for hours

Verify no degradation

Test cleanup doesn't affect active sessions




Extension Churn



Rapidly add/remove extensions

Different extensions per session

Verify no cross-contamination




Provider Switching



Switch providers mid-conversation

Multiple providers across sessions

Verify correct routing




Phase 5: Error Handling

Test Cases



Agent Creation Failure



Simulate OOM conditions

Test graceful degradation

Verify error messages




Cleanup During Active Use



Attempt cleanup while agent is processing

Verify protection mechanisms

Test recovery




Concurrent Modifications



Multiple requests modifying same session

Test lock handling

Verify consistency




Success Criteria

Functional Requirements


 Each session gets unique agent

 No state bleeding between sessions

 Extensions isolated per session (automatic with per-session agents)

 Provider configuration per session (automatic with per-session agents)

 Graceful cleanup of idle agents

 No impact on active sessions during cleanup


Performance Requirements


 Agent creation < 10ms

 Memory per agent < 20MB

 Cleanup execution < 100ms

 Support 100+ concurrent sessions

 No mutex contention hotspots

 Response time degradation < 10% under load


Reliability Requirements


 No memory leaks over 24-hour run

 Graceful handling of resource limits

 Recovery from transient failures

 Proper error propagation

 Clean shutdown without data loss


Testing Tools and Scripts

Monitoring Script


#!/bin/bash
# monitor_goosed.sh

while true; do
    echo "=== $(date) ==="
    
    # Memory usage
    ps aux | grep goosed | grep -v grep | awk '{print "Memory: " $6/1024 " MB"}'
    
    # Open connections
    lsof -i :8080 | wc -l | awk '{print "Connections: " $1-1}'
    
    # Active agents (would need API endpoint)
    # curl -s -H "X-Secret-Key: test_secret" http://localhost:8080/api/agent/stats
    
    sleep 5
done

Session Cleanup Verification


#!/bin/bash
# verify_cleanup.sh

# Create sessions
for i in {1..10}; do
    curl -X POST http://localhost:8080/api/reply \
      -H "X-Secret-Key: test_secret" \
      -H "Content-Type: application/json" \
      -d "{\"session_id\": \"cleanup_test_$i\", \"messages\": [{\"role\": \"user\", \"content\": \"test\"}]}" &
done
wait

# Wait for idle timeout
sleep 310  # 5 minutes + buffer

# Trigger cleanup (would need API endpoint)
# curl -X POST http://localhost:8080/api/agent/cleanup \
#   -H "X-Secret-Key: test_secret"

# Verify agents were cleaned
# Check memory usage decreased

Appendix: Quick Reference

Start goosed for Testing


screen -dmS goosed_test bash -c "
  RUST_LOG=info \
  GOOSE_PORT=8080 \
  GOOSE_API_KEY=$(cat ~/keys/oncall_buddy_goose_etc_databricks.txt) \
  GOOSE_DEFAULT_PROVIDER=databricks \
  GOOSE_SECRET_KEY=test_secret \
  ./target/debug/goosed agent
"

Test Session Isolation


# Session 1
curl -X POST http://localhost:8080/api/reply \
  -H "X-Secret-Key: test_secret" \
  -H "Content-Type: application/json" \
  -d '{"session_id": "test1", "messages": [{"role": "user", "content": "test"}]}'

# Session 2 (concurrent)
curl -X POST http://localhost:8080/api/reply \
  -H "X-Secret-Key: test_secret" \
  -H "Content-Type: application/json" \
  -d '{"session_id": "test2", "messages": [{"role": "user", "content": "test"}]}'

Monitor goosed


watch -n 1 'ps aux | grep goosed | grep -v grep'

Stop goosed


screen -X -S goosed_test quit
