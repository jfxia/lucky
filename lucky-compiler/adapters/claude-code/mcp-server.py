#!/usr/bin/env python3
"""
Lucky MCP Server — bridges Claude Code's Model Context Protocol to the Lucky LTP runtime.

Reads JSON-RPC 2.0 from stdin, writes JSON-RPC 2.0 to stdout.
Handles MCP `tools/list` and `tools/call` by delegating to the Lucky LTP client.

Environment variables:
  LUCKY_LTP_SERVER   URL of the LTP server (http://host:port) or stdio command prefix
                     Default: http://localhost:9700
  LUCKY_LTP_TOKEN    Bearer token for HTTP LTP servers (optional)
"""

import json
import os
import sys
import traceback

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from ltp_client import LtpClient, LtpError, run_lucky_program

MCP_TOOLS = [
    {
        "name": "lucky_run",
        "description": "Execute a Lucky program by compiling and running it through the Lucky LTP runtime. Provide Lucky source code and a goal to pursue.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Lucky source code to execute",
                },
                "goal": {
                    "type": "string",
                    "description": "The goal name to pursue (entry point in the Lucky program)",
                },
                "context": {
                    "type": "object",
                    "description": "Execution context dictionary with project-specific variables",
                },
                "mode": {
                    "type": "string",
                    "enum": ["sync", "async"],
                    "description": "Execution mode",
                    "default": "sync",
                },
            },
            "required": ["source"],
        },
    },
    {
        "name": "lucky_status",
        "description": "Check the status of a running Lucky program execution.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "execution_id": {
                    "type": "string",
                    "description": "The execution ID returned by a previous lucky_run call in async mode",
                }
            },
            "required": ["execution_id"],
        },
    },
    {
        "name": "lucky_approve",
        "description": "Respond to a human approval request from a running Lucky program.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "approval_id": {
                    "type": "string",
                    "description": "The approval request ID to respond to",
                },
                "decision": {
                    "type": "string",
                    "enum": ["approve", "reject", "modify"],
                    "description": "Your decision",
                },
                "reason": {
                    "type": "string",
                    "description": "Explanation for the decision",
                },
            },
            "required": ["approval_id", "decision"],
        },
    },
]

SERVER_INFO = {
    "name": "lucky-ltp-mcp",
    "version": "0.1.0",
}


def log_stderr(message: str):
    print(f"[lucky-mcp] {message}", file=sys.stderr, flush=True)


def _get_ltp_client():
    server = os.environ.get("LUCKY_LTP_SERVER", "http://localhost:9700")
    token = os.environ.get("LUCKY_LTP_TOKEN") or None

    if server.startswith("ltp+stdio://"):
        cmd = server.replace("ltp+stdio://", "").split()
        return LtpClient.stdio(cmd)
    else:
        return LtpClient.http(server, token=token)


