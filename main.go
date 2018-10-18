package main

import (
	"bytes"
	"encoding/binary"
	"flag"
	"fmt"
	"io"
	"log"
	"net"
	"net/url"
	"strings"
	"time"

	amf "github.com/zhangpeihao/goamf"
)

var config struct {
	remoteAddr string
	playUrl    string
	appName    string
	streamName string
}

var verbose bool

func verbosef(format string, args ...interface{}) {
	if verbose {
		log.Printf(format, args...)
	}
}

func handleRtmpCommand(ch *rtmpChunkHeader, payload []byte) ([]byte, bool, error) {
	br := bytes.NewReader(payload)
	command, err := amf.ReadString(br)
	if err != nil {
		return nil, false, err
	}
	transid, err := amf.ReadDouble(br)
	if err != nil {
		return nil, false, err
	}
	args := make([]interface{}, 0, 1)
	for br.Len() > 0 {
		v, err := amf.ReadValue(br)
		if err != nil {
			return nil, false, err
		}
		args = append(args, v)
	}
	verbosef("[rtmp] command %s transid %v args %v", command, transid, args)

	usecopy := false
	switch command {
	case "connect":
		obj := args[0].(amf.Object)
		obj["app"] = config.appName
		obj["swfUrl"] = config.playUrl
		obj["tcUrl"] = config.playUrl
	case "releaseStream", "FCPublish":
		args[1] = config.streamName
	case "publish":
		args[1] = config.streamName
		usecopy = true
	}
	verbosef("[rtmp] new command args: %v", args)
	buf := bytes.NewBuffer(nil)
	amf.WriteString(buf, command)
	amf.WriteDouble(buf, transid)
	for _, arg := range args {
		amf.WriteValue(buf, arg)
	}
	return buf.Bytes(), usecopy, nil
}

func serveConn(conn net.Conn) {
	defer conn.Close()
	conn2, err := net.Dial("tcp", config.remoteAddr)
	if err != nil {
		log.Printf("[rtmp] dial remote failed: %v", err)
		return
	}
	defer conn2.Close()

	err = shadowHandshake(conn, conn2)
	if err != nil {
		log.Printf("[rtmp] handshake failed: %v", err)
		return
	}
	log.Printf("[rtmp] handshake done")

	go func() {
		_, err := io.Copy(conn, conn2)
		log.Printf("[rtmp] copy target to source: %v", err)
		conn.Close()
		conn2.Close()
	}()

	var (
		maxChunkSize = 128
		usecopy      = false

		lastch  rtmpChunkHeader
		payload []byte
		nread   int
	)

	for !usecopy {
		ch, err := rtmpReadHeader(conn)
		if err != nil {
			log.Printf("[rtmp] read header failed: %v", err)
			return
		}
		verbosef("[rtmp] chunk-header: %+v", ch)
		if nread != 0 && ch.csid != lastch.csid {
			log.Printf("[rtmp] unsupport multi-chunkstream at a time")
			return
		}

		switch ch.format {
		case 1:
			ch.streamid = lastch.streamid
		case 2:
			ch.length = lastch.length
			ch.typeid = lastch.typeid
			ch.streamid = lastch.streamid
		case 3:
			ch.timestamp = lastch.timestamp
			ch.length = lastch.length
			ch.typeid = lastch.typeid
			ch.streamid = lastch.streamid
		}
		lastch = *ch

		if len(payload) != int(ch.length) {
			payload = make([]byte, ch.length)
		}

		n := maxChunkSize
		if rem := len(payload) - nread; rem < maxChunkSize {
			n = rem
		}

		_, err = io.ReadFull(conn, payload[nread:nread+n])
		if err != nil {
			log.Printf("[rtmp] read payload failed: %v", err)
			return
		}
		nread += n
		if nread < len(payload) {
			continue
		}

		verbosef("[rtmp] payload: %02x", payload)

		switch ch.typeid {
		case 1:
			if len(payload) != 4 {
				log.Printf("[rtmp] invalid type 0 payload")
				return
			}
			maxChunkSize = int(binary.BigEndian.Uint32(payload))
			if maxChunkSize <= 0 {
				log.Printf("[rtmp] invalid chunk size: %v", maxChunkSize)
			}
		case 20:
			payload, usecopy, err = handleRtmpCommand(ch, payload)
			if err != nil {
				log.Printf("[rtmp] handle command failed: %v", err)
				return
			}
		}
		err = writeRtmpMessage(conn2, ch, payload, maxChunkSize)
		if err != nil {
			log.Printf("[rtmp] write message failed: %v", err)
			return
		}
		payload = nil
		nread = 0
	}
	verbosef("[rtmp] direct copy")
	_, err = io.Copy(conn2, conn)
	log.Printf("[rtmp] copy source to target: %v", err)
}

func main() {
	var (
		puburl     string
		listenAddr string
	)
	flag.StringVar(&puburl, "p", "rtmp://hostname/app/?args", "rtmp url")
	flag.StringVar(&listenAddr, "l", ":1935", "listen address")
	flag.BoolVar(&verbose, "V", false, "verbose")
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
	config.remoteAddr = net.JoinHostPort(host, port)
	config.appName = strings.Trim(u.Path, "/")
	config.playUrl = fmt.Sprintf("rtmp://%s/%s", u.Host, config.appName)
	config.streamName = "?" + u.RawQuery

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
		go serveConn(conn)
	}
}
