/**
 * Serviço simulado — só para desenhar e testar a janela sem backend.
 *
 * Não é mock de teste automatizado: é o serviço de mentira que roda quando a
 * interface abre no navegador (`npm run dev`). Ele imita o comportamento que
 * importa — pausar manda tudo direto, a piscina seca vira `sem_proxies`, a
 * verificação demora e devolve região — para que dê para conferir os quatro
 * estados sem depender de proxies reais.
 *
 * A barra de testes no rodapé do navegador é quem mexe nos cenários daqui.
 */

import { espera } from "../util/tempo";
import type { Conexao, Estado, Servico, Status, Verificacao } from "./tipos";

export type Cenario = "operacional" | "sem_proxies" | "parado";

const HOSTS_CONTROLE = [
  "discord.com",
  "gateway.discord.gg",
  // A variante regional que aparece no tráfego de verdade.
  "gateway-us-east1-b.discord.gg",
  "latency.discord.media",
];
const HOSTS_DIRETOS = [
  "cdn.discordapp.com",
  "media.discordapp.net",
  "c-gru18-6fa2a6cb.discord.media",
  "c-gru17-851904d3.discord.media",
  // Um host que `util/hosts.ts` não traduz, de propósito: é assim que a linha
  // "sem apelido, só o endereço" aparece na única tela onde dá para vê-la hoje.
  "router.discordapp.net",
];

interface Mundo {
  cenario: Cenario;
  pausado: boolean;
  autostart: boolean;
  temAtualizacao: boolean;
  ultimaValidacao: number;
  conexoes: Conexao[];
}

const mundo: Mundo = {
  cenario: "operacional",
  pausado: false,
  autostart: true,
  temAtualizacao: false,
  ultimaValidacao: Date.now() - 2 * 60_000,
  conexoes: [],
};

const sorteio = <T,>(v: readonly T[]) => v[Math.floor(Math.random() * v.length)]!;

function estadoAtual(): Estado {
  if (mundo.cenario === "parado") return "parado";
  if (mundo.pausado) return "pausado";
  if (mundo.cenario === "sem_proxies") return "sem_proxies";
  return "operacional";
}

/** Uma conexão nova, roteada como o serviço rotearia agora. */
function novaConexao(atrasoSegundos = 0): Conexao {
  const controle = Math.random() < 0.45;
  const host = controle ? sorteio(HOSTS_CONTROLE) : sorteio(HOSTS_DIRETOS);
  // Só o tráfego de controle pode sair por fora — e só se houver saída.
  const porFora = controle && estadoAtual() === "operacional";
  return {
    hora_utc: Date.now() - atrasoSegundos * 1000,
    host,
    porta: 443,
    rota: porFora ? "exterior" : "direto",
  };
}

function registrar(atrasoSegundos = 0) {
  if (mundo.cenario === "parado") return;
  mundo.conexoes.unshift(novaConexao(atrasoSegundos));
  mundo.conexoes.sort((a, b) => Number(b.hora_utc) - Number(a.hora_utc));
  mundo.conexoes = mundo.conexoes.slice(0, 50);
}

/** Um histórico plausível: a abertura do Discord, e não nove linhas iguais. */
function semear(quantas = 9) {
  for (let i = 0; i < quantas; i++) registrar(6 + i * 3 + Math.floor(Math.random() * 3));
}

let relogio: ReturnType<typeof setInterval> | null = null;

/**
 * O mundo só começa a andar quando alguém pergunta.
 *
 * `api/index.ts` importa os dois serviços para escolher um deles em tempo de
 * execução, então este módulo é avaliado também dentro do Tauri — onde ele
 * nunca é consultado. Semear e ligar um `setInterval` no topo do arquivo
 * deixaria um cronômetro batendo para sempre dentro do aplicativo publicado.
 */
function acordar() {
  if (relogio) return;
  semear();
  relogio = setInterval(() => registrar(), 2600);
}

export const servicoSimulado: Servico = {
  async status(): Promise<Status> {
    acordar();
    await espera(70);
    if (mundo.cenario === "parado") throw new Error("serviço não está rodando");
    const operacional = estadoAtual() === "operacional";
    return {
      versao: "0.2.4",
      estado: estadoAtual(),
      autostart: mundo.autostart,
      pac_ligado: true,
      proxies_saudaveis: mundo.cenario === "sem_proxies" ? 0 : 15,
      proxy_em_uso: operacional
        ? {
            endereco: "95.81.103.220:1080",
            regiao: "rotterdam",
            pais: "NL",
            latencia_ms: 1031,
          }
        : null,
      ultima_validacao_utc: new Date(mundo.ultimaValidacao).toISOString(),
      atualizacao: mundo.temAtualizacao
        ? {
            versao: "0.3.0",
            url: "https://github.com/schacal/FOL-discord/releases/latest",
          }
        : null,
      erro_inicializacao: null,
    };
  },

  async conexoes() {
    acordar();
    await espera(50);
    if (mundo.cenario === "parado") throw new Error("serviço não está rodando");
    return [...mundo.conexoes];
  },

  async pausar() {
    await espera(180);
    mundo.pausado = true;
  },

  async retomar() {
    await espera(180);
    mundo.pausado = false;
  },

  async verificar(): Promise<Verificacao> {
    // A verificação real consulta a sonda do Discord pelo proxy em uso.
    await espera(2600);
    if (mundo.cenario === "parado") {
      // Verificar num serviço parado é o que o religa.
      mundo.cenario = "operacional";
      mundo.pausado = false;
    }
    mundo.ultimaValidacao = Date.now();
    if (mundo.cenario === "sem_proxies") {
      return {
        ok: false,
        regiao_detectada: null,
        proxies_saudaveis: 0,
        mensagem: "Nenhum proxy respondeu. Tentando de novo em 5 minutos.",
      };
    }
    if (mundo.pausado) {
      return {
        ok: false,
        regiao_detectada: "brazil",
        proxies_saudaveis: 15,
        mensagem: "A correção está pausada. Retome para sair pelo exterior.",
      };
    }
    return {
      ok: true,
      regiao_detectada: "rotterdam",
      proxies_saudaveis: 15,
      mensagem: "Saindo por Rotterdam.",
    };
  },

  async autostart(ligado) {
    await espera(220);
    mundo.autostart = ligado;
  },

  async reiniciarDiscord() {
    await espera(2200);
    return true;
  },

  async atualizar() {
    await espera(3200);
    mundo.temAtualizacao = false;
  },

  async desinstalar() {
    await espera(1400);
  },
};

// --- controles da barra de testes ------------------------------------------

export const testes = {
  cenario: () => mundo.cenario,
  definirCenario(c: Cenario) {
    mundo.cenario = c;
    if (c === "parado") mundo.conexoes = [];
    if (c !== "parado" && mundo.conexoes.length === 0) semear(6);
  },
  temAtualizacao: () => mundo.temAtualizacao,
  alternarAtualizacao() {
    mundo.temAtualizacao = !mundo.temAtualizacao;
  },
  envelhecerValidacao(minutos: number) {
    mundo.ultimaValidacao = Date.now() - minutos * 60_000;
  },
};
