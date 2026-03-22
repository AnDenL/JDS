const vscode = require('vscode');
const path = require('path');
const os = require('os');

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
    let terminal = null;

    const getTerminal = () => {
        if (!terminal || terminal.exitStatus !== undefined) {
            terminal = vscode.window.createTerminal("JDS Toolchain");
        }
        return terminal;
    };

    const saveFile = async (document) => {
        if (document.isDirty) {
            await document.save();
        }
    };

    let buildCmd = vscode.commands.registerCommand('jds.build', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;

        await saveFile(editor.document);
        const filePath = editor.document.fileName;
        const term = getTerminal();
        
        term.show(true);
        term.sendText(`jc "${filePath}"`);
    });

    let runCmd = vscode.commands.registerCommand('jds.run', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;

        await saveFile(editor.document);
        
        const filePath = editor.document.fileName;
        const fileDir = path.dirname(filePath);
        const fileNameNoExt = path.basename(filePath, '.jds');
        const isWindows = os.platform() === 'win32';
        
        const exeName = isWindows ? `${fileNameNoExt}.exe` : `./${fileNameNoExt}`;
        const term = getTerminal();

        term.show(true);
        
        if (isWindows) {
            term.sendText(`jc "${filePath}"; if ($?) { .\\${exeName} }`);
        } else {
            term.sendText(`jc "${filePath}" && ${exeName}`);
        }
    });

    context.subscriptions.push(buildCmd, runCmd);
}

function deactivate() {}

module.exports = { activate, deactivate };