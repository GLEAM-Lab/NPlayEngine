"""NPlayEngine Python SDK: call npe.run(think), where think(obs) returns a dict."""
import json
import sys


def run(think):
    wire, sys.stdout = sys.stdout, sys.stderr  # print() goes to the debug log
    for line in sys.stdin:
        msg = json.loads(line)
        if msg["type"] == "init":
            out = {"ready": True}
        elif msg["type"] == "turn":
            out = {"turn": msg["turn"], "action": think(msg["obs"])}
        else:
            continue
        wire.write(json.dumps(out) + "\n")
        wire.flush()
