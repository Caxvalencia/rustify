import { execSync } from "child_process";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export default {
  meta: {
    type: "problem",
    docs: {
      description: "Run Rustify's compiler analyzer to report native incompatibilities.",
      recommended: true
    },
    schema: []
  },
  create(context) {
    return {
      Program(node) {
        const filePath = context.filename ?? context.getFilename();
        if (!filePath || filePath.includes("<input>") || filePath.includes("eslint")) {
          return;
        }

        // Buscamos la ruta al CLI compilado (tanto en release como en debug)
        const workspaceRoot = path.resolve(__dirname, "../../../..");
        const isWindows = process.platform === "win32";
        const binName = isWindows ? "rustify.exe" : "rustify";

        const paths = [
          path.join(workspaceRoot, "target/release", binName),
          path.join(workspaceRoot, "target/debug", binName),
        ];

        let cliPath = "";
        for (const p of paths) {
          if (fs.existsSync(p)) {
            cliPath = p;
            break;
          }
        }

        if (!cliPath) {
          return;
        }

        try {
          // Ejecutamos `rustify check <file> --json`
          const stdout = execSync(`"${cliPath}" check "${filePath}" --json`, {
            encoding: "utf8",
            stdio: ["ignore", "pipe", "pipe"] // Capturamos stderr también
          });

          const diagnostics = JSON.parse(stdout);
          const sourceCode = context.sourceCode ?? context.getSourceCode();

          for (const diagnostic of diagnostics) {
            const message = diagnostic.hint
              ? `${diagnostic.message} Suggestion: ${diagnostic.hint}`
              : diagnostic.message;

            const startLoc = sourceCode.getLocFromIndex(diagnostic.span.start);
            const endLoc = sourceCode.getLocFromIndex(diagnostic.span.end);

            context.report({
              node,
              loc: {
                start: startLoc,
                end: endLoc
              },
              message: `[${diagnostic.code}] ${message}`
            });
          }
        } catch (error) {
          // Si el comando check falla porque hay diagnósticos de error,
          // execSync lanza un error pero stdout puede contener los diagnósticos JSON válidos.
          const output = error.stdout || error.stderr;
          if (output) {
            try {
              // Intentamos parsear todo o buscar un fragmento JSON en la salida
              const jsonStr = output.substring(output.indexOf("["), output.lastIndexOf("]") + 1);
              const diagnostics = JSON.parse(jsonStr);
              const sourceCode = context.sourceCode ?? context.getSourceCode();

              for (const diagnostic of diagnostics) {
                const message = diagnostic.hint
                  ? `${diagnostic.message} Suggestion: ${diagnostic.hint}`
                  : diagnostic.message;

                const startLoc = sourceCode.getLocFromIndex(diagnostic.span.start);
                const endLoc = sourceCode.getLocFromIndex(diagnostic.span.end);

                context.report({
                  node,
                  loc: {
                    start: startLoc,
                    end: endLoc
                  },
                  message: `[${diagnostic.code}] ${message}`
                });
              }
            } catch (_) {
              // Si no se puede parsear la salida, no hacemos nada
            }
          }
        }
      }
    };
  }
};
