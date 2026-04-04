package main

// Beast TCP fan-out modeled after readsb net_io.c pattern:
// - Per-client send queue (slow clients don't block fast ones)
// - Drop data for lagging clients instead of disconnecting
// - TCP keepalive for connection health
// - Heartbeat when idle to keep connections alive

import (
	"fmt"
	"net"
	"os"
	"sync"
	"time"
)

const (
	sendqMax       = 256 * 1024 // per-client send buffer
	readBuf        = 16384      // upstream read buffer
	writeTimeout   = 5 * time.Second
	heartbeatIdle  = 30 * time.Second
	dropHalfWindow = 2 * time.Second
)

// Beast heartbeat: 0x1a '1' + 9 zero bytes
var beastHeartbeat = []byte{0x1a, 0x31, 0, 0, 0, 0, 0, 0, 0, 0, 0}

type subscriber struct {
	conn      net.Conn
	sendq     []byte
	mu        sync.Mutex
	lastSend  time.Time
	dropUntil time.Time
	dropFlip  bool
	bytesIn   uint64
	bytesOut  uint64
}

func (s *subscriber) enqueue(data []byte) {
	s.mu.Lock()
	defer s.mu.Unlock()

	now := time.Now()

	// Drop half pattern: if client is lagging, skip every other write
	if now.Before(s.dropUntil) {
		s.dropFlip = !s.dropFlip
		if s.dropFlip {
			return
		}
	}

	if len(s.sendq)+len(data) > sendqMax {
		// Buffer full — drop this chunk and enable drop-half
		s.dropUntil = now.Add(dropHalfWindow)
		s.dropFlip = true
		return
	}

	s.sendq = append(s.sendq, data...)
	s.bytesIn += uint64(len(data))
}

func (s *subscriber) flush() error {
	s.mu.Lock()
	if len(s.sendq) == 0 {
		s.mu.Unlock()
		return nil
	}
	buf := s.sendq
	s.sendq = make([]byte, 0, sendqMax/4)
	s.mu.Unlock()

	s.conn.SetWriteDeadline(time.Now().Add(writeTimeout))
	n, err := s.conn.Write(buf)
	if n > 0 {
		s.lastSend = time.Now()
		s.bytesOut += uint64(n)
	}
	return err
}

var (
	subs   []*subscriber
	subsMu sync.RWMutex
)

func addSub(conn net.Conn) *subscriber {
	if tc, ok := conn.(*net.TCPConn); ok {
		tc.SetKeepAlive(true)
		tc.SetKeepAlivePeriod(30 * time.Second)
	}
	s := &subscriber{
		conn:     conn,
		sendq:    make([]byte, 0, sendqMax/4),
		lastSend: time.Now(),
	}
	subsMu.Lock()
	subs = append(subs, s)
	subsMu.Unlock()
	return s
}

func removeSub(target *subscriber) {
	subsMu.Lock()
	for i, s := range subs {
		if s == target {
			subs = append(subs[:i], subs[i+1:]...)
			break
		}
	}
	subsMu.Unlock()
	target.conn.Close()
}

func broadcast(data []byte) {
	subsMu.RLock()
	for _, s := range subs {
		s.enqueue(data)
	}
	subsMu.RUnlock()
}

// Flush loop: periodically push sendq to each client
func flushLoop() {
	ticker := time.NewTicker(50 * time.Millisecond)
	hbTicker := time.NewTicker(heartbeatIdle)
	for {
		select {
		case <-ticker.C:
			subsMu.RLock()
			for _, s := range subs {
				if err := s.flush(); err != nil {
					go removeSub(s)
				}
			}
			subsMu.RUnlock()
		case <-hbTicker.C:
			// Send heartbeat to idle clients
			subsMu.RLock()
			for _, s := range subs {
				if time.Since(s.lastSend) > heartbeatIdle {
					s.enqueue(beastHeartbeat)
				}
			}
			subsMu.RUnlock()
		}
	}
}

func connectUpstream(addr string) {
	for {
		fmt.Printf("[fanout] connecting to %s...\n", addr)
		conn, err := net.DialTimeout("tcp", addr, 10*time.Second)
		if err != nil {
			fmt.Printf("[fanout] connect failed: %v\n", err)
			time.Sleep(5 * time.Second)
			continue
		}
		fmt.Printf("[fanout] upstream connected\n")
		if tc, ok := conn.(*net.TCPConn); ok {
			tc.SetKeepAlive(true)
			tc.SetKeepAlivePeriod(30 * time.Second)
		}

		buf := make([]byte, readBuf)
		for {
			n, err := conn.Read(buf)
			if n > 0 {
				data := make([]byte, n)
				copy(data, buf[:n])
				broadcast(data)
			}
			if err != nil {
				fmt.Printf("[fanout] upstream lost: %v\n", err)
				conn.Close()
				break
			}
		}
		time.Sleep(5 * time.Second)
	}
}

func main() {
	host := os.Getenv("UPSTREAM_HOST")
	port := os.Getenv("UPSTREAM_PORT")
	listen := os.Getenv("LISTEN_PORT")
	if host == "" {
		host = "skylink"
	}
	if port == "" {
		port = "30005"
	}
	if listen == "" {
		listen = "40004"
	}

	ln, err := net.Listen("tcp", ":"+listen)
	if err != nil {
		fmt.Fprintf(os.Stderr, "listen failed: %v\n", err)
		os.Exit(1)
	}
	fmt.Printf("[fanout] listening :%s → upstream %s:%s\n", listen, host, port)

	go flushLoop()

	go func() {
		for {
			conn, err := ln.Accept()
			if err != nil {
				continue
			}
			s := addSub(conn)
			fmt.Printf("[fanout] +sub %s (total %d)\n", conn.RemoteAddr(), len(subs))
			go func() {
				buf := make([]byte, 1)
				for {
					conn.SetReadDeadline(time.Now().Add(5 * time.Minute))
					if _, err := conn.Read(buf); err != nil {
						fmt.Printf("[fanout] -sub %s\n", conn.RemoteAddr())
						removeSub(s)
						return
					}
				}
			}()
		}
	}()

	// Stats
	go func() {
		for range time.Tick(60 * time.Second) {
			subsMu.RLock()
			fmt.Printf("[fanout] subs=%d\n", len(subs))
			subsMu.RUnlock()
		}
	}()

	connectUpstream(host + ":" + port)
}
