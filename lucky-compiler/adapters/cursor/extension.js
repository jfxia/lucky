const vscode = require("vscode");
const http = require("http");
const child_process = require("child_process");
const path = require("path");
const fs = require("fs");

const LTP_ENDPOINT = "/ltp/v1";

let outputChannel;
let statusBarItem;
let serverProcess = null;
let sessionId = null;
let nextId = 1;

function activate(context) {
  outputChannel = vscode.window.createOutputChannel("Lucky Runner");
  outputChannel.appendLine("Lucky Runner activated");

  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    100
  );
  statusBarItem.text = "$(debug-start) Lucky";
  statusBarItem.tooltip = "Lucky Runner — click to run";
  statusBarItem.command = "lucky.run";
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

  context.subscriptions.push(
    vscode.commands.registerCommand("lucky.run", cmdRun)
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("lucky.status", cmdStatus)
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("lucky.approve", cmdApprove)
  );

  context.subscriptions.push({ dispose: () => stopServer() });

  outputChannel.appendLine("Commands registered: lucky.run, lucky.status, lucky.approve");
}

function deactivate() {
  stopServer();
  if (outputChannel) {
    outputChannel.dispose();
  }
}

function config() {
  return vscode.workspace.getConfiguration("lucky");
}

function serverUrl() {
  const host = config().get("serverHost", "localhost");
  const port = config().get("serverPort", 9700);
  return `http://${host}:${port}`;
}

function binaryPath() {
  return config().get("binaryPath", "lucky");
}

function ltpRequest(method, params) {
  return new Promise((resolve, reject) => {
    const id = nextId++;
    const body = JSON.stringify({
      jsonrpc: "2.0",
      method: method,
      params: params || {},
      id: id,
    });

    const url = new URL(LTP_ENDPOINT, serverUrl());
    const options = {
      hostname: url.hostname,
      port: url.port,
      path: url.pathname,
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Content-Length": Buffer.byteLength(body),
      },
      timeout: 300000,
    };

    if (sessionId) {
      options.headers["Ltp-Session-Id"] = sessionId;
    }

    const req = http.request(options, (res) => {
      let data = "";
      res.on("data", (chunk) => {
        data += chunk;
      });
      res.on("end", () => {
        try {
          const response = JSON.parse(data);
          if (response.error) {
            reject(
              new Error(
                `LTP Error ${response.error.code}: ${response.error.message}`
              )
            );
          } else {
            resolve(response.result || {});
          }
        } catch (e) {
          reject(new Error(`Failed to parse LTP response: ${e.message}`));
        }
      });
    });

    req.on("error", (e) => {
      reject(new Error(`LTP connection failed: ${e.message}`));
    });

    req.on("timeout", () => {
      req.destroy();
      reject(new Error("LTP request timed out"));
    });

    req.write(body);
    req.end();
  });
}

function startServer() {
  return new Promise((resolve, reject) => {
    if (serverProcess && !serverProcess.killed) {
      resolve();
      return;
    }

    const cmd = binaryPath();
    const port = config().get("serverPort", 9700);

    outputChannel.appendLine(`Starting Lucky server: ${cmd} serve --port ${port}`);

    try {
      serverProcess = child_process.spawn(cmd, ["serve", "--port", String(port)], {
        stdio: ["pipe", "pipe", "pipe"],
        windowsHide: true,
      });

      serverProcess.stdout.on("data", (data) => {
        outputChannel.appendLine(`[server] ${data.toString().trim()}`);
      });

      serverProcess.stderr.on("data", (data) => {
        outputChannel.appendLine(`[server:err] ${data.toString().trim()}`);
      });

      serverProcess.on("error", (err) => {
        outputChannel.appendLine(`[server] Failed to start: ${err.message}`);
        reject(err);
      });

      serverProcess.on("exit", (code) => {
        outputChannel.appendLine(`[server] Exited with code ${code}`);
        serverProcess = null;
        sessionId = null;
      });

      setTimeout(() => resolve(), 1000);
    } catch (e) {
      reject(new Error(`Failed to spawn server: ${e.message}`));
    }
  });
}

