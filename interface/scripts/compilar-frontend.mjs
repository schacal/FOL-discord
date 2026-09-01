import { spawnSync } from "node:child_process";

function rodar(script, argumentos) {
  const resultado = spawnSync(process.execPath, [script, ...argumentos], {
    stdio: "inherit",
  });
  if (resultado.error) throw resultado.error;
  if (resultado.status !== 0) process.exit(resultado.status ?? 1);
}

rodar("node_modules/typescript/bin/tsc", ["--noEmit"]);
rodar("node_modules/vite/bin/vite.js", ["build", "--configLoader", "runner"]);
