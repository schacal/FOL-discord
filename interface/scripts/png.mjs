/**
 * Leitor mínimo de PNG RGBA de 8 bits, só o bastante para conferir um ícone.
 *
 * O `icones.mjs` já escreve PNG na mão; aqui é o caminho de volta. Sem
 * dependência nenhuma, porque um teste de embalagem que precisa de `npm i`
 * de uma biblioteca de imagem deixa de ser rodado.
 */

import { inflateSync } from "node:zlib";

const ASSINATURA = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);

/** Desfaz o filtro por linha do PNG (tipos 0 a 4 da especificação). */
function desfiltrar(bruto, largura, altura, canais) {
  const passo = largura * canais;
  const saida = Buffer.alloc(passo * altura);

  for (let linha = 0; linha < altura; linha++) {
    const filtro = bruto[linha * (passo + 1)];
    const entrada = bruto.subarray(
      linha * (passo + 1) + 1,
      linha * (passo + 1) + 1 + passo,
    );
    const atual = saida.subarray(linha * passo, (linha + 1) * passo);
    const acima = linha > 0 ? saida.subarray((linha - 1) * passo, linha * passo) : null;

    for (let i = 0; i < passo; i++) {
      const a = i >= canais ? atual[i - canais] : 0;
      const b = acima ? acima[i] : 0;
      const c = acima && i >= canais ? acima[i - canais] : 0;
      let valor = entrada[i];
      switch (filtro) {
        case 0:
          break;
        case 1:
          valor += a;
          break;
        case 2:
          valor += b;
          break;
        case 3:
          valor += (a + b) >> 1;
          break;
        case 4: {
          const p = a + b - c;
          const da = Math.abs(p - a);
          const db = Math.abs(p - b);
          const dc = Math.abs(p - c);
          valor += da <= db && da <= dc ? a : db <= dc ? b : c;
          break;
        }
        default:
          throw new Error(`filtro de PNG desconhecido: ${filtro}`);
      }
      atual[i] = valor & 0xff;
    }
  }
  return saida;
}

/**
 * Devolve `{ largura, altura, canais, pixels }` de um PNG sem entrelaçamento,
 * com 8 bits por canal. Recusa o resto em vez de devolver dado errado.
 */
export function lerPng(arquivo) {
  if (!arquivo.subarray(0, 8).equals(ASSINATURA)) {
    throw new Error("não é um PNG");
  }

  let cabecalho = null;
  const partes = [];
  let posicao = 8;
  while (posicao < arquivo.length) {
    const tamanho = arquivo.readUInt32BE(posicao);
    const tipo = arquivo.toString("ascii", posicao + 4, posicao + 8);
    const dados = arquivo.subarray(posicao + 8, posicao + 8 + tamanho);
    if (tipo === "IHDR") {
      cabecalho = {
        largura: dados.readUInt32BE(0),
        altura: dados.readUInt32BE(4),
        bits: dados[8],
        cor: dados[9],
        entrelacado: dados[12],
      };
    } else if (tipo === "IDAT") {
      partes.push(dados);
    } else if (tipo === "IEND") {
      break;
    }
    posicao += tamanho + 12;
  }

  if (!cabecalho) throw new Error("PNG sem IHDR");
  if (cabecalho.bits !== 8 || cabecalho.entrelacado !== 0) {
    throw new Error("só leio PNG de 8 bits sem entrelaçamento");
  }
  const canais = { 0: 1, 2: 3, 4: 2, 6: 4 }[cabecalho.cor];
  if (!canais) throw new Error(`tipo de cor sem paleta esperado, veio ${cabecalho.cor}`);

  return {
    largura: cabecalho.largura,
    altura: cabecalho.altura,
    canais,
    pixels: desfiltrar(
      inflateSync(Buffer.concat(partes)),
      cabecalho.largura,
      cabecalho.altura,
      canais,
    ),
  };
}

/**
 * Quantas cores distintas o desenho usa, ignorando o que é transparente.
 *
 * É o que separa uma logo ilustrada de uma marca desenhada por fórmula: a
 * segunda tem um punhado de cores chapadas, a primeira tem centenas.
 */
export function coresDistintas(arquivo) {
  const { pixels, canais } = lerPng(arquivo);
  const vistas = new Set();
  for (let i = 0; i < pixels.length; i += canais) {
    const alfa = canais === 4 ? pixels[i + 3] : 255;
    if (alfa < 128) continue;
    vistas.add((pixels[i] << 16) | (pixels[i + 1] << 8) | pixels[i + 2]);
  }
  return vistas.size;
}