function stopServer() {
  if (serverProcess && !serverProcess.killed) {
    outputChannel.appendLine("Stopping Lucky server...");
    try {
      serverProcess.kill();
    } catch (e) {
      // ignore
    }
    serverProcess = null;
    sessionId = null;
  }
}

async function ensureSession() {
  await startServer();
  if (!sessionId) {
    const result = await ltpRequest("session/initialize", {
      protocol_version: "0.1",
      client_info: {
        name: "lucky-runner",
        version: "0.1.0",
      },
      capabilities: {
        streaming: true,
        batch: true,
        human_approval: true,
      },
    });
    sessionId = result.session_id;
    outputChannel.appendLine(`Session initialized: ${sessionId}`);
  }
}

async function compileToIR(filePath) {
  const source = fs.readFileSync(filePath, "utf-8");
  const cmd = binaryPath();
  const tmpDir = path.join(
    require("os").tmpdir(),
    "lucky-runner"
  );
  if (!fs.existsSync(tmpDir)) {
    fs.mkdirSync(tmpDir, { recursive: true });
  }
  const irPath = path.join(tmpDir, `${path.basename(filePath, ".lk")}.lir`);
  fs.writeFileSync(irPath, "");

  return new Promise((resolve, reject) => {
    const proc = child_process.spawn(cmd, ["ir", "--opt", "O2", filePath], {
      stdio: ["pipe", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    proc.stdout.on("data", (data) => {
      stdout += data.toString();
    });

    proc.stderr.on("data", (data) => {
      stderr += data.toString();
      outputChannel.appendLine(`[compile] ${data.toString().trim()}`);
    });

    proc.on("error", (err) => {
      reject(new Error(`Failed to spawn compiler: ${err.message}`));
    });

    proc.on("exit", (code) => {
      if (code !== 0) {
        reject(new Error(`Compilation failed (exit ${code}):\n${stderr}`));
        return;
      }

      try {
        const ir = JSON.parse(stdout);
        if (ir.hir) {
          fs.writeFileSync(irPath, JSON.stringify(ir.hir, null, 2));
          resolve({ ir, irPath });
        } else {
          resolve({ ir, irPath: null });
        }
      } catch (e) {
        reject(new Error(`Failed to parse IR output: ${e.message}\n${stdout.substring(0, 500)}`));
      }
    });
  });
}

async function cmdRun() {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showErrorMessage("No active editor. Open a .lk file first.");
    return;
  }

  const filePath = editor.document.uri.fsPath;
  if (!filePath.endsWith(".lk")) {
    vscode.window.showErrorMessage("Active file is not a Lucky (.lk) file.");
    return;
  }

  if (editor.document.isDirty) {
    await editor.document.save();
  }

  statusBarItem.text = "$(sync~spin) Lucky: compiling...";
  statusBarItem.tooltip = "Compiling...";
  outputChannel.clear();
  outputChannel.show(true);

  let irResult;
  try {
    outputChannel.appendLine(`Compiling: ${filePath}`);
    irResult = await compileToIR(filePath);
    outputChannel.appendLine(`Compiled successfully. HIR nodes: ${irResult.ir.hir ? "present" : "none"}`);
  } catch (e) {
    outputChannel.appendLine(`Compilation failed: ${e.message}`);
    statusBarItem.text = "$(error) Lucky: compile failed";
    vscode.window.showErrorMessage(`Lucky compilation failed: ${e.message}`);
    return;
  }

  const goal = await vscode.window.showInputBox({
    prompt: "Enter goal name to pursue (leave empty for default)",
    placeHolder: "e.g. SayHello",
  });

  statusBarItem.text = "$(sync~spin) Lucky: starting server...";

  try {
    await ensureSession();
  } catch (e) {
    outputChannel.appendLine(`Failed to connect to server: ${e.message}`);
    statusBarItem.text = "$(error) Lucky: server offline";
    vscode.window.showErrorMessage(`Lucky server unavailable: ${e.message}`);
    return;
  }

  try {
    statusBarItem.text = "$(sync~spin) Lucky: loading IR...";
    outputChannel.appendLine("Loading IR...");

    await ltpRequest("ir/load", {
      ir: irResult.ir.hir,
      options: { validate: true },
    });

    statusBarItem.text = "$(sync~spin) Lucky: executing...";
    outputChannel.appendLine(goal ? `Executing goal: ${goal}` : "Executing (default entry)");

    const result = await ltpRequest("execution/start", {
      context: {},
      mode: "sync",
      ...(goal ? { entry_point: goal, entry_kind: "goal" } : {}),
    });

    outputChannel.appendLine("");
    outputChannel.appendLine("=== Execution Result ===");
    outputChannel.appendLine(JSON.stringify(result, null, 2));

    if (result.cost) {
      outputChannel.appendLine(`\nCost: $${result.cost.total_usd || "N/A"}`);
    }
    if (result.duration_ms) {
      outputChannel.appendLine(`Duration: ${result.duration_ms}ms`);
    }

    statusBarItem.text = "$(check) Lucky: done";
    statusBarItem.tooltip = "Execution completed successfully";
    vscode.window.showInformationMessage("Lucky program executed successfully.");
  } catch (e) {
    outputChannel.appendLine(`\nExecution failed: ${e.message}`);
    statusBarItem.text = "$(error) Lucky: failed";
    statusBarItem.tooltip = e.message;
    vscode.window.showErrorMessage(`Lucky execution failed: ${e.message}`);
  }
}

async function cmdStatus() {
  if (!sessionId) {
    vscode.window.showInformationMessage("Lucky Runner: No active execution session.");
    return;
  }

  try {
    const executions = await ltpRequest("execution/list", {});
    outputChannel.clear();
    outputChannel.show(true);

    if (executions.executions && executions.executions.length > 0) {
      outputChannel.appendLine("=== Active Executions ===");
      for (const exec of executions.executions) {
        outputChannel.appendLine(
          `  ${exec.id}: ${exec.status || "unknown"} (${exec.entry_point || "-"})`
        );
        const status = await ltpRequest("execution/get_status", {
          execution_id: exec.id,
        });
        outputChannel.appendLine(JSON.stringify(status, null, 2));
      }
    } else {
      outputChannel.appendLine("No active executions.");
    }

    const cost = await ltpRequest("query/cost", {});
    outputChannel.appendLine("\n=== Cost Summary ===");
    outputChannel.appendLine(JSON.stringify(cost, null, 2));
  } catch (e) {
    vscode.window.showErrorMessage(`Failed to get status: ${e.message}`);
  }
}

async function cmdApprove() {
  if (!sessionId) {
    const connect = await vscode.window.showInformationMessage(
      "No active Lucky session. Start the server?",
      "Yes",
      "No"
    );
    if (connect === "Yes") {
      try {
        await ensureSession();
      } catch (e) {
        vscode.window.showErrorMessage(`Failed to start server: ${e.message}`);
        return;
      }
    } else {
      return;
    }
  }

  try {
    const result = await ltpRequest("approval/list", {});
    const pending = result.pending || [];

    if (pending.length === 0) {
      vscode.window.showInformationMessage("No pending approval requests.");
      return;
    }

    for (const approval of pending) {
      const label = approval.description || approval.action || approval.id || "Unknown action";
      const detail = approval.agent
        ? `Agent: ${approval.agent} — ${approval.reason || "No reason provided"}`
        : approval.reason || "";

      const decision = await vscode.window.showQuickPick(["Approve", "Reject", "Skip"], {
        title: `Lucky Approval: ${label}`,
        placeHolder: detail || "Choose action",
      });

      if (!decision || decision === "Skip") {
        continue;
      }

      statusBarItem.text = "$(sync~spin) Lucky: sending approval...";
      const response = await ltpRequest("approval/respond", {
        approval_id: approval.id,
        decision: decision === "Approve" ? "approved" : "rejected",
        reason: `User ${decision.toLowerCase()}d via Cursor`,
      });

      statusBarItem.text = "$(check) Lucky";
      outputChannel.appendLine(
        `${decision}d: ${label} (${approval.id})`
      );
    }
  } catch (e) {
    vscode.window.showErrorMessage(`Approval failed: ${e.message}`);
    statusBarItem.text = "$(error) Lucky";
  }
}

module.exports = { activate, deactivate };
