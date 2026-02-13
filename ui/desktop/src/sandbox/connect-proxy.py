#!/usr/bin/env python3
import sys
import os
import socket
import select

if len(sys.argv) != 3:
    print("Usage: connect-proxy.py <host> <port>", file=sys.stderr)
    sys.exit(1)

host, port = sys.argv[1], sys.argv[2]
proxy_port = os.environ.get("SANDBOX_PROXY_PORT")
if not proxy_port:
    print("SANDBOX_PROXY_PORT not set", file=sys.stderr)
    sys.exit(1)

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.connect(("127.0.0.1", int(proxy_port)))

sock.sendall(f"CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n".encode())

f = sock.makefile("rb")
status_line = f.readline().decode()
if "200" not in status_line:
    print(f"Proxy error: {status_line.strip()}", file=sys.stderr)
    sys.exit(1)

while True:
    hdr = f.readline().decode()
    if hdr in ("\r\n", "\n", ""):
        break

stdin_fd = sys.stdin.buffer.fileno()
stdout_fd = sys.stdout.buffer.fileno()
sock_fd = sock.fileno()

while True:
    readable, _, _ = select.select([sock_fd, stdin_fd], [], [])
    for fd in readable:
        if fd == sock_fd:
            data = sock.recv(8192)
            if not data:
                sys.exit(0)
            os.write(stdout_fd, data)
        else:
            data = os.read(stdin_fd, 8192)
            if not data:
                sys.exit(0)
            sock.sendall(data)
