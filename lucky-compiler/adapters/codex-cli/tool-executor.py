#!/usr/bin/env python3
"""
Codex CLI tool executor for Lucky.
Reads tool name and arguments from command line or stdin JSON,
delegates to the LTP client, prints JSON result to stdout.

Usage:
    python tool-executor.py lucky_execute --program path/to/prog.lk --goal MyGoal
    python tool-executor.py lucky_status --execution_id abc123
    python tool-executor.py lucky_approve --approval_id xyz --decision approved
    echo '{"tool":"lucky_execute","args":{"program":"prog.lk"}}' | python tool-executor.py
"""

import json
import os
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ADAPTERS_DIR = os.path.dirname(SCRIPT_DIR)
sys.path.insert(0, ADAPTERS_DIR)

from ltp_client import LtpClient, LtpError


def get_client():
    server_url = os.environ.get("LUCKY_SERVER_URL", "http://localhost:9700")
    token = os.environ.get("LUCKY_SERVER_TOKEN")

    if server_url.startswith("ltp+stdio://"):
        cmd = server_url.replace("ltp+stdio://", "").split()
        return LtpClient.stdio(cmd)
    else:
        return LtpClient.http(server_url, token=token)


def handle_lucky_execute(args, client):
    program = args.get("program", "")
    goal = args.get("goal")
    context = args.get("context") or {}
    mode = args.get("mode", "sync").lower()

    if mode not in ("sync", "async"):
        return {"error": f"Invalid mode: {mode}. Must be 'sync' or 'async'."}

    client.load_ir_from_source(_read_file(program))
    if mode == "async":
        result = client.execute(goal=goal, context=context, mode="async")
    else:
        result = client.execute(goal=goal, context=context, mode="sync")
    return result


def handle_lucky_status(args, client):
    execution_id = args.get("execution_id", "")
    if not execution_id:
        return {"error": "execution_id is required"}
    return client.get_status(execution_id)


def handle_lucky_approve(args, client):
    approval_id = args.get("approval_id", "")
    decision = args.get("decision", "")
    reason = args.get("reason", "")

    if not approval_id:
        return {"error": "approval_id is required"}
    if decision not in ("approved", "rejected"):
        return {"error": "decision must be 'approved' or 'rejected'"}

    return client.respond_approval(approval_id, decision, reason)


def _read_file(path):
    if not path:
        raise ValueError("program path is empty")
    with open(path, "r", encoding="utf-8") as f:
        return f.read()


TOOL_HANDLERS = {
    "lucky_execute": handle_lucky_execute,
    "lucky_status": handle_lucky_status,
    "lucky_approve": handle_lucky_approve,
}


def parse_args_from_cmdline(argv):
    if len(argv) < 1:
        raise ValueError("No tool name provided")

    tool_name = argv[0]
    args = {}
    i = 1
    while i < len(argv):
        if argv[i].startswith("--"):
            key = argv[i][2:]
            if i + 1 < len(argv) and not argv[i + 1].startswith("--"):
                args[key] = argv[i + 1]
                i += 2
            else:
                args[key] = True
                i += 1
        else:
            i += 1

    for k, v in args.items():
        if v == "true":
            args[k] = True
        elif v == "false":
            args[k] = False
        elif isinstance(v, str) and (v.startswith("{") or v.startswith("[")):
            try:
                args[k] = json.loads(v)
            except json.JSONDecodeError:
                pass

    return tool_name, args


def main():
    tool_name = None
    args = {}

    if len(sys.argv) > 1:
        tool_name, args = parse_args_from_cmdline(sys.argv[1:])
    else:
        raw = sys.stdin.read()
        if raw.strip():
            payload = json.loads(raw)
            tool_name = payload.get("tool", payload.get("name", ""))
            args = payload.get("args", payload.get("arguments", payload.get("parameters", {})))

    if not tool_name:
        print(json.dumps({"error": "No tool specified"}))
        sys.exit(1)

    handler = TOOL_HANDLERS.get(tool_name)
    if not handler:
        print(json.dumps({"error": f"Unknown tool: {tool_name}"}))
        sys.exit(1)

    client = None
    try:
        client = get_client()
        client.initialize(client_name="codex-cli-adapter", client_version="0.1.0")
        result = handler(args, client)
        print(json.dumps(result))
    except FileNotFoundError as e:
        print(json.dumps({"error": f"File not found: {e}"}))
        sys.exit(1)
    except LtpError as e:
        print(json.dumps({"error": f"LTP error: {e}", "code": e.code}))
        sys.exit(1)
    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)
    finally:
        if client:
            try:
                client.close()
            except Exception:
                pass


if __name__ == "__main__":
    main()
