#!/bin/bash
# Verification script for file descriptor limit fix

echo "=== File Descriptor Limit Verification ==="
echo ""

# 1. Check container limits
echo "1. Container ulimit settings:"
docker exec skylink sh -c "ulimit -n" 2>/dev/null
echo ""

# 2. Check container process limits
echo "2. Container process limits:"
docker exec skylink sh -c "cat /proc/1/limits | grep 'open files'" 2>/dev/null
echo ""

# 3. Current file descriptor usage
echo "3. Current file descriptor usage in container:"
FD_COUNT=$(docker exec skylink sh -c "ls -1 /proc/1/fd 2>/dev/null | wc -l" 2>/dev/null)
echo "   Active FDs: ${FD_COUNT}"
echo ""

# 4. Active connections
echo "4. Active TCP connections on port 30004:"
ACTIVE=$(ss -tn '( sport = :30004 )' 2>/dev/null | grep ESTAB | wc -l)
echo "   ESTABLISHED: ${ACTIVE}"
echo ""

# 5. Health check calculation
if [ -n "$FD_COUNT" ] && [ -n "$ACTIVE" ]; then
    echo "5. Health Analysis:"
    LIMIT=$(docker exec skylink sh -c "ulimit -n" 2>/dev/null)
    
    if [ "$LIMIT" -ge 65536 ]; then
        echo "   ✓ File descriptor limit is adequate (${LIMIT})"
    else
        echo "   ✗ File descriptor limit is TOO LOW (${LIMIT})"
        echo "     Need at least 65536 for 2000+ connections"
    fi
    
    if [ "$FD_COUNT" -lt "$((LIMIT * 80 / 100))" ]; then
        echo "   ✓ FD usage is healthy (${FD_COUNT}/${LIMIT} = $((FD_COUNT * 100 / LIMIT))%)"
    else
        echo "   ⚠️  FD usage is high (${FD_COUNT}/${LIMIT} = $((FD_COUNT * 100 / LIMIT))%)"
    fi
    
    if [ "$ACTIVE" -gt 1800 ]; then
        echo "   ✓ Connection count is healthy (${ACTIVE})"
    elif [ "$ACTIVE" -gt 1000 ]; then
        echo "   ⚠️  Connection count moderate (${ACTIVE})"
    else
        echo "   ✗ Connection count LOW (${ACTIVE}) - may indicate issue"
    fi
fi

echo ""
echo "=== Monitoring Command ==="
echo "Run this to monitor in real-time:"
echo "watch -n 5 'echo \"FDs: \$(docker exec skylink ls -1 /proc/1/fd 2>/dev/null | wc -l) / \$(docker exec skylink ulimit -n 2>/dev/null)\"; echo \"Connections: \$(ss -tn sport = :30004 2>/dev/null | grep ESTAB | wc -l)\"'"
