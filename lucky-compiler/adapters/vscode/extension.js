const vscode = require('vscode');
const { exec, spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

let outputChannel;
let statusBarItem;
let diagnosticCollection;

/** @param {vscode.ExtensionContext} context */
function activate(context) {
    outputChannel = vscode.window.createOutputChannel('Lucky');
    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusBarItem.text = '$(rocket) Lucky';
    statusBarItem.tooltip = 'Lucky Language';
    statusBarItem.command = 'lucky.run';
    statusBarItem.show();

    diagnosticCollection = vscode.languages.createDiagnosticCollection('lucky');

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('lucky.run', cmdRun),
        vscode.commands.registerCommand('lucky.fmt', cmdFormat),
        vscode.commands.registerCommand('lucky.test', cmdTest),
        vscode.commands.registerCommand('lucky.check', cmdCheck),
        vscode.commands.registerCommand('lucky.debug', cmdDebug),
        vscode.commands.registerCommand('lucky.ir', cmdIr),
        vscode.commands.registerCommand('lucky.init', cmdInit),
        statusBarItem
    );

    // Format on save
    vscode.workspace.onDidSaveTextDocument((doc) => {
        if (doc.languageId === 'lucky' && vscode.workspace.getConfiguration('lucky').get('formatOnSave')) {
            formatDocument(doc);
        }
        if (doc.languageId === 'lucky' && vscode.workspace.getConfiguration('lucky').get('lintOnSave')) {
            lintDocument(doc);
        }
    });

    // Lint on open
    vscode.workspace.onDidOpenTextDocument((doc) => {
        if (doc.languageId === 'lucky') {
            lintDocument(doc);
        }
    });

    outputChannel.appendLine('Lucky extension activated');
}

/** Get the Lucky binary path from config */
function luckyPath() {
    return vscode.workspace.getConfiguration('lucky').get('serverPath', 'lucky');
}

/** Execute a Lucky CLI command and return stdout */
function execLucky(args, cwd) {
    return new Promise((resolve, reject) => {
        const cmd = luckyPath();
        outputChannel.appendLine(`$ ${cmd} ${args.join(' ')}`);
        exec(`${cmd} ${args.join(' ')}`, { cwd: cwd || vscode.workspace.rootPath }, (err, stdout, stderr) => {
            if (stderr) {
                // Lucky writes diagnostics to stderr
                outputChannel.append(stderr);
            }
            if (stdout) {
                outputChannel.append(stdout);
            }
            resolve({ stdout, stderr, code: err ? err.code : 0 });
        });
    });
}

async function cmdRun() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) return;

    const filePath = editor.document.uri.fsPath;
    outputChannel.clear();
    outputChannel.appendLine(`=== Running ${filePath} ===`);

    const { stdout, stderr, code } = await execLucky(['run', filePath], path.dirname(filePath));
    if (code !== 0) {
        vscode.window.showErrorMessage('Lucky execution failed. See output for details.');
    } else {
        vscode.window.showInformationMessage('Lucky execution completed successfully.');
    }
    statusBarItem.text = code === 0 ? '$(check) Lucky' : '$(error) Lucky';
}

async function cmdFormat() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) return;

    const filePath = editor.document.uri.fsPath;
    await execLucky(['fmt', filePath]);
    vscode.window.showInformationMessage('Formatted.');
}

async function cmdTest() {
    const editor = vscode.window.activeTextEditor;
    const testPath = editor ? path.dirname(editor.document.uri.fsPath) : vscode.workspace.rootPath;
    outputChannel.clear();
    outputChannel.appendLine(`=== Running tests in ${testPath} ===`);
    const { stdout, code } = await execLucky(['test', testPath], testPath);
    if (code !== 0) {
        vscode.window.showErrorMessage('Tests failed. See output for details.');
    } else {
        vscode.window.showInformationMessage('All tests passed.');
    }
}

async function cmdCheck() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) return;
    lintDocument(editor.document);
}

async function cmdDebug() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) return;

    const filePath = editor.document.uri.fsPath;
    // Start DAP debug session
    vscode.debug.startDebugging(undefined, {
        type: 'lucky',
        name: 'Debug Lucky Program',
        request: 'launch',
        program: filePath,
        goal: '',
        stopOnEntry: false
    });
}

async function cmdIr() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) return;

    const filePath = editor.document.uri.fsPath;
    outputChannel.clear();
    const { stdout } = await execLucky(['ir', filePath], path.dirname(filePath));

    // Show IR in a new document
    const doc = await vscode.workspace.openTextDocument({ content: stdout, language: 'json' });
    await vscode.window.showTextDocument(doc, { preview: true });
}

async function cmdInit() {
    const folder = vscode.workspace.workspaceFolders?.[0];
    const projectPath = folder ? folder.uri.fsPath : vscode.workspace.rootPath;
    if (!projectPath) {
        vscode.window.showErrorMessage('No workspace folder open.');
        return;
    }

    // Create project structure
    const dirs = ['agents', 'tasks', 'memory'];
    for (const d of dirs) {
        fs.mkdirSync(path.join(projectPath, d), { recursive: true });
    }

    // Create lucky.toml
    const toml = `[package]
name = "${path.basename(projectPath)}"
version = "0.1.0"
description = "A Lucky project"
`;
    fs.writeFileSync(path.join(projectPath, 'lucky.toml'), toml);

    // Create main.lk
    const mainLk = `project ${path.basename(projectPath)}

use Claude

goal MainGoal
    success
        completed
    workflow MainWorkflow
`;
    fs.writeFileSync(path.join(projectPath, 'main.lk'), mainLk);

    vscode.window.showInformationMessage('Lucky project initialized.');
}

/** Format a document in-place */
async function formatDocument(doc) {
    const filePath = doc.uri.fsPath;
    outputChannel.appendLine(`Formatting: ${filePath}`);
    await execLucky(['fmt', filePath]);
}

/** Lint a document and show diagnostics */
async function lintDocument(doc) {
    const filePath = doc.uri.fsPath;
    const { stderr } = await execLucky(['check', filePath], path.dirname(filePath));

    const diagnostics = [];
    // Parse stderr for error/warning lines
    const lines = stderr.split('\n');
    for (const line of lines) {
        // Format: file: error message
        const match = line.match(/^(.+\.lk):\s+(error|warning)\s+(.+)$/);
        if (match) {
            // Extract line number from "here" marker
            const hereMatch = stderr.match(new RegExp(`${match[1]}:${match[2]}\\s+${match[3]}[\\s\\S]*?${match[1]}:(\\d+):(\\d+):\\s+here`, 'm'));
            let range = new vscode.Range(0, 0, 0, 0);
            if (hereMatch) {
                const lineNum = parseInt(hereMatch[1]) - 1;
                const col = parseInt(hereMatch[2]) - 1;
                range = new vscode.Range(lineNum, col, lineNum, col + 5);
            }

            diagnostics.push(new vscode.Diagnostic(
                range,
                match[3],
                match[2] === 'error' ? vscode.DiagnosticSeverity.Error : vscode.DiagnosticSeverity.Warning
            ));
        }
    }

    diagnosticCollection.set(doc.uri, diagnostics);
}

function deactivate() {
    outputChannel?.dispose();
    statusBarItem?.dispose();
    diagnosticCollection?.dispose();
}

module.exports = { activate, deactivate };
