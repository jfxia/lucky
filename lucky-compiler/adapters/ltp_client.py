#!/usr/bin/env python3
"""
Lucky Tool Protocol (LTP) Client — Python reference implementation.
Communicates with a Lucky LTP server over stdio or HTTP to execute Lucky programs.

Usage:
    from ltp_client import LtpClient

    # stdio mode (spawns server as subprocess)
    client = LtpClient.stdio(["lucky", "serve", "--transport", "stdio"])

    # HTTP mode (connects to running server)
    client = LtpClient.http("http://localhost:9700", token="my-token")

    # Initialize, load IR, execute
    client.initialize()
    client.load_ir_file("program.lir")
    result = client.execute(goal="BuildWebsite", mode="sync")
    print(f"Result: {result['result']}, Cost: ${result['cost']['total_usd']}")
    client.close()
"""

import json
import subprocess
import sys
import uuid
import urllib.request
import urllib.error
from typing import Any, Optional, Callable


class LtpError(Exception):
    """LTP protocol error."""
    def __init__(self, code: int, message: str, data: Any = None):
        self.code = code
        self.message = message
        self.data = data
        super().__init__(f"LTP Error {code}: {message}")


class LtpClient:
    """Client for the Lucky Tool Protocol."""

    def __init__(self):
        self._next_id = 1
        self._session_id: Optional[str] = None
        self._ir_loaded = False
        self._event_handlers: list[Callable] = []

    # ── Factory methods ────────────────────────────────────────

    @classmethod
    def stdio(cls, command: list[str]) -> "LtpClient":
        """Create a client that communicates over stdio with a subprocess."""
        client = cls()
        client._transport = "stdio"
        client._process = subprocess.Popen(
            command,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        client._stdin = client._process.stdin
        client._stdout = client._process.stdout
        return client

    @classmethod
    def http(cls, endpoint: str, token: Optional[str] = None) -> "LtpClient":
        """Create a client that communicates over HTTP."""
        client = cls()
        client._transport = "http"
        client._endpoint = endpoint.rstrip("/") + "/ltp/v1"
        client._token = token
        return client

    # ── Low-level RPC ──────────────────────────────────────────

    def _send_stdio(self, request: dict) -> dict:
        """Send a JSON-RPC request over stdio and return the response."""
        msg = json.dumps(request)
        self._stdin.write(msg + "\n")
        self._stdin.flush()
        line = self._stdout.readline()
        if not line:
            raise LtpError(-32010, "Server closed connection")
        return json.loads(line)

    def _send_http(self, request: dict) -> dict:
        """Send a JSON-RPC request over HTTP and return the response."""
        data = json.dumps(request).encode("utf-8")
        headers = {"Content-Type": "application/json"}
        if self._token:
            headers["Authorization"] = f"Bearer {self._token}"
        if self._session_id:
            headers["Ltp-Session-Id"] = self._session_id

        req = urllib.request.Request(
            self._endpoint, data=data, headers=headers, method="POST"
        )
        try:
            with urllib.request.urlopen(req, timeout=300) as resp:
                return json.loads(resp.read().decode("utf-8"))
        except urllib.error.HTTPError as e:
            body = e.read().decode("utf-8", errors="replace")
            raise LtpError(-32010, f"HTTP {e.code}: {body}")
        except urllib.error.URLError as e:
            raise LtpError(-32010, f"Connection error: {e.reason}")

    def _call(self, method: str, params: dict = None) -> dict:
        """Call an LTP method and return the result."""
        request = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params or {},
            "id": self._next_id,
        }
        self._next_id += 1

        if self._transport == "stdio":
            response = self._send_stdio(request)
        elif self._transport == "http":
            response = self._send_http(request)
        else:
            raise LtpError(-32603, f"Unknown transport: {self._transport}")

        if "error" in response:
            err = response["error"]
            raise LtpError(err.get("code", -32603), err.get("message", "Unknown"), err.get("data"))

        return response.get("result", {})

    # ── Session methods ────────────────────────────────────────

    def initialize(self, client_name: str = "LtpClient", client_version: str = "0.1.0") -> dict:
        """Initialize an LTP session."""
        result = self._call("session/initialize", {
            "protocol_version": "0.1",
            "client_info": {
                "name": client_name,
                "version": client_version,
            },
            "capabilities": {
                "streaming": True,
                "batch": True,
                "human_approval": True,
            },
        })
        self._session_id = result.get("session_id")
        return result

    def close(self, reason: str = "completed"):
        """Close the LTP session."""
        try:
            self._call("session/close", {"reason": reason})
        except LtpError:
            pass
        if self._transport == "stdio" and hasattr(self, "_process"):
            self._process.terminate()
            self._process.wait(timeout=5)

    # ── IR methods ─────────────────────────────────────────────

    def load_ir(self, ir: dict, validate: bool = True) -> dict:
        """Load a Lucky IR program (as a JSON dict)."""
        result = self._call("ir/load", {
            "ir": ir,
            "options": {"validate": validate},
        })
        self._ir_loaded = True
        return result

    def load_ir_file(self, path: str) -> dict:
        """Load a Lucky IR program from a .lir JSON file."""
        with open(path, "r") as f:
            ir = json.load(f)
        return self.load_ir(ir)

    def load_ir_from_source(self, source: str) -> dict:
        """Compile Lucky source and load the resulting IR.
        This requires the Lucky compiler to be available as `lucky ir`.
        """
        proc = subprocess.run(
            ["lucky", "ir", "--opt", "O2", "-"],
            input=source,
            capture_output=True,
            text=True,
            timeout=30,
        )
        if proc.returncode != 0:
            raise LtpError(-32004, f"Compilation failed: {proc.stderr}")
        ir = json.loads(proc.stdout)
        return self.load_ir(ir.get("hir", ir))

    # ── Execution methods ──────────────────────────────────────

    def execute(
        self,
        goal: str = None,
        workflow: str = None,
        task: str = None,
        context: dict = None,
        mode: str = "sync",
        on_event: Callable = None,
    ) -> dict:
        """Execute a loaded Lucky program."""
        params = {
            "context": context or {},
            "mode": mode,
        }
        if goal:
            params["entry_point"] = goal
            params["entry_kind"] = "goal"
        elif workflow:
            params["entry_point"] = workflow
            params["entry_kind"] = "workflow"
        elif task:
            params["entry_point"] = task
            params["entry_kind"] = "task"

        return self._call("execution/start", params)

    def get_status(self, execution_id: str) -> dict:
        """Get the status of a running execution."""
        return self._call("execution/get_status", {"execution_id": execution_id})

    def cancel(self, execution_id: str, reason: str = "User cancelled") -> dict:
        """Cancel a running execution."""
        return self._call("execution/cancel", {
            "execution_id": execution_id,
            "reason": reason,
        })

    def pause(self, execution_id: str) -> dict:
        """Pause a running execution."""
        return self._call("execution/pause", {"execution_id": execution_id})

    def resume(self, execution_id: str) -> dict:
        """Resume a paused execution."""
        return self._call("execution/resume", {"execution_id": execution_id})

    # ── Approval methods ───────────────────────────────────────

    def respond_approval(self, approval_id: str, decision: str, reason: str = "") -> dict:
        """Respond to a human approval request."""
        return self._call("approval/respond", {
            "approval_id": approval_id,
            "decision": decision,
            "reason": reason,
        })

    def list_approvals(self) -> list:
        """List pending approval requests."""
        return self._call("approval/list", {}).get("pending", [])

    # ── Checkpoint methods ─────────────────────────────────────

    def create_checkpoint(self, execution_id: str, label: str = "") -> dict:
        """Create a checkpoint of the current execution state."""
        return self._call("checkpoint/create", {
            "execution_id": execution_id,
            "label": label,
        })

    def restore_checkpoint(self, execution_id: str, checkpoint_id: str) -> dict:
        """Restore execution from a checkpoint."""
        return self._call("checkpoint/restore", {
            "execution_id": execution_id,
            "checkpoint_id": checkpoint_id,
        })

    # ── Query methods ──────────────────────────────────────────

    def query_cost(self, execution_id: str = None) -> dict:
        """Query cost information."""
        params = {}
        if execution_id:
            params["execution_id"] = execution_id
        return self._call("query/cost", params)

    def query_context(self, execution_id: str, node_id: str = None) -> dict:
        """Query execution context."""
        params = {"execution_id": execution_id}
        if node_id:
            params["node_id"] = node_id
        return self._call("query/context", params)

    def query_tools(self) -> list:
        """List available tools on the server."""
        return self._call("query/tools", {}).get("tools", [])

    def query_models(self) -> list:
        """List available models on the server."""
        return self._call("query/models", {}).get("models", [])

    # ── Tool methods ───────────────────────────────────────────

    def invoke_tool(self, tool_id: str, method: str, arguments: dict) -> dict:
        """Invoke a tool directly."""
        return self._call("tool/invoke", {
            "tool_id": tool_id,
            "method": method,
            "arguments": arguments,
        })


