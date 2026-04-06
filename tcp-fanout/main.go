package main

// Beast splitter: accepts N feeders on ingest port, broadcasts to N subscribers on output port
// Per-client sendq, drop-half for slow subscribers, heartbeat on idle

import (
	"fmt"
	"net"
	"os"
	"sync"
	"sync/atomic"
	"time"
)

const (
	sendqMax      = 256 * 1024
	readBuf       = 16384
	writeTimeout  = 5 * time.Second
	heartbeatIdle = 30 * time.Second
	dropWindow    = 2 * time.Second
)

var beastHeartbeat = []byte{0x1a, 0x31, 0, 0, 0, 0, 0, 0, 0, 0, 0}

type subscriber struct {
	conn      net.Conn
	sendq     []byte
	mu        sync.Mutex
	lastSend  time.Time
	dropUntil time.Time
	dropFlip  bool
}

func (s *subscriber) enqueue(data []byte) {
	s.mu.Lock()
	defer s.mu.Unlock()
	now := time.Now()
	if now.Before(s.dropUntil) {
		s.dropFlip = !s.dropFlip
		if s.dropFlip {
			return
		}
	}
	if len(s.sendq)+len(data) > sendqMax {
		s.dropUntil = now.Add(dropWindow)
		s.dropFlip = true
		return
	}
	s.sendq = append(s.sendq, data...)
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
	_, err := s.conn.Write(buf)
	if err == nil {
		s.lastSend = time.Now()
	}
	return err
}

var (
	subs      []*subscriber
	subsMu    sync.RWMutex
	feedCount atomic.Int32
	bytesIn   atomic.Uint64
)

func broadcast(data []byte) {
	subsMu.RLock()
	for _, s := range subs {
		s.enqueue(data)
	}
	subsMu.RUnlock()
}

func addSub(conn net.Conn) *subscriber {
	if tc, ok := conn.(*net.TCPConn); ok {
		tc.SetKeepAlive(true)
		tc.SetKeepAlivePeriod(30 * time.Second)
	}
	s := &subscriber{conn: conn, sendq: make([]byte, 0, sendqMax/4), lastSend: time.Now()}
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

func handleFeeder(conn net.Conn) {
	addr := conn.RemoteAddr().String()
	feedCount.Add(1)
	if tc, ok := conn.(*net.TCPConn); ok {
		tc.SetKeepAlive(true)
		tc.SetKeepAlivePeriod(30 * time.Second)
	}
	// Send Beast heartbeat on connect so feeder knows we're alive
	conn.SetWriteDeadline(time.Now().Add(5 * time.Second))
	conn.Write(beastHeartbeat)
	buf := make([]byte, readBuf)
	for {
		n, err := conn.Read(buf)
		if n > 0 {
			bytesIn.Add(uint64(n))
			data := make([]byte, n)
			copy(data, buf[:n])
			broadcast(data)
		}
		if err != nil {
			feedCount.Add(-1)
			conn.Close()
			fmt.Printf("[splitter] feeder lost: %s\n", addr)
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

	ingestLn, err := net.Listen("tcp", ":"+ingestPort)
	if err != nil {
		fmt.Fprintf(os.Stderr, "ingest listen failed: %v\n", err)
		os.Exit(1)
	}

	subLn, err := net.Listen("tcp", ":"+listenPort)
	if err != nil {
		fmt.Fprintf(os.Stderr, "output listen failed: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("[splitter] ingest :%s → output :%s\n", ingestPort, listenPort)

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
			s := addSub(conn)
			fmt.Printf("[splitter] +sub %s (total %d)\n", conn.RemoteAddr(), len(subs))
			go func() {
				buf := make([]byte, 1)
				for {
					conn.SetReadDeadline(time.Now().Add(5 * time.Minute))
					if _, err := conn.Read(buf); err != nil {
						fmt.Printf("[splitter] -sub %s\n", conn.RemoteAddr())
						removeSub(s)
						return
					}
				}
			}()
		}
	}()

	// Flush + heartbeat + stats
	flushTick := time.NewTicker(50 * time.Millisecond)
	statsTick := time.NewTicker(60 * time.Second)
	for {
		select {
		case <-flushTick.C:
			subsMu.RLock()
			for _, s := range subs {
				if err := s.flush(); err != nil {
					go removeSub(s)
				}
			}
			subsMu.RUnlock()
		case <-statsTick.C:
			subsMu.RLock()
			n := len(subs)
			subsMu.RUnlock()
			fmt.Printf("[splitter] feeders=%d subs=%d in=%.1fMB\n",
				feedCount.Load(), n, float64(bytesIn.Load())/1048576)
		}
	}
}
