// Performance Benchmark: Before vs After Phase 1
// Inject via browser console at skylink.hpradar.com
// Measures the actual hot-path functions with real aircraft data

(function() {
    'use strict';

    const ITERATIONS = 20;
    const results = {};

    function bench(name, fn) {
        // warmup
        for (let i = 0; i < 3; i++) fn();
        // measure
        const times = [];
        for (let i = 0; i < ITERATIONS; i++) {
            const t0 = performance.now();
            fn();
            times.push(performance.now() - t0);
        }
        times.sort((a, b) => a - b);
        const median = times[Math.floor(times.length / 2)];
        const p95 = times[Math.floor(times.length * 0.95)];
        const avg = times.reduce((a, b) => a + b, 0) / times.length;
        const min = times[0];
        const max = times[times.length - 1];
        results[name] = { median, p95, avg, min, max, samples: times.length };
        return results[name];
    }

    const planeCount = g.planesOrdered.length;
    const withPos = g.planesOrdered.filter(p => p.position).length;

    console.log('='.repeat(70));
    console.log(`SKYLINK PERFORMANCE BENCHMARK`);
    console.log(`Aircraft: ${planeCount} total, ${withPos} with position`);
    console.log(`Iterations per test: ${ITERATIONS}`);
    console.log(`Timestamp: ${new Date().toISOString()}`);
    console.log('='.repeat(70));

    // --- Test 1: updateVisible ---
    bench('updateVisible', function() {
        if (mapIsVisible || !lastRenderExtent) {
            lastRenderExtent = getRenderExtent();
        }
        let shown = 0;
        const planes = g.planesOrdered;
        const len = planes.length;
        for (let i = 0; i < len; i++) {
            const plane = planes[i];
            plane.inView = inView(plane.position, lastRenderExtent);
            plane.visible = plane.checkVisible() && !plane.isFiltered();
            shown += (plane.visible & plane.inView);
        }
    });

    // --- Test 2: for..in loop (OLD pattern) vs indexed loop ---
    bench('loop_forIn_planes', function() {
        let count = 0;
        for (let i in g.planesOrdered) {
            const plane = g.planesOrdered[i];
            if (plane.position) count++;
        }
    });

    bench('loop_indexed_planes', function() {
        let count = 0;
        const planes = g.planesOrdered;
        const len = planes.length;
        for (let i = 0; i < len; i++) {
            if (planes[i].position) count++;
        }
    });

    // --- Test 3: Array.slice vs Array.splice (trace buffer pattern) ---
    const testArrays = [];
    for (let i = 0; i < 200; i++) {
        const arr = [];
        for (let j = 0; j < 100; j++) arr.push({ now: j, position: [100 + j * 0.01, 20 + j * 0.01], altitude: j * 100 });
        testArrays.push(arr);
    }

    bench('trace_slice_pattern', function() {
        for (let k = 0; k < testArrays.length; k++) {
            let arr = testArrays[k].slice(); // copy
            if (arr.length > 100) {
                arr = arr.slice(-80);
            }
        }
    });

    bench('trace_splice_pattern', function() {
        for (let k = 0; k < testArrays.length; k++) {
            let arr = testArrays[k].slice(); // copy
            if (arr.length > 80) {
                arr.splice(0, arr.length - 60);
            }
        }
    });

    // --- Test 4: Object.keys delete vs null assignment (destroy pattern) ---
    function makeTestObj() {
        return {
            trace: [1,2,3], track_linesegs: [4,5,6], position: [100, 20],
            marker: {}, markerStyle: {}, flight: 'TEST', icao: 'abc123',
            altitude: 35000, speed: 450, track: 180, visible: true,
            a: 1, b: 2, c: 3, d: 4, e: 5, f: 6, g: 7, h: 8
        };
    }

    bench('destroy_objectKeys_delete', function() {
        for (let k = 0; k < 500; k++) {
            const obj = makeTestObj();
            for (let key in Object.keys(obj)) {
                delete obj[key];
            }
        }
    });

    bench('destroy_null_assign', function() {
        for (let k = 0; k < 500; k++) {
            const obj = makeTestObj();
            obj.trace = null;
            obj.track_linesegs = null;
            obj.position = null;
        }
    });

    // --- Test 5: mapRefresh simulation (sort + feature update count) ---
    bench('mapRefresh_sort', function() {
        const addToMap = [];
        const planes = g.planesOrdered;
        const len = planes.length;
        for (let i = 0; i < len; i++) {
            const plane = planes[i];
            if (plane.inView && plane.visible) {
                addToMap.push(plane);
            }
        }
        addToMap.sort(function(x, y) { return (x.zIndex || 0) - (y.zIndex || 0); });
    });

    bench('mapRefresh_sort_capped4000', function() {
        const addToMap = [];
        const planes = g.planesOrdered;
        const len = planes.length;
        let n = 0;
        for (let i = 0; i < len; i++) {
            const plane = planes[i];
            if (plane.inView && plane.visible && n < 4000) {
                addToMap.push(plane);
                n++;
            }
        }
        addToMap.sort(function(x, y) { return (x.zIndex || 0) - (y.zIndex || 0); });
    });

    // --- Test 6: processAircraft loop patterns ---
    // Simulate the data.aircraft iteration
    bench('processLoop_forLen', function() {
        const aircraft = g.planesOrdered;
        const acLen = aircraft.length;
        let count = 0;
        for (let j = 0; j < acLen; j++) {
            if (aircraft[j].icao) count++;
        }
    });

    bench('processLoop_forIn', function() {
        const aircraft = g.planesOrdered;
        let count = 0;
        for (let j in aircraft) {
            if (aircraft[j].icao) count++;
        }
    });

    // --- Print Results ---
    console.log('');
    console.log('RESULTS (all times in ms):');
    console.log('-'.repeat(70));
    console.log(
        'Test'.padEnd(32) +
        'Median'.padStart(8) +
        'P95'.padStart(8) +
        'Avg'.padStart(8) +
        'Min'.padStart(8) +
        'Max'.padStart(8)
    );
    console.log('-'.repeat(70));

    for (const [name, r] of Object.entries(results)) {
        console.log(
            name.padEnd(32) +
            r.median.toFixed(2).padStart(8) +
            r.p95.toFixed(2).padStart(8) +
            r.avg.toFixed(2).padStart(8) +
            r.min.toFixed(2).padStart(8) +
            r.max.toFixed(2).padStart(8)
        );
    }

    console.log('-'.repeat(70));

    // Speedup comparisons
    console.log('');
    console.log('SPEEDUP COMPARISONS:');
    const comparisons = [
        ['loop_forIn_planes', 'loop_indexed_planes', 'for..in → indexed loop'],
        ['trace_slice_pattern', 'trace_splice_pattern', 'slice → splice (trace buffer)'],
        ['destroy_objectKeys_delete', 'destroy_null_assign', 'Object.keys delete → null assign'],
        ['mapRefresh_sort', 'mapRefresh_sort_capped4000', 'uncapped → capped 4000 planes'],
        ['processLoop_forIn', 'processLoop_forLen', 'for..in → for(len) process loop'],
    ];

    for (const [before, after, label] of comparisons) {
        if (results[before] && results[after]) {
            const speedup = results[before].median / results[after].median;
            const saved = results[before].median - results[after].median;
            console.log(`  ${label}: ${speedup.toFixed(2)}x faster (saved ${saved.toFixed(2)}ms per call)`);
        }
    }

    // Per-refresh estimate
    const refreshSaved =
        (results['loop_forIn_planes'].median - results['loop_indexed_planes'].median) * 3 + // 3 hot loops
        (results['mapRefresh_sort'].median - results['mapRefresh_sort_capped4000'].median);

    console.log('');
    console.log(`ESTIMATED TOTAL SAVINGS PER REFRESH CYCLE: ~${refreshSaved.toFixed(1)}ms`);
    console.log(`At ${planeCount} aircraft, refresh runs every ~${(g.lastRefreshInt || 1000)}ms`);
    console.log(`CPU headroom recovered: ~${(refreshSaved / (g.lastRefreshInt || 1000) * 100).toFixed(1)}%`);
    console.log('='.repeat(70));

    // Store for later comparison
    window._perfBenchmark = { results, planeCount, withPos, timestamp: Date.now() };
    console.log('Results stored in window._perfBenchmark');

    return results;
})();
