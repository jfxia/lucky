# Lucky Runner for Cursor

Run [Lucky](https://github.com/lucky) goal-oriented agent programs directly from Cursor IDE.

## Prerequisites

- **Cursor IDE** (VS Code 1.80.0+ compatible)
- **Lucky CLI** built and available in your PATH

Build Lucky from source:

```bash
cd lucky-compiler
cargo build --release
```

Add the binary to your PATH (or set `lucky.binaryPath` in Cursor settings).

## Installation

### From source (development)

1. Open this directory in Cursor: `File > Open Folder...` and select `lucky-compiler/adapters/cursor`.

2. Press `F5` to launch the Extension Development Host.

3. Open any `.lk` file and press `Ctrl+Shift+L` to run it.

### Install via VSIX

```bash
npm install -g @vscode/vsce
cd lucky-compiler/adapters/cursor
vsce package
```

Then in Cursor: `Extensions > ... > Install from VSIX...` and select the generated `.vsix` file.

## Usage

| Command               | Keybinding       | Description                          |
|-----------------------|------------------|--------------------------------------|
| **Lucky: Run Program** | `Ctrl+Shift+L`   | Compile and execute the current `.lk` file |
| **Lucky: Show Status** | _(none)_         | Show execution status and cost       |
| **Lucky: Approve Pending Action** | _(none)_ | Respond to human-in-the-loop approval requests |

## Settings

Configure via `File > Preferences > Settings` and search for "Lucky":

| Setting              | Default       | Description                         |
|----------------------|---------------|-------------------------------------|
| `lucky.serverPort`   | `9700`        | Port for the Lucky LTP HTTP server  |
| `lucky.serverHost`   | `localhost`   | Host for the Lucky LTP HTTP server  |
| `lucky.binaryPath`   | `lucky`       | Path to the `lucky` CLI binary      |

## How it works

1. The extension compiles your `.lk` file to IR using the Lucky compiler (`lucky ir`).
2. It spawns a local LTP server (`lucky serve`) that executes the program.
3. Communication with the server uses JSON-RPC 2.0 over HTTP (the Lucky Tool Protocol).
4. Results and execution events appear in the **Lucky Runner** output channel.

## Troubleshooting

- **Server won't start**: Ensure the `lucky` binary is built and in your PATH. Check the output channel for errors.
- **Compilation errors**: Open the output channel to see full compiler diagnostics.
- **Connection refused on port 9700**: Ensure no other process is using that port, or change `lucky.serverPort` in settings.
