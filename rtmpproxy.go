package rtmpproxy

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"io"
	"net"

	amf "github.com/zhangpeihao/goamf"
)

type Server struct {
	verbose    bool
	remoteAddr string
	playUrl    string
	appName    string
	streamName string
}

func (s *Server) handleRtmpCommand(ch *rtmpChunkHeader, payload []byte) ([]byte, bool, error) {
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

	usecopy := false
	switch command {
	case "connect":
		obj := args[0].(amf.Object)
		obj["app"] = s.appName
		obj["swfUrl"] = s.playUrl
		obj["tcUrl"] = s.playUrl
	case "releaseStream", "FCPublish":
		args[1] = s.streamName
	case "publish":
		args[1] = s.streamName
		usecopy = true
	}
	buf := bytes.NewBuffer(nil)
	amf.WriteString(buf, command)
	amf.WriteDouble(buf, transid)
	for _, arg := range args {
		amf.WriteValue(buf, arg)
	}
	return buf.Bytes(), usecopy, nil
}

func (s *Server) handleMessages(conn net.Conn, conn2 net.Conn) error {
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
			return err
		}
		if nread != 0 && ch.csid != lastch.csid {
			return fmt.Errorf("unsupport multi-chunkstream at a time")
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
			return err
		}
		nread += n
		if nread < len(payload) {
			continue
		}

		switch ch.typeid {
		case 1:
			if len(payload) != 4 {
				return fmt.Errorf("invalid type 0 payload size: %d", len(payload))
			}
			maxChunkSize = int(binary.BigEndian.Uint32(payload))
			if maxChunkSize <= 0 {
				return fmt.Errorf("invalid chunk size: %d", maxChunkSize)
			}
		case 20:
			payload, usecopy, err = s.handleRtmpCommand(ch, payload)
			if err != nil {
				return err
			}
		}
		err = writeRtmpMessage(conn2, ch, payload, maxChunkSize)
		if err != nil {
			return err
		}
		payload = nil
		nread = 0
	}
	return nil
}

func (s *Server) Serve(conn net.Conn) error {
	defer conn.Close()
	conn2, err := net.Dial("tcp", s.remoteAddr)
	if err != nil {
		return err
	}
	defer conn2.Close()

	err = shadowHandshake(conn, conn2)
	if err != nil {
		return err
	}

	go func() {
		io.Copy(conn, conn2)
		conn.Close()
		conn2.Close()
	}()

	err = s.handleMessages(conn, conn2)
	if err != nil {
		return err
	}
	_, err = io.Copy(conn2, conn)
	return err
}

func NewServer(remoteAddr string, appName string, playUrl string, streamName string) *Server {
	return &Server{
		remoteAddr: remoteAddr,
		appName:    appName,
		playUrl:    playUrl,
		streamName: streamName,
	}
}
