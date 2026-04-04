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
	clientsMu sync.Mutex
)

func addClient(c net.Conn) {
	clientsMu.Lock()
	clients[c] = struct{}{}
	clientsMu.Unlock()
}

func removeClient(c net.Conn) {
	clientsMu.Lock()
	delete(clients, c)
	clientsMu.Unlock()
	c.Close()
}

func broadcast(data []byte) {
	clientsMu.Lock()
	defer clientsMu.Unlock()
	for c := range clients {
		_, err := c.Write(data)
		if err != nil {
			c.Close()
			delete(clients, c)
		}
	}
}

func handleUpstream(addr string) {
	for {
		fmt.Printf("Connecting to upstream %s...\n", addr)
		upstream, err := net.Dial("tcp", addr)
		if err != nil {
			fmt.Printf("Upstream connect failed: %v. Retrying in 5s...\n", err)
			time.Sleep(5 * time.Second)
			continue
		}
		fmt.Println("Upstream connected.")

		buf := make([]byte, 4096)
		for {
			n, err := upstream.Read(buf)
			if n > 0 {
				// Create copy before broadcasting to avoid buffer sharing issues
				dataCopy := make([]byte, n)
				copy(dataCopy, buf[:n])
				broadcast(dataCopy)
			}
			if err != nil {
				if err == io.EOF {
					fmt.Println("Upstream closed. Reconnecting...")
				} else {
					fmt.Printf("Upstream error: %v. Reconnecting...\n", err)
				}
				upstream.Close()
				break
			}
		}
		time.Sleep(5 * time.Second) // wait before reconnect
	}
}

func main() {
	// Read env vars
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

	// Local listen for subscribers
	ln, err := net.Listen("tcp", listenAddr)
	if err != nil {
		panic(fmt.Sprintf("failed to listen: %v", err))
	}
	defer ln.Close()
	fmt.Printf("Listening on %s for subscribers\n", listenAddr)

	// Accept clients in background
	go func() {
		for {
			conn, err := ln.Accept()
			if err != nil {
				continue
			}
			addClient(conn)
			fmt.Printf("Client connected: %s\n", conn.RemoteAddr())

			go func(c net.Conn) {
				buf := make([]byte, 1)
				for {
					_, err := c.Read(buf)
					if err != nil {
						fmt.Printf("Client disconnected: %s\n", c.RemoteAddr())
						removeClient(c)
						return
					}
				}
			}(conn)
		}
	}()

	// Run upstream loop
	handleUpstream(upstreamAddr)
}