const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;

function activate(context) {
  const configuration = vscode.workspace.getConfiguration("rustify");
  const serverPath = configuration.get("server.path", "rustify-lsp");
  const cliPath = configuration.get("cli.path", "rustify");
  client = new LanguageClient(
    "rustify",
    "Rustify Language Server",
    { command: serverPath, transport: TransportKind.stdio },
    { documentSelector: [{ scheme: "file", language: "typescript" }] }
  );
  context.subscriptions.push(client.start());
  context.subscriptions.push(
    vscode.commands.registerCommand("rustify.preview", async () => {
      const editor = vscode.window.activeTextEditor;
      if (editor) {
        try {
          const rust = await client.sendRequest("workspace/executeCommand", {
            command: "rustify.preview",
            arguments: [editor.document.uri.toString()]
          });
          const preview = await vscode.workspace.openTextDocument({
            content: rust,
            language: "rust"
          });
          await vscode.window.showTextDocument(preview, {
            preview: true,
            viewColumn: vscode.ViewColumn.Beside
          });
        } catch (error) {
          vscode.window.showErrorMessage(`Rustify preview failed: ${error.message ?? error}`);
        }
      }
    })
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("rustify.check", () => {
      const editor = vscode.window.activeTextEditor;
      if (editor) {
        const terminal = vscode.window.createTerminal("Rustify Check");
        terminal.sendText(`${shellQuote(cliPath)} check ${shellQuote(editor.document.fileName)}`);
        terminal.show();
      }
    })
  );
}

function deactivate() {
  return client?.stop();
}

module.exports = { activate, deactivate };

function shellQuote(value) {
  return `'${String(value).replaceAll("'", "'\\''")}'`;
}
