/**
 * Gera os PNGs compactos da bandeja a partir do "L" que sai e volta.
 *
 * O ícone do programa — janela, barra de tarefas, atalho e instalador — é a
 * logo ilustrada de `assets/icons/app.png`, gerada com `npx tauri icon`. Só a
 * bandeja continua desenhada por fórmula, porque ela muda de cor por estado e
 * precisa continuar legível em 16 px.
 *
 * Sem dependência nenhuma: rasteriza por distância (cápsulas e retângulo
 * arredondado) e escreve o PNG na mão, com o zlib que já vem no Node.
 *
 *   node scripts/icones.mjs
 */

import { deflateSync } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const AQUI = dirname(fileURLToPath(import.meta.url));
const SAIDA = join(AQUI, "..", "src-tauri", "icones");

// A marca vive num quadrado de 16 unidades, igual ao viewBox do SVG.
const TRACO = 2.0;
const SEGMENTOS = [
  [4.4, 2.6, 4.4, 9.9], // a haste do L
  [4.4, 9.9, 6.0, 11.4], // o joelho
  [6.0, 11.4, 11.2, 11.4], // a base
  [9.4, 9.1, 11.9, 11.4], // a ponta da seta
  [11.9, 11.4, 9.4, 13.7],
];

const CORES = {
  destaque: [0x4f, 0x46, 0xe5],
  operacional: [0x16, 0xa3, 0x4a],
  pausado: [0x78, 0x71, 0x6c],
  sem_proxies: [0xd9, 0x77, 0x06],
  parado: [0xdc, 0x26, 0x26],
};

// --- distâncias -------------------------------------------------------------

function distSegmento(px, py, ax, ay, bx, by) {
  const vx = bx - ax;
  const vy = by - ay;
  const wx = px - ax;
  const wy = py - ay;
  const t = Math.max(0, Math.min(1, (wx * vx + wy * vy) / (vx * vx + vy * vy)));
  const dx = wx - t * vx;
  const dy = wy - t * vy;
  return Math.hypot(dx, dy);
}

function distMarca(x, y) {
  let d = Infinity;
  for (const [ax, ay, bx, by] of SEGMENTOS) {
    d = Math.min(d, distSegmento(x, y, ax, ay, bx, by) - TRACO / 2);
  }
  return d;
}

/** Retângulo arredondado centrado em 8,8. */
function distFundo(x, y, meio, raio) {
  const dx = Math.abs(x - 8) - (meio - raio);
  const dy = Math.abs(y - 8) - (meio - raio);
  const fx = Math.max(dx, 0);
  const fy = Math.max(dy, 0);
  return Math.min(Math.max(dx, dy), 0) + Math.hypot(fx, fy) - raio;
}

/** Cobertura antisserrilhada: a distância vira alfa ao longo de um pixel. */
const cobertura = (d, pixel) => Math.max(0, Math.min(1, 0.5 - d / pixel));

// --- desenho ----------------------------------------------------------------

/** Marca branca sobre um quadrado arredondado da cor pedida. */
function desenhar(lado, cor) {
  const px = new Uint8Array(lado * lado * 4);
  const unidade = 16 / lado; // quanto de "unidade da marca" cabe num pixel

  for (let j = 0; j < lado; j++) {
    for (let i = 0; i < lado; i++) {
      const x = (i + 0.5) * unidade;
      const y = (j + 0.5) * unidade;

      const aFundo = cobertura(distFundo(x, y, 7.4, 3.4), unidade);
      const aMarca = cobertura(distMarca(x, y), unidade);

      const alfa = Math.max(aFundo, aMarca);
      if (alfa <= 0) continue;

      const k = (j * lado + i) * 4;
      for (let canal = 0; canal < 3; canal++) {
        px[k + canal] = Math.round(
          cor[canal] * (1 - aMarca) + 255 * aMarca,
        );
      }
      px[k + 3] = Math.round(alfa * 255);
    }
  }
  return px;
}

// --- PNG --------------------------------------------------------------------

const TABELA_CRC = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = TABELA_CRC[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function pedaco(tipo, dados) {
  const cabecalho = Buffer.alloc(8);
  cabecalho.writeUInt32BE(dados.length, 0);
  cabecalho.write(tipo, 4, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([Buffer.from(tipo, "ascii"), dados])), 0);
  return Buffer.concat([cabecalho, dados, crc]);
}

function png(lado, px) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(lado, 0);
  ihdr.writeUInt32BE(lado, 4);
  ihdr[8] = 8; // 8 bits por canal
  ihdr[9] = 6; // RGBA
  const linhas = Buffer.alloc(lado * (lado * 4 + 1));
  for (let j = 0; j < lado; j++) {
    linhas[j * (lado * 4 + 1)] = 0; // filtro "nenhum"
    Buffer.from(px.buffer, j * lado * 4, lado * 4).copy(
      linhas,
      j * (lado * 4 + 1) + 1,
    );
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    pedaco("IHDR", ihdr),
    pedaco("IDAT", deflateSync(linhas, { level: 9 })),
    pedaco("IEND", Buffer.alloc(0)),
  ]);
}

// --- saída ------------------------------------------------------------------

mkdirSync(SAIDA, { recursive: true });

const escrever = (nome, lado, cor) => {
  const caminho = join(SAIDA, nome);
  writeFileSync(caminho, png(lado, desenhar(lado, cor)));
  console.log(`  ${nome}  ${lado}x${lado}`);
};

// O ícone do aplicativo não sai daqui: ele é a logo ilustrada de
// `assets/icons/app.png`, a mesma que a janela mostra no cabeçalho. Este
// script desenha só a bandeja, que precisa de uma cor por estado.
console.log("bandeja:");
for (const estado of ["operacional", "pausado", "sem_proxies", "parado"]) {
  escrever(`bandeja-${estado}.png`, 64, CORES[estado]);
}
