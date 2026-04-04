#!/bin/bash
# Backend Performance Benchmark for Skylink
# Measures API response times, data sizes, and throughput

HOST="http://localhost:31787"
ITERATIONS=20

echo "======================================================================"
echo "SKYLINK BACKEND PERFORMANCE BENCHMARK"
echo "Host: $HOST"
echo "Iterations: $ITERATIONS"
echo "Timestamp: $(date -Iseconds)"
echo "======================================================================"

# Current aircraft stats
echo ""
echo "--- CURRENT LOAD ---"
AC_JSON=$(curl -s "$HOST/data/aircraft.json")
TOTAL_AC=$(echo "$AC_JSON" | python3 -c "import json,sys; d=json.load(sys.stdin); print(len(d.get('aircraft',[])))" 2>/dev/null)
WITH_POS=$(echo "$AC_JSON" | python3 -c "import json,sys; d=json.load(sys.stdin); print(sum(1 for a in d.get('aircraft',[]) if a.get('lat')))" 2>/dev/null)
MESSAGES=$(echo "$AC_JSON" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('messages','N/A'))" 2>/dev/null)
echo "Total aircraft: $TOTAL_AC"
echo "With position:  $WITH_POS"
echo "Messages:       $MESSAGES"

# Container resource usage
echo ""
echo "--- CONTAINER RESOURCES ---"
docker stats skylink --no-stream --format "CPU: {{.CPUPerc}}  MEM: {{.MemUsage}}  NET I/O: {{.NetIO}}  BLOCK I/O: {{.BlockIO}}"

# readsb process stats
echo ""
echo "--- READSB PROCESS ---"
docker exec skylink ps aux | grep readsb | grep -v grep | awk '{printf "PID: %s  CPU: %s%%  MEM: %s%%  RSS: %s\n", $2, $3, $4, $6}'

echo ""
echo "--- API RESPONSE BENCHMARKS ---"
echo "----------------------------------------------------------------------"
printf "%-35s %8s %8s %8s %8s\n" "Endpoint" "Avg(ms)" "Min(ms)" "Max(ms)" "Size(KB)"
echo "----------------------------------------------------------------------"

benchmark_endpoint() {
    local name="$1"
    local url="$2"
    local times=()
    local sizes=()

    for i in $(seq 1 $ITERATIONS); do
        result=$(curl -s -o /dev/null -w "%{time_total} %{size_download}" "$url")
        time_ms=$(echo "$result" | awk '{printf "%.1f", $1 * 1000}')
        size_kb=$(echo "$result" | awk '{printf "%.1f", $2 / 1024}')
        times+=("$time_ms")
        sizes+=("$size_kb")
    done

    # Calculate stats
    avg=$(printf '%s\n' "${times[@]}" | awk '{s+=$1} END {printf "%.1f", s/NR}')
    min=$(printf '%s\n' "${times[@]}" | sort -n | head -1)
    max=$(printf '%s\n' "${times[@]}" | sort -n | tail -1)
    avg_size=$(printf '%s\n' "${sizes[@]}" | awk '{s+=$1} END {printf "%.1f", s/NR}')

    printf "%-35s %8s %8s %8s %8s\n" "$name" "$avg" "$min" "$max" "$avg_size"
}

benchmark_endpoint "aircraft.json (full)" "$HOST/data/aircraft.json"
benchmark_endpoint "receiver.json" "$HOST/data/receiver.json"

# Find a globe tile to test
GLOBE_TILES=$(curl -s "$HOST/chunks/chunks.json" 2>/dev/null | python3 -c "
import json,sys
try:
    d=json.load(sys.stdin)
    if 'chunk_list' in d:
        print(d['chunk_list'][0] if d['chunk_list'] else '')
except: pass
" 2>/dev/null)

if [ -n "$GLOBE_TILES" ]; then
    benchmark_endpoint "chunks.json (index)" "$HOST/chunks/chunks.json"
fi

# Test current_large (binCraft compressed)
benchmark_endpoint "current_large.gz (binCraft)" "$HOST/chunks/current_large.gz"
benchmark_endpoint "current_small.gz (binCraft)" "$HOST/chunks/current_small.gz"

echo "----------------------------------------------------------------------"

# Throughput test: rapid sequential requests
echo ""
echo "--- THROUGHPUT TEST (50 rapid requests) ---"
START=$(date +%s%N)
for i in $(seq 1 50); do
    curl -s -o /dev/null "$HOST/data/aircraft.json" &
done
wait
END=$(date +%s%N)
ELAPSED_MS=$(( (END - START) / 1000000 ))
RPS=$(echo "scale=1; 50000 / $ELAPSED_MS" | bc)
echo "50 concurrent requests completed in ${ELAPSED_MS}ms"
echo "Effective throughput: ${RPS} req/s"

# Memory and tmpfs usage
echo ""
echo "--- MEMORY & TMPFS ---"
docker exec skylink df -h /run /tmp | tail -2
echo ""
docker exec skylink du -sh /run/readsb/ /run/tar1090/ 2>/dev/null

# Network buffer check
echo ""
echo "--- READSB CONFIG ---"
docker exec skylink ps aux | grep readsb | grep -v grep | tr ' ' '\n' | grep -E "net-buffer|net-connector|quiet|json-trace"

echo ""
echo "======================================================================"
echo "BENCHMARK COMPLETE"
echo "======================================================================"
