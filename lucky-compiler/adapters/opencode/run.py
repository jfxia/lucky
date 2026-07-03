#!/usr/bin/env python3
"""
OpenCode Lucky Executor — run.py

Implements the lucky_run, lucky_status, and lucky_approve tool functions
that OpenCode calls when the lucky-executor skill is activated.

Uses the Lucky CLI binary directly for reliable operation.
"""
import json
import os
import subprocess
import sys
import tempfile
import uuid

# Path to the Lucky CLI binary — customize if needed
LUCKY_BIN = os.environ.get("LUCKY_BIN", "lucky")


def _run_lucky(args, input_text=None):
    """Run the Lucky CLI binary and return stdout, stderr, exit code."""
    cmd = [LUCKY_BIN] + args
    result = subprocess.run(
        cmd,
        input=input_text,
        capture_output=True,
        text=True,
        timeout=120,
    )
    return result.stdout, result.stderr, result.returncode


def lucky_run(source=None, ir_path=None, goal=None, context=None):
    """
    Run a Lucky program from source code or a .lk file.

    Args:
        source (str, optional): Lucky source code string.
        ir_path (str, optional): Path to a .lk source file.
        goal (str, optional): Goal name to pursue.
        context (dict, optional): Execution context.

    Returns:
        dict with keys: execution_id, status, result, output, duration_ms.
    """
    exec_id = str(uuid.uuid4())[:8]

    try:
        if ir_path:
            args = ["run", ir_path]
        elif source:
            # Write source to temp file
            tmp = tempfile.NamedTemporaryFile(
                mode="w", suffix=".lk", delete=False, encoding="utf-8"
            )
            tmp.write(source)
            tmp.close()
            args = ["run", tmp.name]
        else:
            return {"execution_id": exec_id, "status": "error",
                    "error": "Either source or ir_path is required"}

        stdout, stderr, code = _run_lucky(args)

        if code == 0:
            return {
                "execution_id": exec_id,
                "status": "completed",
                "result": "success",
                "output": stdout.strip(),
                "stderr": stderr.strip(),
                "duration_ms": 0,
            }
        else:
            return {
                "execution_id": exec_id,
                "status": "failed",
                "result": "failure",
                "error": stderr.strip() or "Exit code {}".format(code),
            }
    except subprocess.TimeoutExpired:
        return {"execution_id": exec_id, "status": "error",
                "error": "Execution timed out"}
    except FileNotFoundError:
        return {"execution_id": exec_id, "status": "error",
                "error": "Lucky binary not found: {}".format(LUCKY_BIN)}
    except Exception as e:
        return {"execution_id": exec_id, "status": "error",
                "error": str(e)}


def lucky_check(source=None, file_path=None):
    """
    Check a Lucky program for syntax errors.

    Args:
        source (str, optional): Lucky source code.
        file_path (str, optional): Path to a .lk file.

    Returns:
        dict with keys: valid, errors, warnings.
    """
    try:
        if file_path:
            args = ["check", file_path]
        elif source:
            tmp = tempfile.NamedTemporaryFile(
                mode="w", suffix=".lk", delete=False, encoding="utf-8"
            )
            tmp.write(source)
            tmp.close()
            args = ["check", tmp.name]
        else:
            return {"valid": False, "errors": ["No source provided"]}

        stdout, stderr, code = _run_lucky(args)

        errors = []
        if code != 0:
            for line in stderr.split("\n"):
                if "error" in line.lower():
                    errors.append(line.strip())

        return {
            "valid": code == 0,
            "errors": errors,
            "warnings": [],
        }
    except Exception as e:
        return {"valid": False, "errors": [str(e)], "warnings": []}


def lucky_ir(source=None, file_path=None):
    """
    Compile a Lucky program to IR JSON.

    Args:
        source (str, optional): Lucky source code.
        file_path (str, optional): Path to a .lk file.

    Returns:
        dict with keys: hir_json, mir_json, error.
    """
    try:
        if file_path:
            args = ["ir", file_path]
        elif source:
            tmp = tempfile.NamedTemporaryFile(
                mode="w", suffix=".lk", delete=False, encoding="utf-8"
            )
            tmp.write(source)
            tmp.close()
            args = ["ir", tmp.name]
        else:
            return {"error": "No source provided"}

        stdout, stderr, code = _run_lucky(args)

        if code == 0:
            try:
                ir = json.loads(stdout)
                return {"hir_json": ir.get("hir"), "mir_json": ir.get("mir")}
            except json.JSONDecodeError:
                return {"hir_json": stdout, "mir_json": None}
        else:
            return {"error": stderr.strip()}
    except Exception as e:
        return {"error": str(e)}


def lucky_format(source=None, file_path=None):
    """
    Format a Lucky source file.

    Returns:
        dict with keys: formatted, error.
    """
    try:
        if file_path:
            args = ["fmt", file_path]
            stdout, stderr, code = _run_lucky(args)
            return {"formatted": code == 0, "error": stderr.strip() if code != 0 else None}
        elif source:
            tmp = tempfile.NamedTemporaryFile(
                mode="w", suffix=".lk", delete=False, encoding="utf-8"
            )
            tmp.write(source)
            tmp.close()
            args = ["fmt", tmp.name]
            stdout, stderr, code = _run_lucky(args)
            if code == 0:
                with open(tmp.name, "r") as f:
                    formatted = f.read()
                return {"formatted": True, "source": formatted}
            return {"formatted": False, "error": stderr.strip()}
        else:
            return {"error": "No source provided"}
    except Exception as e:
        return {"error": str(e)}


def lucky_init(path):
    """Initialize a new Lucky project."""
    try:
        stdout, stderr, code = _run_lucky(["init", path])
        return {"success": code == 0, "path": path, "output": stdout.strip()}
    except Exception as e:
        return {"success": False, "error": str(e)}


# ── CLI entry point (for standalone testing) ─────────────────────

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="OpenCode Lucky Executor — run Lucky programs"
    )
    sub = parser.add_subparsers(dest="command", required=True)

    run_parser = sub.add_parser("run", help="Run a Lucky program")
    run_parser.add_argument("file", help="Path to .lk file or inline source", nargs="?")
    run_parser.add_argument("--source", "-s", help="Inline Lucky source code")
    run_parser.add_argument("--goal", "-g", help="Goal to pursue")

    check_parser = sub.add_parser("check", help="Check syntax")
    check_parser.add_argument("file", help="Path to .lk file", nargs="?")

    ir_parser = sub.add_parser("ir", help="Compile to IR")
    ir_parser.add_argument("file", help="Path to .lk file", nargs="?")

    fmt_parser = sub.add_parser("fmt", help="Format source")
    fmt_parser.add_argument("file", help="Path to .lk file", nargs="?")

    init_parser = sub.add_parser("init", help="Initialize project")
    init_parser.add_argument("path", help="Project path")

    args = parser.parse_args()

    if args.command == "run":
        result = lucky_run(
            source=args.source,
            ir_path=args.file if not args.source else None,
            goal=args.goal,
        )
        print(json.dumps(result, indent=2))
    elif args.command == "check":
        result = lucky_check(file_path=args.file)
        print(json.dumps(result, indent=2))
    elif args.command == "ir":
        result = lucky_ir(file_path=args.file)
        print(json.dumps(result, indent=2))
    elif args.command == "fmt":
        result = lucky_format(file_path=args.file)
        print(json.dumps(result, indent=2))
    elif args.command == "init":
        result = lucky_init(args.path)
        print(json.dumps(result, indent=2))