# ── High-level convenience functions ────────────────────────────

def run_lucky_program(
    source: str,
    goal: str = None,
    context: dict = None,
    server_command: list[str] = None,
    server_url: str = None,
) -> dict:
    """Convenience function: compile and execute a Lucky program in one call.

    Args:
        source: Lucky source code string.
        goal: Goal name to pursue (or None for default).
        context: Execution context dict.
        server_command: Command to spawn an LTP server (e.g., ["lucky", "serve"]).
        server_url: URL of a running LTP server (e.g., "http://localhost:9700").

    Returns:
        Execution result dict with keys: result, cost, outputs, duration_ms.
    """
    if server_command:
        client = LtpClient.stdio(server_command)
    elif server_url:
        client = LtpClient.http(server_url)
    else:
        client = LtpClient.stdio(["lucky", "serve", "--transport", "stdio"])

    try:
        client.initialize()
        client.load_ir_from_source(source)
        result = client.execute(goal=goal, context=context, mode="sync")
        return result
    finally:
        client.close()


# ── CLI entry point ─────────────────────────────────────────────

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Lucky LTP Client")
    parser.add_argument("--server", default="http://localhost:9700", help="LTP server URL")
    parser.add_argument("--token", help="Auth token")
    parser.add_argument("action", choices=["run", "status", "cancel", "tools", "models", "approvals"])
    parser.add_argument("--file", "-f", help="Lucky source or IR file")
    parser.add_argument("--goal", "-g", help="Goal to pursue")
    parser.add_argument("--exec-id", help="Execution ID")
    parser.add_argument("--source", "-s", help="Inline Lucky source code")

    args = parser.parse_args()

    if args.source:
        source = args.source
    elif args.file:
        with open(args.file, "r") as f:
            source = f.read()
    else:
        source = None

    if args.server.startswith("ltp+stdio://"):
        cmd = args.server.replace("ltp+stdio://", "").split()
        client = LtpClient.stdio(cmd)
    else:
        client = LtpClient.http(args.server, token=args.token)

    try:
        client.initialize()

        if args.action == "run":
            if not source:
                parser.error("--file or --source required for 'run'")
            if args.file and args.file.endswith(".lir"):
                client.load_ir_file(args.file)
            else:
                client.load_ir_from_source(source)
            result = client.execute(goal=args.goal, mode="sync")
            print(json.dumps(result, indent=2))

        elif args.action == "status":
            if not args.exec_id:
                parser.error("--exec-id required for 'status'")
            status = client.get_status(args.exec_id)
            print(json.dumps(status, indent=2))

        elif args.action == "cancel":
            if not args.exec_id:
                parser.error("--exec-id required for 'cancel'")
            client.cancel(args.exec_id)
            print("Cancelled")

        elif args.action == "tools":
            tools = client.query_tools()
            print(json.dumps(tools, indent=2))

        elif args.action == "models":
            models = client.query_models()
            print(json.dumps(models, indent=2))

        elif args.action == "approvals":
            approvals = client.list_approvals()
            print(json.dumps(approvals, indent=2))

    except LtpError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    finally:
        client.close()
