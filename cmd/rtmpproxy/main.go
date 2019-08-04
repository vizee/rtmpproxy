package main

import (
	"flag"
	"fmt"
	"log"
	"net"
	"net/url"
	"strings"
	"time"

	"github.com/vizee/rtmpproxy"
)

func main() {
	var (
		puburl     string
		listenAddr string
	)
	flag.StringVar(&puburl, "p", "rtmp://hostname/app/?args", "rtmp url")
	flag.StringVar(&listenAddr, "l", ":1935", "listen address")
	flag.Parse()

	u, err := url.Parse(puburl)
	if err != nil {
		log.Fatalf("url parse failed: %v", err)
	}
	host, port, _ := net.SplitHostPort(u.Host)
	if port == "" {
		host = u.Host
		port = "1935"
	}
	appName := strings.Trim(u.Path, "/")
	s := rtmpproxy.NewServer(net.JoinHostPort(host, port), appName, fmt.Sprintf("rtmp://%s/%s", u.Host, appName), "?"+u.RawQuery)

	ln, err := net.Listen("tcp", listenAddr)
	if err != nil {
		log.Fatalf("listen failed: %v", err)
	}
	for {
		conn, err := ln.Accept()
		if err != nil {
			log.Printf("accept failed: %v", err)
			time.Sleep(time.Second)
			continue
		}
		go func(conn net.Conn) {
			err := s.Serve(conn)
			if err != nil {
				log.Printf("serve: %v", err)
			}
		}(conn)
	}
}
