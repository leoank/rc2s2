"""A stand-in for Anthropic's usage endpoint: answers GET /api/oauth/usage with the saved
response shape when the bearer equals EXPECT_TOKEN and the oauth beta header is present,
401 otherwise. Logs one line per request, never the token."""
import http.server, json, os, re, sys
shape = open(sys.argv[1]).read()
body = shape[shape.index("{"):]
expect = os.environ["EXPECT_TOKEN"]

class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        auth = self.headers.get("Authorization", "")
        beta = self.headers.get("anthropic-beta", "")
        if self.path == "/api/oauth/usage" and auth == f"Bearer {expect}" and "oauth-2025-04-20" in beta:
            print("ok bearer matched, beta header present", flush=True)
            data = body.encode()
            self.send_response(200); self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data))); self.end_headers(); self.wfile.write(data)
        else:
            print(f"refused path={self.path} bearer_matched={auth == f'Bearer {expect}'} beta={bool(beta)}", flush=True)
            self.send_response(401); self.end_headers()
    def log_message(self, *a): pass

http.server.HTTPServer(("127.0.0.1", 7799), H).serve_forever()
