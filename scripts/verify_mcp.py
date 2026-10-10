#!/usr/bin/env python3
"""Black-box acceptance: the real AIGE binary, JSON-RPC, GPU and replay."""
import base64
import json
import pathlib
import select
import subprocess
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
BINARY = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "dist/neon-clash"
ARTIFACTS = ROOT / "artifacts"
ARTIFACTS.mkdir(exist_ok=True)


class Client:
    def __init__(self):
        self.err = open(ARTIFACTS / "mcp-stderr.log", "w")
        self.p = subprocess.Popen([str(BINARY), "--headless", "--agent", "stdio", "--seed", "42"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.err, bufsize=0)
        self.serial = 0
        self.buffer = b""
        self.rpc("initialize", {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "neon-clash-acceptance", "version": "1"}})
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def send(self, message):
        self.p.stdin.write((json.dumps(message) + "\n").encode())
        self.p.stdin.flush()

    def rpc(self, method, params):
        self.serial += 1
        self.send({"jsonrpc": "2.0", "id": self.serial, "method": method, "params": params})
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            if b"\n" not in self.buffer:
                ready, _, _ = select.select([self.p.stdout], [], [], 1)
                if not ready:
                    assert self.p.poll() is None, "game exited before response"
                    continue
                data = self.p.stdout.read(65536)
                assert data, "unexpected end of MCP stream"
                self.buffer += data
                continue
            line, self.buffer = self.buffer.split(b"\n", 1)
            result = json.loads(line)
            if result.get("id") != self.serial:
                continue
            assert "error" not in result, result
            return result["result"]
        raise TimeoutError(method)

    def tool(self, name, args=None):
        result = self.rpc("tools/call", {"name": name, "arguments": args or {}})
        assert not result.get("isError"), result
        return result

    def value(self, name, args=None):
        return self.tool(name, args)["structuredContent"]

    def game(self):
        return self.value("get_resource", {"name": "game"})["value"]

    def step(self, ticks, inputs=None):
        args = {"ticks": ticks}
        if inputs:
            args["inputs"] = inputs
        return self.value("step", args)

    def tap(self, key, ticks=2):
        return self.step(ticks, [event(0, key, True), event(1, key, False)])

    def shot(self, name):
        result = self.tool("screenshot", {"width": 1920, "height": 1080})
        image = next(c for c in result["content"] if c["type"] == "image")
        data = base64.b64decode(image["data"])
        assert data[:8] == b"\x89PNG\r\n\x1a\n"
        assert int.from_bytes(data[16:20], "big") == 1920
        assert int.from_bytes(data[20:24], "big") == 1080
        (ARTIFACTS / f"mcp-{name}.png").write_bytes(data)

    def close(self):
        self.p.stdin.close()
        try:
            self.p.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.p.terminate()
            self.p.wait(timeout=5)
        self.err.close()


def event(at, key, pressed):
    return {"at": at, "event": {"type": "Key", "key": key, "pressed": pressed}}


c = Client()
try:
    status = c.value("status")
    assert status["paused"] is True and status["tick"] == 0
    assert {"info", "match_finished", "hit_connected"}.issubset(status["probes"])
    assert len(c.rpc("tools/list", {})["tools"]) == 20
    c.shot("title")
    c.tap("Down"); c.tap("Down"); c.tap("Enter")
    assert c.game()["mode"] == "Training"
    c.shot("select")
    c.tap("Enter"); c.tap("Enter"); c.tap("Enter")
    assert c.game()["phase"] == "Fight"
    c.step(70, [event(0, "D", True)])
    c.step(1, [event(0, "D", False)])
    initial_health = c.game()["fighters"][1]["health"]
    for key, ticks in [("J", 6), ("J", 13), ("K", 20), ("U", 27)]:
        c.tap(key, ticks)
    g = c.game()
    assert g["fighters"][0]["combo_hits"] == 4, g["fighters"]
    assert g["fighters"][1]["health"] < initial_health
    c.shot("combo")
    snap = c.value("snapshot", {"label": "combo"})
    c.step(25)
    restored = c.value("restore", {"id": snap["id"]})
    assert restored["hash"] == snap["hash"]
    c.value("replay_save", {"path": str(ARTIFACTS / "acceptance-replay.json")})
    c.tap("F")
    before = c.game()["fight_tick"]
    c.step(30)
    assert c.game()["fight_tick"] == before
    c.shot("manual")
    report = {"transport": "MCP stdio / real subprocess", "seed": 42, "tools": 20, "combo_hits": g["fighters"][0]["combo_hits"], "combo_damage": g["fighters"][0]["combo_damage"], "snapshot_hash_restored": True, "GPU_screenshots": 4}
    report["resolution"] = [1920, 1080]
finally:
    c.close()

verified = subprocess.run([str(BINARY), "--replay", str(ARTIFACTS / "acceptance-replay.json")], capture_output=True, text=True, timeout=30)
assert verified.returncode == 0, verified.stderr
assert "replay verified" in verified.stderr + verified.stdout
report["replay"] = (verified.stderr + verified.stdout).strip()
(ARTIFACTS / "mcp-report.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
