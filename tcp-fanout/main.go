package main

import (
	"fmt"
	"io"
	"net"
	"os"
	"sync"
	"time"
)

var (
	clients   = make(map[net.Conn]struct{})
	clientsMu sync.RWMutex
	stats     struct {
		bytesIn    uint64
		bytesOut   uint64
		clientsNow int
	}
)

func addClient(c net.Conn) {
	clientsMu.Lock()
	clients[c] = struct{}{}
	stats.clientsNow = len(clients)
	clientsMu.Unlock()
}

func removeClient(c net.Conn) {
	clientsMu.Lock()
	delete(clients, c)
	stats.clientsNow = len(clients)
	clientsMu.Unlock()
	c.Close()
}

func broadcast(data []byte) {
	clientsMu.RLock()
	defer clientsMu.RUnlock()
	for c := range clients {
		c.SetWriteDeadline(time.Now().Add(5 * time.Second))
		_, err := c.Write(data)
		if err != nil {
			// Mark for removal but don't modify map during RLock
			go removeClient(c)
		}
	}
	stats.bytesOut += uint64(len(data)) * uint64(len(clients))
}

func handleUpstream(addr string) {
	for {
		fmt.Printf("[fanout] connecting to upstream %s...\n", addr)
		upstream, err := net.DialTimeout("tcp", addr, 10*time.Second)
		if err != nil {
			fmt.Printf("[fanout] upstream connect failed: %v. Retrying in 5s...\n", err)
			time.Sleep(5 * time.Second)
			continue
		}
		fmt.Printf("[fanout] upstream connected (%s)\n", addr)

		// Enable TCP keepalive instead of read deadline
		if tc, ok := upstream.(*net.TCPConn); ok {
			tc.SetKeepAlive(true)
			tc.SetKeepAlivePeriod(30 * time.Second)
		}

		buf := make([]byte, 16384)
		for {
			n, err := upstream.Read(buf)
			if n > 0 {
				stats.bytesIn += uint64(n)
				dataCopy := make([]byte, n)
				copy(dataCopy, buf[:n])
				broadcast(dataCopy)
			}
			if err != nil {
				if err == io.EOF {
					fmt.Println("[fanout] upstream closed. Reconnecting...")
				} else {
					fmt.Printf("[fanout] upstream error: %v. Reconnecting...\n", err)
				}
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
		upstreamHost = "192.168.11.10"
	}
	if upstreamPort == "" {
		upstreamPort = "30004"
	}
	if listenPort == "" {
		listenPort = "40004"
	}

	upstreamAddr := fmt.Sprintf("%s:%s", upstreamHost, upstreamPort)
	listenAddr := fmt.Sprintf(":%s", listenPort)

	ln, err := net.Listen("tcp", listenAddr)
	if err != nil {
		panic(fmt.Sprintf("failed to listen: %v", err))
	}
	defer ln.Close()
	fmt.Printf("[fanout] listening on %s (upstream: %s)\n", listenAddr, upstreamAddr)

	// Stats logger
	go func() {
		for {
			time.Sleep(60 * time.Second)
			fmt.Printf("[fanout] stats: clients=%d in=%.1fMB out=%.1fMB\n",
				stats.clientsNow,
				float64(stats.bytesIn)/1048576,
				float64(stats.bytesOut)/1048576)
		}
	}()

	// Accept clients
	go func() {
		for {
			conn, err := ln.Accept()
			if err != nil {
				continue
			}
			addClient(conn)
			fmt.Printf("[fanout] client connected: %s (total: %d)\n", conn.RemoteAddr(), stats.clientsNow)

			go func(c net.Conn) {
				buf := make([]byte, 1)
				for {
					c.SetReadDeadline(time.Now().Add(300 * time.Second))
					_, err := c.Read(buf)
					if err != nil {
						fmt.Printf("[fanout] client disconnected: %s\n", c.RemoteAddr())
						removeClient(c)
						return
					}
				}
			}(conn)
		}
	}()

	handleUpstream(upstreamAddr)
}
