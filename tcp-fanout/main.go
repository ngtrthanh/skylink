package main

import (
	"fmt"
	"net"
	"os"
	"sync"
	"time"
)

// Subscribers receive broadcast data
var (
	subscribers   = make(map[net.Conn]struct{})
	subscribersMu sync.RWMutex
	stats         struct {
		bytesIn      uint64
		bytesOut     uint64
		subsNow      int
		feedersNow   int
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

// handleFeeder reads Beast data from a feeder connection and broadcasts it
func handleFeeder(conn net.Conn) {
	addr := conn.RemoteAddr().String()
	fmt.Printf("[fanout] feeder connected: %s\n", addr)
	stats.feedersNow++

	if tc, ok := conn.(*net.TCPConn); ok {
		tc.SetKeepAlive(true)
		tc.SetKeepAlivePeriod(30 * time.Second)
	}

	buf := make([]byte, 16384)
	for {
		n, err := conn.Read(buf)
		if n > 0 {
			stats.bytesIn += uint64(n)
			// Copy before broadcast to avoid buffer reuse race
			data := make([]byte, n)
			copy(data, buf[:n])
			broadcast(data)
		}
		if err != nil {
			fmt.Printf("[fanout] feeder disconnected: %s (%v)\n", addr, err)
			stats.feedersNow--
			conn.Close()
			return
		}
	}
}

func main() {
	ingestPort := os.Getenv("INGEST_PORT")
	listenPort := os.Getenv("LISTEN_PORT")
	if ingestPort == "" {
		ingestPort = "30004"
	}
	if listenPort == "" {
		listenPort = "40004"
	}

	// Listen for feeders (Beast input)
	ingestLn, err := net.Listen("tcp", ":"+ingestPort)
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to listen on :%s: %v\n", ingestPort, err)
		os.Exit(1)
	}
	fmt.Printf("[fanout] ingest listening on :%s (feeders push here)\n", ingestPort)

	// Listen for subscribers (prod/test connect here)
	subLn, err := net.Listen("tcp", ":"+listenPort)
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to listen on :%s: %v\n", listenPort, err)
		os.Exit(1)
	}
	fmt.Printf("[fanout] output listening on :%s (subscribers connect here)\n", listenPort)

	// Accept feeders
	go func() {
		for {
			conn, err := ingestLn.Accept()
			if err != nil {
				continue
			}
			go handleFeeder(conn)
		}
	}()

	// Accept subscribers
	go func() {
		for {
			conn, err := subLn.Accept()
			if err != nil {
				continue
			}
			addSub(conn)
			fmt.Printf("[fanout] subscriber connected: %s (total: %d)\n", conn.RemoteAddr(), stats.subsNow)

			// Detect subscriber disconnect
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
	for {
		time.Sleep(60 * time.Second)
		fmt.Printf("[fanout] feeders=%d subscribers=%d in=%.1fMB out=%.1fMB\n",
			stats.feedersNow, stats.subsNow,
			float64(stats.bytesIn)/1048576,
			float64(stats.bytesOut)/1048576)
	}
}
