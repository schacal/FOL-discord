/**
 * Traduz o host de uma conexão para o que ele é, em português.
 *
 * `c-gru18-6fa2a6cb.discord.media` não diz nada para quem só quer transmitir
 * a tela. "Servidor de voz — São Paulo" diz tudo: inclusive que a voz continua
 * saindo pelo Brasil, que é a promessa central do programa. O host cru
 * continua visível ao lado, porque é ele que se cola numa issue.
 *
 * Os servidores de voz se chamam `c-<cidade><n>-<hash>`, com o código de
 * aeroporto da região — a única parte do endereço que interessa. O nome em
 * português de cada código mora em `lugares.ts`, junto com o das regiões.
 */

import { CIDADES } from "./lugares";

/** Sufixo → o que aquele endereço faz na vida do usuário. */
const PAPEIS: [string, string][] = [
  ["latency.discord.media", "Escolha do servidor de voz"],
  ["status.discord.com", "Estado dos serviços"],
  ["cdn.discordapp.com", "Imagens e arquivos"],
  ["media.discordapp.net", "Imagens e vídeos"],
  ["images-ext-1.discordapp.net", "Imagens de outros sites"],
  ["images-ext-2.discordapp.net", "Imagens de outros sites"],
  ["discord.com", "Entrada no Discord"],
  ["discordapp.com", "Entrada no Discord"],
  ["discord.gg", "Convites de servidor"],
];

function casa(host: string, dominio: string): boolean {
  return host === dominio || host.endsWith(`.${dominio}`);
}

/**
 * O que aquele endereço é, ou `null` quando não sabemos — e aí quem chama
 * mostra só o endereço cru. Devolver o próprio host aqui imprimia ele duas
 * vezes na linha ("google.com  google.com").
 */
export function apelidoDeHost(host: string): string | null {
  const h = host.trim().replace(/\.$/, "").toLowerCase();

  // Servidor de voz: `c-gru18-6fa2a6cb.discord.media`
  const voz = /^c-([a-z]{3})\d*-/.exec(h);
  if (voz && casa(h, "discord.media")) {
    const cidade = CIDADES[voz[1]!];
    return cidade ? `Servidor de voz — ${cidade}` : "Servidor de voz";
  }

  // O gateway tem sabor regional e variante: no tráfego real aparecem
  // `gateway.discord.gg`, `gateway-us-east1-b.discord.gg` e
  // `remote-auth-gateway.discord.gg`. Todos são a conexão principal — e cair
  // na regra genérica de `discord.gg` os rotularia como "Convites de
  // servidor", que é o oposto do que eles são.
  if (casa(h, "discord.gg") && h.split(".")[0]!.includes("gateway")) {
    return "Conexão principal";
  }

  for (const [dominio, papel] of PAPEIS) {
    if (casa(h, dominio)) return papel;
  }

  if (casa(h, "discord.media")) return "Mídia do Discord";
  return null;
}
