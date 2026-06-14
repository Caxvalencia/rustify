const assert = require("assert");
const vscode = require("vscode");

suite("Rustify Extension Integration Test Suite", () => {
  vscode.window.showInformationMessage("Start all tests.");

  test("Extension should be present and activated", async () => {
    const extension = vscode.extensions.getExtension("caxvalencia.vscode-rustify");
    assert.ok(extension, "Extension should be present.");
    
    if (!extension.isActive) {
      await extension.activate();
    }
    assert.strictEqual(extension.isActive, true, "Extension should be activated.");
  });

  test("Commands should be registered in vscode", async () => {
    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes("rustify.preview"), "Command rustify.preview should be registered.");
    assert.ok(commands.includes("rustify.check"), "Command rustify.check should be registered.");
  });
});
