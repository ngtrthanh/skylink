package main

import (
	"fmt"
	"net"
	"os"
	"sync"
	"time"
)

var (
	subscribers   = make(map[net.Conn]struct{})
	subscribersMu sync.RWMutex
	stats         struct {
		bytesIn  uint64
		bytesOut uint64
		subsNow  int
	}
)

func broadcast(data []byte) {
	subscribersMu.RLock()
	defer subscribersMu.RUnlock()
	for c := range subscribers {
		c.SetWriteDeadline(time.Now().Add(5 * time.Second))
		_, err := c.Write(data)
		if err != nil {
			go removeSub(c)
		}
	}
	stats.bytesOut += uint64(len(data)) * uint64(len(subscribers))
}

func addSub(c net.Conn) {
	subscribersMu.Lock()
	subscribers[c] = struct{}{}
	stats.subsNow = len(subscribers)
	subscribersMu.Unlock()
}

func removeSub(c net.Conn) {
	subscribersMu.Lock()
	delete(subscribers, c)
	stats.subsNow = len(subscribers)
	subscribersMu.Unlock()
	c.Close()
}

func connectUpstream(addr string) {
	for {
		fmt.Printf("[fanout] connecting to upstream %s...\n", addr)
		upstream, err := net.DialTimeout("tcp", addr, 10*time.Second)
		if err != nil {
			fmt.Printf("[fanout] upstream connect failed: %v. Retrying in 5s...\n", err)
			time.Sleep(5 * time.Second)
			continue
		}
		fmt.Printf("[fanout] upstream connected (%s)\n", addr)

		if tc, ok := upstream.(*net.TCPConn); ok {
			tc.SetKeepAlive(true)
			tc.SetKeepAlivePeriod(30 * time.Second)
		}

		buf := make([]byte, 16384)
		for {
			n, err := upstream.Read(buf)
			if n > 0 {
				stats.bytesIn += uint64(n)
				data := make([]byte, n)
				copy(data, buf[:n])
				broadcast(data)
			}
			if err != nil {
				fmt.Printf("[fanout] upstream error: %v. Reconnecting in 5s...\n", err)
				upstream.Close()
				break
			}
		}
		time.Sleep(5 * time.Second)
	}
}

func main() {
	upstreamHost := os.Getenv("UPSTREAM_HOST")
	upstreamPort := os.Getenv("UPSTREAM_PORT")
	listenPort := os.Getenv("LISTEN_PORT")

	if upstreamHost == "" {
		upstreamHost = "skylink"
	}
	if upstreamPort == "" {
		upstreamPort = "30005"
	}
	if listenPort == "" {
		listenPort = "40004"
	}

	upstreamAddr := fmt.Sprintf("%s:%s", upstreamHost, upstreamPort)
	listenAddr := fmt.Sprintf(":%s", listenPort)

	ln, err := net.Listen("tcp", listenAddr)
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to listen on %s: %v\n", listenAddr, err)
		os.Exit(1)
	}
	fmt.Printf("[fanout] listening on %s (upstream: %s)\n", listenAddr, upstreamAddr)

	// Accept subscribers
	go func() {
		for {
			conn, err := ln.Accept()
			if err != nil {
				continue
			}
			addSub(conn)
			fmt.Printf("[fanout] subscriber connected: %s (total: %d)\n", conn.RemoteAddr(), stats.subsNow)

			go func(c net.Conn) {
				buf := make([]byte, 1)
				for {
					c.SetReadDeadline(time.Now().Add(300 * time.Second))
					_, err := c.Read(buf)
					if err != nil {
						fmt.Printf("[fanout] subscriber disconnected: %s\n", c.RemoteAddr())
						removeSub(c)
						return
					}
				}
			}(conn)
		}
	}()

	// Stats logger
	go func() {
		for {
			time.Sleep(60 * time.Second)
			fmt.Printf("[fanout] subscribers=%d in=%.1fMB out=%.1fMB\n",
				stats.subsNow,
				float64(stats.bytesIn)/1048576,
				float64(stats.bytesOut)/1048576)
		}
	}()

	// Connect to upstream (blocks, auto-reconnects)
	connectUpstream(upstreamAddr)
}
