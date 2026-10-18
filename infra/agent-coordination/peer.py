"""An Ubuntu-side deterministic peer, using the real HTTP/SSE transports.

Private request input arrives on stdin, never in command arguments or logs.
"""
import json
import sys
import urllib.error
import urllib.request


def main():
    data = json.load(sys.stdin)
    headers = {"Content-Type": "application/json"}
    if data.get("token"):
        headers["Authorization"] = "Bearer " + data["token"]
    headers.update(data.get("headers", {}))
    body = data.get("body")
    request = urllib.request.Request(
        "http://127.0.0.1:18787" + data["path"],
        method=data.get("method", "GET"), headers=headers,
        data=json.dumps(body).encode() if body is not None else None,
    )
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        response = opener.open(request, timeout=20)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        if data.get("stream") and response.status == 200:
            event = {}
            for raw in response:
                line = raw.decode().rstrip("\r\n")
                if line.startswith("id:"):
                    event["id"] = line[3:].strip()
                elif line.startswith("data:"):
                    event["data"] = json.loads(line[5:].strip())
                elif not line and "data" in event:
                    print(json.dumps({"status": 200, "event": event}), flush=True)
                    return
            raise RuntimeError("fixture_stream_closed_without_event")
        raw = response.read()
        print(json.dumps({"status": response.status, "body": json.loads(raw) if raw else None}))


if __name__ == "__main__":
    try:
        main()
    except Exception:
        # Do not print request data, credentials or unbounded driver errors.
        print(json.dumps({"error": "ubuntu_peer_transport_failed"}))
        sys.exit(1)
