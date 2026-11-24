#!/usr/bin/env python3
"""One-shot sync: pending Metro Notch tasks -> a local-ontology document."""
import argparse
import json
import urllib.request

NOTCH = "http://127.0.0.1:7878"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--workspace", default="default", help="local-ontology workspace id")
    ap.add_argument("--ontology", default="http://localhost:8000", help="local-ontology base url")
    args = ap.parse_args()

    with urllib.request.urlopen(NOTCH + "/api/tasks", timeout=5) as r:
        tasks = json.loads(r.read())
    pending = [t for t in tasks if not t.get("done")]
    text = "\n".join(f"- {t['text']}" for t in pending) or "(sem tarefas pendentes)"

    url = f"{args.ontology}/api/workspaces/{args.workspace}/ingest/text"
    req = urllib.request.Request(
        url,
        data=json.dumps({"text": text, "source": "metro-notch"}).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=30) as r:
        print(r.read().decode())


if __name__ == "__main__":
    main()
