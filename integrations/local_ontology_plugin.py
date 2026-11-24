"""Drop-in adapter: exposes Metro Notch tasks as a local-ontology data source.

Usage (local-ontology backend):
  from integrations.local_ontology_plugin import MetroNotchSource
  src = MetroNotchSource()  # reads http://127.0.0.1:7878
  tasks = src.list_tasks()
  src.sync(workspace_id="default")
"""
import json
import urllib.request

BASE = "http://127.0.0.1:7878"


def _get(path):
    with urllib.request.urlopen(BASE + path, timeout=5) as r:
        return json.loads(r.read().decode())


def _post(path, data):
    req = urllib.request.Request(
        BASE + path,
        data=json.dumps(data).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=5) as r:
        return json.loads(r.read().decode())


class MetroNotchSource:
    def __init__(self, base=BASE):
        self.base = base.rstrip("/")

    def health(self):
        return _get("/health") if self.base == BASE else json.loads(
            urllib.request.urlopen(self.base + "/health", timeout=5).read()
        )

    def list_tasks(self):
        return _get("/api/tasks")

    def get_state(self):
        return _get("/api/state")

    def add_task(self, text):
        return _post("/api/tasks", {"text": text})

    def sync_to_ontology(self, onto_ingest_url, workspace=None):
        """Push pending tasks into local-ontology as a text document via its MCP/REST.

        onto_ingest_url: e.g. http://localhost:8000/api/workspaces/{ws}/ingest/text
        """
        tasks = [t for t in self.list_tasks() if not t.get("done")]
        text = "\n".join(f"- {t.get('name') or t.get('text','')}" for t in tasks) or "(sem tarefas pendentes)"
        payload = {"text": text, "source": "metro-notch"}
        req = urllib.request.Request(
            onto_ingest_url,
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=30) as r:
            return json.loads(r.read().decode())