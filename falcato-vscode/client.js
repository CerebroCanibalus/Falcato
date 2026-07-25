// Falcato LSP Client — Conecta VS Code con el servidor LSP de Falcato
// Se ejecuta como child process via stdio (falcato lsp)

const vscode = require('vscode');
const path = require('path');
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');

let client = null;

/**
 * Activar la extensión — inicia el cliente LSP
 */
function activate(context) {
    // Buscar el ejecutable falcato
    const falcatoExe = findFalcato();
    if (!falcatoExe) {
        vscode.window.showWarningMessage(
            'Falcato: No se encontró "falcato" en PATH. ' +
            'Instálalo desde https://github.com/CerebroCanibalus/falcato'
        );
        return;
    }

    console.log(`Falcato: LSP usando ${falcatoExe}`);

    const serverOptions = {
        run: { command: falcatoExe, args: ['lsp'] },
        debug: { command: falcatoExe, args: ['lsp'] }
    };

    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'falcato' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.fc')
        },
        diagnosticCollectionName: 'falcato'
    };

    client = new LanguageClient(
        'falcato',
        'Falcato Language Server',
        serverOptions,
        clientOptions
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('falcato.version', () => {
            const exec = require('child_process').execSync;
            try {
                const output = exec(`${falcatoExe} version`).toString().trim();
                vscode.window.showInformationMessage(`Falcato ${output}`);
            } catch (e) {
                vscode.window.showErrorMessage('Falcato: Error al obtener versión');
            }
        })
    );

    client.start();
}

/**
 * Desactivar la extensión — detiene el cliente LSP
 */
function deactivate() {
    if (client) {
        return client.stop();
    }
}

/**
 * Buscar falcato en PATH o en rutas comunes
 */
function findFalcato() {
    // 1. Buscar en PATH
    const which = require('child_process').execSync;
    try {
        const result = which('where falcato', { encoding: 'utf8', timeout: 3000 });
        const paths = result.trim().split('\n');
        if (paths.length > 0 && paths[0].trim()) {
            return paths[0].trim();
        }
    } catch (e) {
        // No está en PATH
    }

    // 2. Buscar en %USERPROFILE%\.falcato\bin
    const homePath = process.env.USERPROFILE || process.env.HOME;
    if (homePath) {
        const localPath = path.join(homePath, '.falcato', 'bin', 'falcato.exe');
        const fs = require('fs');
        if (fs.existsSync(localPath)) {
            return localPath;
        }
    }

    // 3. Buscar al lado de la extensión
    try {
        const extPath = path.join(__dirname, '..', 'falcato.exe');
        if (require('fs').existsSync(extPath)) {
            return extPath;
        }
    } catch (e) {}

    return null;
}

module.exports = { activate, deactivate };
