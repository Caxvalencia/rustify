const path = require("path");
const Mocha = require("mocha");
const { glob } = require("glob");

function run() {
  const mocha = new Mocha({
    ui: "tdd",
    color: true
  });

  const testsRoot = path.resolve(__dirname, "..");

  return new Promise(async (resolve, reject) => {
    try {
      const files = await glob("**/**.test.js", { cwd: testsRoot });

      // Agrega todos los archivos al suite de mocha
      files.forEach((f) => mocha.addFile(path.resolve(testsRoot, f)));

      // Ejecuta las pruebas
      mocha.run((failures) => {
        if (failures > 0) {
          reject(new Error(`${failures} tests failed.`));
        } else {
          resolve();
        }
      });
    } catch (err) {
      reject(err);
    }
  });
}

module.exports = { run };
