const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;

function activate(context) {
  client = new LanguageClient(
    "rustify",
    "Rustify Language Server",
    { command: "rustify-lsp", transport: TransportKind.stdio },
    { documentSelector: [{ scheme: "file", language: "typescript" }] }
  );
  context.subscriptions.push(client.start());
  context.subscriptions.push(
    vscode.commands.registerCommand("rustify.preview", () => {
      const editor = vscode.window.activeTextEditor;
      if (editor) {
        const terminal = vscode.window.createTerminal("Rustify Preview");
        terminal.sendText(`rustify explain "${editor.document.fileName}"`);
        terminal.show();
      }
    })
  );
}

function deactivate() {
  return client?.stop();
}

module.exports = { activate, deactivate };
