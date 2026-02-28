#!/usr/bin/env python3
"""Tiny static server for the NES web demo with explicit wasm/NES MIME types."""

from __future__ import annotations

import argparse
import http.server
import os
import socketserver
from pathlib import Path


class DemoRequestHandler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".nes": "application/octet-stream",
    }

    def __init__(self, *args, directory: str | None = None, **kwargs):
        super().__init__(*args, directory=directory, **kwargs)


def main() -> int:
    parser = argparse.ArgumentParser(description="Serve NES web demo assets.")
    parser.add_argument("--port", type=int, default=8080, help="listen port (default: 8080)")
    parser.add_argument(
        "--root",
        type=str,
        default=os.getcwd(),
        help="repo root to serve from (default: cwd)",
    )
    args = parser.parse_args()

    root = Path(args.root).resolve()
    os.chdir(root)

    with socketserver.TCPServer(("127.0.0.1", args.port), DemoRequestHandler) as httpd:
        print(f"NES web demo server: http://127.0.0.1:{args.port}/web/index.html")
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            pass
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