class LuckyMcpServer:
    """MCP server that wraps the Lucky LTP client."""

    def __init__(self):
        self._client = None
        self._initialized = False

    def _ensure_client(self):
        if self._client is None:
            self._client = _get_ltp_client()
            self._client.initialize(client_name="claude-code-mcp", client_version="0.1.0")
            log_stderr("LTP session initialized")
        return self._client

    def handle_initialize(self, _params, _id):
        return {
            "protocolVersion": "0.1.0",
            "capabilities": {
                "tools": {},
            },
            "serverInfo": SERVER_INFO,
        }

    def handle_tools_list(self, _params, _id):
        return {"tools": MCP_TOOLS}

    def handle_tools_call(self, params, _id):
        name = params.get("name", "")
        arguments = params.get("arguments", {})

        if name == "lucky_run":
            return self._tool_lucky_run(arguments)
        elif name == "lucky_status":
            return self._tool_lucky_status(arguments)
        elif name == "lucky_approve":
            return self._tool_lucky_approve(arguments)
        else:
            return {"content": [{"type": "text", "text": f"Unknown tool: {name}"}], "isError": True}

    def _tool_lucky_run(self, args):
        source = args.get("source", "")
        goal = args.get("goal")
        context = args.get("context") or {}
        mode = args.get("mode", "sync")

        if not source.strip():
            return {"content": [{"type": "text", "text": "Error: 'source' parameter is required and must not be empty."}], "isError": True}

        try:
            client = self._ensure_client()
            client.load_ir_from_source(source)
            result = client.execute(goal=goal, context=context, mode=mode)
            return {"content": [{"type": "text", "text": json.dumps(result, indent=2)}]}
        except LtpError as e:
            return {"content": [{"type": "text", "text": f"LTP Error {e.code}: {e.message}"}], "isError": True}
        except FileNotFoundError:
            return {"content": [{"type": "text", "text": "Error: The 'lucky' CLI is not found on PATH. Install the Lucky compiler to compile source programs."}], "isError": True}
        except Exception as e:
            log_stderr(traceback.format_exc())
            return {"content": [{"type": "text", "text": f"Error: {e}"}], "isError": True}

    def _tool_lucky_status(self, args):
        execution_id = args.get("execution_id", "")

        try:
            client = self._ensure_client()
            status = client.get_status(execution_id)
            return {"content": [{"type": "text", "text": json.dumps(status, indent=2)}]}
        except LtpError as e:
            return {"content": [{"type": "text", "text": f"LTP Error {e.code}: {e.message}"}], "isError": True}
        except Exception as e:
            log_stderr(traceback.format_exc())
            return {"content": [{"type": "text", "text": f"Error: {e}"}], "isError": True}

    def _tool_lucky_approve(self, args):
        approval_id = args.get("approval_id", "")
        decision = args.get("decision", "approve")
        reason = args.get("reason", "")

        if decision not in ("approve", "reject", "modify"):
            return {"content": [{"type": "text", "text": f"Error: 'decision' must be one of: approve, reject, modify. Got: {decision}"}], "isError": True}

        try:
            client = self._ensure_client()
            result = client.respond_approval(approval_id, decision, reason)
            return {"content": [{"type": "text", "text": json.dumps(result, indent=2)}]}
        except LtpError as e:
            return {"content": [{"type": "text", "text": f"LTP Error {e.code}: {e.message}"}], "isError": True}
        except Exception as e:
            log_stderr(traceback.format_exc())
            return {"content": [{"type": "text", "text": f"Error: {e}"}], "isError": True}

    def _send(self, message: dict):
        line = json.dumps(message, ensure_ascii=False)
        sys.stdout.write(line + "\n")
        sys.stdout.flush()
        log_stderr(f"-> {line[:200]}")

    def run(self):
        log_stderr("Lucky MCP server starting (stdio transport)")
        for raw_line in sys.stdin:
            line = raw_line.strip()
            if not line:
                continue

            try:
                request = json.loads(line)
            except json.JSONDecodeError as e:
                log_stderr(f"Invalid JSON received: {e}")
                continue

            log_stderr(f"<- {line[:200]}")

            req_id = request.get("id")
            method = request.get("method", "")

            if method == "initialize":
                result = self.handle_initialize(request.get("params", {}), req_id)
                self._initialized = True
                self._send({"jsonrpc": "2.0", "result": result, "id": req_id})

            elif method == "notifications/initialized":
                pass

            elif method == "tools/list":
                if not self._initialized:
                    self._send({"jsonrpc": "2.0", "error": {"code": -32003, "message": "Not initialized"}, "id": req_id})
                    continue
                result = self.handle_tools_list(request.get("params", {}), req_id)
                self._send({"jsonrpc": "2.0", "result": result, "id": req_id})

            elif method == "tools/call":
                if not self._initialized:
                    self._send({"jsonrpc": "2.0", "error": {"code": -32003, "message": "Not initialized"}, "id": req_id})
                    continue
                result = self.handle_tools_call(request.get("params", {}), req_id)
                self._send({"jsonrpc": "2.0", "result": result, "id": req_id})

            elif method == "shutdown":
                if self._client:
                    try:
                        self._client.close(reason="mcp_shutdown")
                    except Exception:
                        pass
                self._send({"jsonrpc": "2.0", "result": {}, "id": req_id})
                break

            elif method == "exit":
                break

            else:
                log_stderr(f"Unknown method: {method}")
                self._send({"jsonrpc": "2.0", "error": {"code": -32601, "message": f"Method not found: {method}"}, "id": req_id})


if __name__ == "__main__":
    server = LuckyMcpServer()
    server.run()
