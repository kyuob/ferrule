#!/bin/sh
# Host HTTPS transport/bridge checks. Optional argument reuses a disposable combination fixture.
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
cd "$FERRULE_ROOT"
cargo build --locked --quiet
mkdir -p .work
NETWORK_ROOT=$(mktemp -d "$FERRULE_ROOT/.work/network-XXXXXX")
echo "Network artifacts: $NETWORK_ROOT"
openssl req -x509 -newkey rsa:2048 -nodes -keyout "$NETWORK_ROOT/ca-key.pem" \
  -out "$NETWORK_ROOT/cert.pem" -days 1 -subj '/CN=Ferrule disposable test CA' \
  -addext 'basicConstraints=critical,CA:TRUE' \
  > "$NETWORK_ROOT/certificate.log" 2>&1
openssl req -newkey rsa:2048 -nodes -keyout "$NETWORK_ROOT/key.pem" \
  -out "$NETWORK_ROOT/server.csr" -subj '/CN=localhost' >> "$NETWORK_ROOT/certificate.log" 2>&1
cat > "$NETWORK_ROOT/extensions.cnf" <<'EXT'
subjectAltName=DNS:localhost
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
EXT
openssl x509 -req -in "$NETWORK_ROOT/server.csr" -CA "$NETWORK_ROOT/cert.pem" \
  -CAkey "$NETWORK_ROOT/ca-key.pem" -CAcreateserial -out "$NETWORK_ROOT/server.pem" \
  -days 1 -extfile "$NETWORK_ROOT/extensions.cnf" >> "$NETWORK_ROOT/certificate.log" 2>&1
python3 - "$NETWORK_ROOT" <<'PY' &
import http.server, pathlib, ssl, sys, time
root = pathlib.Path(sys.argv[1])
class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def do_GET(self):
        if self.path == '/slow': time.sleep(11)
        body = b'hello TLS'
        if self.path in ('/large', '/stream'): body = b'x' * 65537
        if self.path == '/binary': body = b'\xff'
        self.send_response(302 if self.path == '/redirect' else 500 if self.path == '/error' else 200)
        if self.path == '/redirect': self.send_header('Location', 'http://localhost/never-follow')
        if self.path != '/stream': self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        try:
            if self.path == '/slow_body': time.sleep(11)
            if self.path == '/drip':
                for byte in body:
                    self.wfile.write(bytes([byte])); self.wfile.flush(); time.sleep(2)
            else: self.wfile.write(body)
        except (OSError, ssl.SSLError): pass
class Server(http.server.ThreadingHTTPServer):
    def handle_error(self, *args): pass
server = Server(('127.0.0.1', 0), Handler)
tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
tls.load_cert_chain(root/'server.pem', root/'key.pem')
server.socket = tls.wrap_socket(server.socket, server_side=True)
(root/'port').write_text(str(server.server_port))
server.serve_forever()
PY
NETWORK_SERVER_PID=$!
trap 'kill "$NETWORK_SERVER_PID" 2>/dev/null || true' EXIT INT TERM
NETWORK_WAIT=0
while [ ! -f "$NETWORK_ROOT/port" ]; do
  NETWORK_WAIT=$((NETWORK_WAIT + 1))
  if [ "$NETWORK_WAIT" -ge 10 ]; then echo 'TLS fixture failed to start' >&2; exit 1; fi
  sleep 1
done
export FERRULE_TLS_ORIGIN="https://localhost:$(cat "$NETWORK_ROOT/port")"
export FERRULE_TLS_CERT="$NETWORK_ROOT/cert.pem"
FERRULE="$FERRULE_ROOT/target/debug/ferrule"
if [ "${1:-}" = --check ]; then
  sh "$FERRULE_ROOT/scripts/network-app-check.sh" "${2:?Usage: network-smoke.sh --check SYNCHRONIZED_APP}"
  echo 'PASS: existing synchronized network app host checks'
  exit 0
fi
"$FERRULE" new network_app --modules network --directory "$NETWORK_ROOT"
"$FERRULE" sync --project "$NETWORK_ROOT/network_app"
sh "$FERRULE_ROOT/scripts/network-app-check.sh" "$NETWORK_ROOT/network_app"
if [ "$#" -gt 0 ]; then
  NETWORK_COMBINED=$1
  "$FERRULE" add network --project "$NETWORK_COMBINED"
else
  NETWORK_COMBINED="$NETWORK_ROOT/network_app"
  "$FERRULE" add ble,data,security --project "$NETWORK_COMBINED"
fi
"$FERRULE" sync --project "$NETWORK_COMBINED"
sh "$FERRULE_ROOT/scripts/network-app-check.sh" "$NETWORK_COMBINED"
echo "PASS: network-only and combined app host checks; combined project: $NETWORK_COMBINED"
