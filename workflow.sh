#!/bin/bash
# Skylink Test/Prod Workflow
# Usage:
#   ./workflow.sh setup     - First time: copy prod source to test dir
#   ./workflow.sh start     - Start fan-out + prod (normal operation)
#   ./workflow.sh test-up   - Spin up test instance
#   ./workflow.sh test-down - Tear down test instance
#   ./workflow.sh compare   - Compare prod vs test metrics
#   ./workflow.sh promote   - Merge test config into prod
#   ./workflow.sh status    - Show status of all services

set -e
COMPOSE="docker compose -f docker-compose-fanout.yml"
PROD_SRC="./local/skylink-lc2"
TEST_SRC="./local/skylink-test"

case "$1" in

  setup)
    echo "=== Setting up test environment ==="
    if [ -d "$TEST_SRC" ]; then
      echo "Test source already exists at $TEST_SRC"
      echo "Delete it first if you want a fresh copy: rm -rf $TEST_SRC"
      exit 1
    fi
    cp -r "$PROD_SRC" "$TEST_SRC"
    rm -rf "$TEST_SRC/.git"
    echo "Copied prod source to $TEST_SRC"
    echo "Edit files in $TEST_SRC/html/ to test changes"
    echo "Then run: $0 test-up"
    ;;

  start)
    echo "=== Starting fan-out + production ==="
    $COMPOSE up -d beast-fanout skylink
    echo ""
    echo "Fan-out: localhost:40004"
    echo "Prod:    http://localhost:31787"
    ;;

  test-up)
    echo "=== Starting test instance ==="
    if [ ! -d "$TEST_SRC" ]; then
      echo "Run '$0 setup' first to create test source"
      exit 1
    fi
    $COMPOSE --profile test up -d skylink-test
    echo ""
    echo "Test: http://localhost:31788"
    echo "Edit: $TEST_SRC/html/"
    echo ""
    echo "To apply changes without restart:"
    echo "  docker exec skylink-test cp /var/tar1090_git_source/html/config.js /usr/local/share/tar1090/html-webroot/config.js"
    ;;

  test-down)
    echo "=== Stopping test instance ==="
    $COMPOSE --profile test stop skylink-test
    $COMPOSE --profile test rm -f skylink-test
    echo "Test instance removed. Prod untouched."
    ;;

  compare)
    echo "=== Comparing Prod vs Test ==="
    echo ""
    echo "--- Aircraft Count ---"
    PROD_AC=$(curl -s http://localhost:31787/data/aircraft.json 2>/dev/null | python3 -c "import json,sys; d=json.load(sys.stdin); print(f\"{len(d.get('aircraft',[]))} total, {sum(1 for a in d.get('aircraft',[]) if a.get('lat'))} with pos\")" 2>/dev/null || echo "unavailable")
    TEST_AC=$(curl -s http://localhost:31788/data/aircraft.json 2>/dev/null | python3 -c "import json,sys; d=json.load(sys.stdin); print(f\"{len(d.get('aircraft',[]))} total, {sum(1 for a in d.get('aircraft',[]) if a.get('lat'))} with pos\")" 2>/dev/null || echo "unavailable")
    echo "  Prod: $PROD_AC"
    echo "  Test: $TEST_AC"

    echo ""
    echo "--- API Response Time (20 requests) ---"
    PROD_TIME=$(for i in $(seq 1 20); do curl -s -o /dev/null -w "%{time_total}\n" http://localhost:31787/data/aircraft.json; done | awk '{s+=$1} END {printf "%.1f", s/NR*1000}')
    TEST_TIME=$(for i in $(seq 1 20); do curl -s -o /dev/null -w "%{time_total}\n" http://localhost:31788/data/aircraft.json; done | awk '{s+=$1} END {printf "%.1f", s/NR*1000}')
    echo "  Prod: ${PROD_TIME}ms avg"
    echo "  Test: ${TEST_TIME}ms avg"

    echo ""
    echo "--- Page Size (gzipped) ---"
    PROD_SIZE=$(curl -s -H "Accept-Encoding: gzip" -o /dev/null -w "%{size_download}" http://localhost:31787/)
    TEST_SIZE=$(curl -s -H "Accept-Encoding: gzip" -o /dev/null -w "%{size_download}" http://localhost:31788/)
    echo "  Prod: $((PROD_SIZE/1024)) KB"
    echo "  Test: $((TEST_SIZE/1024)) KB"

    echo ""
    echo "--- Container Resources ---"
    docker stats --no-stream --format "  {{.Name}}: CPU={{.CPUPerc}} MEM={{.MemUsage}}" skylink skylink-test 2>/dev/null

    echo ""
    echo "--- JS File Diff ---"
    diff <(docker exec skylink md5sum /usr/local/share/tar1090/html-webroot/config.js 2>/dev/null) \
         <(docker exec skylink-test md5sum /usr/local/share/tar1090/html-webroot/config.js 2>/dev/null) \
         && echo "  config.js: IDENTICAL" || echo "  config.js: DIFFERENT"
    ;;

  promote)
    echo "=== Promoting test → prod ==="
    echo "This will:"
    echo "  1. Copy $TEST_SRC/html/ → $PROD_SRC/html/"
    echo "  2. Restart prod container"
    echo ""
    read -p "Continue? (y/N) " confirm
    if [ "$confirm" != "y" ]; then
      echo "Aborted."
      exit 0
    fi

    # Backup prod
    BACKUP="$PROD_SRC.bak.$(date +%Y%m%d_%H%M%S)"
    cp -r "$PROD_SRC" "$BACKUP"
    echo "Backed up prod to $BACKUP"

    # Copy test files to prod (preserve .git if exists)
    rsync -av --exclude='.git' "$TEST_SRC/html/" "$PROD_SRC/html/"
    echo "Copied test HTML to prod source"

    # Restart prod
    $COMPOSE up -d skylink
    echo ""
    echo "Prod restarted with test config."
    echo "Verify at http://localhost:31787"
    echo "Rollback: cp -r $BACKUP/* $PROD_SRC/ && $COMPOSE up -d skylink"
    ;;

  status)
    echo "=== Service Status ==="
    docker ps --format "  {{.Names}}: {{.Status}} ({{.Ports}})" --filter name=beast-fanout --filter name=skylink 2>/dev/null
    echo ""
    echo "=== Fan-out Connections ==="
    docker exec beast-fanout sh -c 'netstat -tn 2>/dev/null | grep 40004 | grep ESTABLISHED | wc -l' 2>/dev/null && echo " active subscribers" || echo "  fan-out not running"
    ;;

  *)
    echo "Usage: $0 {setup|start|test-up|test-down|compare|promote|status}"
    echo ""
    echo "Workflow:"
    echo "  1. $0 setup       # Create test source from prod"
    echo "  2. Edit local/skylink-test/html/  # Make changes"
    echo "  3. $0 test-up     # Start test instance"
    echo "  4. Compare http://localhost:31787 (prod) vs :31788 (test)"
    echo "  5. $0 compare     # Automated comparison"
    echo "  6. $0 promote     # Merge test → prod (with backup)"
    echo "  7. $0 test-down   # Clean up test"
    ;;
esac
