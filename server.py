#!/usr/bin/env python3
"""Pan UI Server — Local Dual-Control Interface Server with SSE Event Broadcasting."""

import http.server
import json
import os
import queue
import socketserver
import sys
import threading
import time
from urllib.parse import urlparse

PORT = int(os.environ.get("PAN_UI_PORT", 7401))
PUBLIC_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "public")

# SSE Subscriber queues
CLIENT_QUEUES = set()
CLIENTS_LOCK = threading.Lock()

def broadcast_event(data: dict):
    """Broadcast an event payload to all connected browser SSE clients."""
    payload = f"data: {json.dumps(data)}\n\n"
    with CLIENTS_LOCK:
        for q in list(CLIENT_QUEUES):
            try:
                q.put_nowait(payload)
            except Exception:
                CLIENT_QUEUES.discard(q)

class PanUIHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=PUBLIC_DIR, **kwargs)

    def do_GET(self):
        parsed = urlparse(self.path)

        # ── SSE Stream ──────────────────────────────────────────
        if parsed.path == "/api/events":
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Cache-Control", "no-cache")
            self.send_header("Connection", "keep-alive")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()

            q = queue.Queue()
            with CLIENTS_LOCK:
                CLIENT_QUEUES.add(q)
                print(f"[SSE] Browser client connected. Total clients: {len(CLIENT_QUEUES)}")

            # Send welcome ping
            welcome = f"data: {json.dumps({'type': 'init', 'msg': 'Connected to Pan UI dual-control server'})}\n\n"
            try:
                self.wfile.write(welcome.encode("utf-8"))
                self.wfile.flush()

                while True:
                    try:
                        msg = q.get(timeout=20.0)
                        self.wfile.write(msg.encode("utf-8"))
                        self.wfile.flush()
                    except queue.Empty:
                        # Heartbeat ping
                        self.wfile.write(b": keepalive\n\n")
                        self.wfile.flush()
            except (ConnectionResetError, BrokenPipeError):
                pass
            finally:
                with CLIENTS_LOCK:
                    CLIENT_QUEUES.discard(q)
                    print(f"[SSE] Browser client disconnected. Remaining: {len(CLIENT_QUEUES)}")
            return

        # Fall back to static file server
        super().do_GET()

    def do_POST(self):
        parsed = urlparse(self.path)

        if parsed.path == "/api/cast":
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length)
            try:
                data = json.loads(body.decode("utf-8"))
                print(f"[CAST] Received event from CLI/Agent: {data}")
                broadcast_event(data)

                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Access-Control-Allow-Origin", "*")
                self.end_headers()
                self.wfile.write(json.dumps({"status": "ok", "delivered_to": len(CLIENT_QUEUES)}).encode("utf-8"))
            except Exception as e:
                self.send_response(400)
                self.end_headers()
                self.wfile.write(json.dumps({"error": str(e)}).encode("utf-8"))
            return

        self.send_response(404)
        self.end_headers()

class ThreadingHTTPServer(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True

def run():
    with ThreadingHTTPServer(("127.0.0.1", PORT), PanUIHandler) as httpd:
        print("=" * 65)
        print(f"  ⚡ Pan UI Dual-Control Server running at http://127.0.0.1:{PORT}")
        print("  Control live from CLI using: ./pansee cast ...")
        print("=" * 65)
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nShutting down server.")

if __name__ == "__main__":
    run()
