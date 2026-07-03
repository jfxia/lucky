#!/usr/bin/env python3
"""
OpenCode Lucky Executor — run.py

Implements the lucky_run, lucky_status, and lucky_approve tool functions
that OpenCode calls when the lucky-executor skill is activated.

Uses the LTP client (adapters/ltp_client.py) to communicate with a
Lucky runtime via stdio or HTTP.
"""

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from ltp_client import LtpClient, LtpError


def _get_client():
    server_url = os.environ.get("LUCKY_SERVER_URL")
    token = os.environ.get("LUCKY_SERVER_TOKEN")

    if server_url:
        return LtpClient.http(server_url, token=token)
    else:
        cmd = os.environ.get("LUCKY_SERVER_COMMAND", "lucky serve --transport stdio").split()
        return LtpClient.stdio(cmd)


def lucky_run(ir_path, goal=None, context=None):
    """
    Run a Lucky program from a .lir IR file.

    Args:
        ir_path (str): Path to the .lir JSON IR file.
        goal (str, optional): Goal name to pursue.
        context (dict, optional): Execution context.

    Returns:
        dict with keys: execution_id, status, result, outputs, cost, duration_ms.
    """
    client = _get_client()

    try:
        init_result = client.initialize(
            client_name="OpenCode-lucky-executor",
            client_version="0.1.0",
        )

        with open(ir_path, "r") as f:
            ir = json.load(f)

        client.load_ir(ir)

        result = client.execute(
            goal=goal,
            context=context or {},
            mode="sync",
        )

        return result
    except LtpError as e:
        return {
            "execution_id": None,
            "status": "error",
            "result": "failure",
            "error": {"code": e.code, "message": e.message, "data": e.data},
        }
    except FileNotFoundError:
        return {
            "execution_id": None,
            "status": "error",
            "result": "failure",
            "error": {"code": -32603, "message": f"IR file not found: {ir_path}"},
        }
    except Exception as e:
        return {
            "execution_id": None,
            "status": "error",
            "result": "failure",
            "error": {"code": -32603, "message": str(e)},
        }
    finally:
        client.close()


def lucky_status(exec_id):
    """
    Check the status of a Lucky execution.

    Args:
        exec_id (str): Execution ID returned by lucky_run.

    Returns:
        dict with keys: execution_id, status, progress, current_node,
                        elapsed_ms, estimated_remaining_ms, node_states, cost.
    """
    client = _get_client()

    try:
        client.initialize(
            client_name="OpenCode-lucky-executor",
            client_version="0.1.0",
        )
        return client.get_status(exec_id)
    except LtpError as e:
        return {
            "execution_id": exec_id,
            "status": "error",
            "error": {"code": e.code, "message": e.message, "data": e.data},
        }
    except Exception as e:
        return {
            "execution_id": exec_id,
            "status": "error",
            "error": {"code": -32603, "message": str(e)},
        }
    finally:
        client.close()


def lucky_approve(approval_id, decision, reason=""):
    """
    Respond to a Lucky human-approval request.

    Args:
        approval_id (str): Approval request ID.
        decision (str): approve, reject, or modify.
        reason (str, optional): Reason for the decision.

    Returns:
        dict with keys: acknowledged (bool), approval_id (str).
    """
    client = _get_client()

    try:
        client.initialize(
            client_name="OpenCode-lucky-executor",
            client_version="0.1.0",
        )
        result = client.respond_approval(approval_id, decision, reason)
        return result
    except LtpError as e:
        return {
            "approval_id": approval_id,
            "acknowledged": False,
            "error": {"code": e.code, "message": e.message, "data": e.data},
        }
    except Exception as e:
        return {
            "approval_id": approval_id,
            "acknowledged": False,
            "error": {"code": -32603, "message": str(e)},
        }
    finally:
        client.close()


# ── CLI entry point (for standalone testing) ─────────────────────

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="OpenCode Lucky Executor — run Lucky programs via LTP"
    )
    sub = parser.add_subparsers(dest="command", required=True)

    run_parser = sub.add_parser("run", help="Run a Lucky program")
    run_parser.add_argument("ir_path", help="Path to .lir IR file")
    run_parser.add_argument("--goal", "-g", help="Goal to pursue")
    run_parser.add_argument("--context", "-c", help="Context as JSON string")

    status_parser = sub.add_parser("status", help="Check execution status")
    status_parser.add_argument("exec_id", help="Execution ID")

    approve_parser = sub.add_parser("approve", help="Respond to approval")
    approve_parser.add_argument("approval_id", help="Approval ID")
    approve_parser.add_argument("decision", choices=["approve", "reject", "modify"])
    approve_parser.add_argument("--reason", "-r", default="", help="Reason")

    args = parser.parse_args()

    if args.command == "run":
        context = json.loads(args.context) if args.context else None
        result = lucky_run(args.ir_path, goal=args.goal, context=context)
        print(json.dumps(result, indent=2))

    elif args.command == "status":
        result = lucky_status(args.exec_id)
        print(json.dumps(result, indent=2))

    elif args.command == "approve":
        result = lucky_approve(args.approval_id, args.decision, args.reason)
        print(json.dumps(result, indent=2))
