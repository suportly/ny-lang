const { workspace } = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;

function activate(context) {
  const config = workspace.getConfiguration("ny");
  const lspPath = config.get("lspPath", "ny-lsp");

  const serverOptions = {
    command: lspPath,
    transport: TransportKind.stdio,
  };

  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "ny" }],
  };

  client = new LanguageClient("ny-lsp", "Ny Language Server", serverOptions, clientOptions);
  client.start();
}

function deactivate() {
  if (client) return client.stop();
}

module.exports = { activate, deactivate };
