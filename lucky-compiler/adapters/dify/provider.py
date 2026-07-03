#!/usr/bin/env python3
"""
Dify Tool Provider for Lucky Program Executor.

Integrates Lucky as a custom Tool in Dify workflows by implementing the
Dify tool provider interface. Communicates with a Lucky LTP server to
compile (when needed), load, and execute Lucky IR programs.

Requirements:
    pip install requests      # Only needed for HTTP transport
"""

import json
import os
import sys
import time
import uuid
from typing import Any, Optional

_HERE = os.path.dirname(os.path.abspath(__file__))
_PARENT = os.path.dirname(_HERE)
if _PARENT not in sys.path:
    sys.path.insert(0, _PARENT)

from ltp_client import LtpClient, LtpError


class LuckyExecutorTool:
    """
    Dify tool provider that wraps a Lucky LTP client.

    Credentials:
        ltp_endpoint   — str: URL of the LTP server (e.g. "http://localhost:9700")
        ltp_token      — str: optional bearer token for authentication
        ltp_transport  — str: "http" (default) or "stdio"
        ltp_command    — str: server command, only for stdio transport
                           (e.g. "lucky serve --transport stdio")
    """

    # ── Dify interface ──────────────────────────────────────────

    def validate_credentials(self, credentials: dict) -> None:
        """
        Validate the supplied credentials by establishing a test
        connection to the LTP server and querying server info.

        Raises:
            LtpError: if the connection or handshake fails.
            ValueError: if required credential keys are missing.
        """
        client = self._make_client(credentials)
        try:
            client.initialize(
                client_name="Dify-LuckyExecutor",
                client_version="0.1.0",
            )
            tools = client.query_tools()
            if not tools:
                raise LtpError(-32010, "Server returned no tools — may not be ready")
        finally:
            client.close(reason="credential_validation")

    def invoke(
        self,
        model: str,
        tool_params: dict,
        credentials: dict,
    ) -> dict:
        """
        Execute a Lucky program.

        Args:
            model: The LLM model name — unused by this tool but required
                   by the Dify interface.
            tool_params: Dict with keys:
                ir (str, required):   Lucky IR program as a JSON string.
                goal (str, required): Goal name to execute.
                context (dict):       Optional execution context.
                mode (str):           "sync" (default) or "async".

        Returns:
            Dict with keys: result, cost_usd, outputs, duration_ms,
            and optionally execution_id (for async).

        Raises:
            LtpError: on protocol errors.
            ValueError: on invalid parameters.
        """
        ir = tool_params.get("ir")
        if not ir:
            raise ValueError("Missing required parameter: ir")

        goal = tool_params.get("goal")
        if not goal:
            raise ValueError("Missing required parameter: goal")

        context = tool_params.get("context") or {}
        if isinstance(context, str):
            context = json.loads(context)
        mode = tool_params.get("mode", "sync")

        client = self._make_client(credentials)
        try:
            client.initialize(
                client_name="Dify-LuckyExecutor",
                client_version="0.1.0",
            )

            # Load IR — parse JSON string to dict if needed
            try:
                ir_obj = json.loads(ir) if isinstance(ir, str) else ir
            except json.JSONDecodeError as e:
                raise ValueError(f"IR must be valid JSON: {e}")

            client.load_ir(ir_obj)

            t0 = time.time()
            result = client.execute(
                goal=goal,
                context=context,
                mode=mode,
            )
            duration_ms = int((time.time() - t0) * 1000)

            return self._format_result(result, duration_ms)
        finally:
            client.close(reason="workflow_completed")

    # ── Helpers ─────────────────────────────────────────────────

    @staticmethod
    def _make_client(credentials: dict) -> LtpClient:
        transport = credentials.get("ltp_transport", "http")

        if transport == "stdio":
            command = credentials.get("ltp_command")
            if not command:
                command = "lucky serve --transport stdio"
            return LtpClient.stdio(command.split())

        endpoint = credentials.get("ltp_endpoint")
        if not endpoint:
            endpoint = "http://localhost:9700"
        token = credentials.get("ltp_token")
        return LtpClient.http(endpoint, token=token)

    @staticmethod
    def _format_result(raw: dict, duration_ms: int) -> dict:
        """
        Normalize the raw LTP execution response into the Dify output schema.
        """
        cost = raw.get("cost", {})
        total_usd = cost.get("total_usd", 0.0) if isinstance(cost, dict) else 0.0

        outputs = raw.get("outputs", {})
        result_val = raw.get("result", "unknown")

        formatted = {
            "result": json.dumps(result_val) if not isinstance(result_val, str) else result_val,
            "cost_usd": total_usd,
            "outputs": outputs,
            "duration_ms": raw.get("duration_ms", duration_ms),
        }

        exec_id = raw.get("execution_id")
        if exec_id:
            formatted["execution_id"] = exec_id

        return formatted


# ── Standalone entry point for testing ──────────────────────────

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Test LuckyExecutorTool")
    parser.add_argument("--endpoint", default="http://localhost:9700")
    parser.add_argument("--token")
    parser.add_argument("--ir-file", required=True, help="Path to .lir JSON file")
    parser.add_argument("--goal", "-g", required=True)
    parser.add_argument("--context", "-c", default="{}", help="JSON context string")
    parser.add_argument("--mode", default="sync", choices=["sync", "async"])

    args = parser.parse_args()

    with open(args.ir_file, "r") as f:
        ir_text = f.read()

    tool = LuckyExecutorTool()
    credentials = {
        "ltp_endpoint": args.endpoint,
        "ltp_token": args.token,
    }
    params = {
        "ir": ir_text,
        "goal": args.goal,
        "context": json.loads(args.context),
        "mode": args.mode,
    }

    try:
        tool.validate_credentials(credentials)
        print("Credentials valid.")
    except Exception as e:
        print(f"Credential validation failed: {e}")
        sys.exit(1)

    result = tool.invoke(model="default", tool_params=params, credentials=credentials)
    print(json.dumps(result, indent=2))
