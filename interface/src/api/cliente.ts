/**
 * Cliente HTTP da API de controle local.
 *
 * Só loopback, sem autenticação — mesma razão que já vale para as portas 9250
 * e 9251: a porta não é alcançável de fora da máquina.
 *
 * O serviço da 9252 **ainda não existe**. Até existir, tudo o que chega aqui é
 * promessa, e promessa se confere: nenhum campo cru atravessa para a tela. Um
 * `proxies_saudaveis` faltando tem que virar "nenhum", não "undefined
 * saudáveis" na cara de quem só queria saber se dá para transmitir a tela.
 */

import type {
  Conexao,
  Estado,
  ProxyEmUso,
  Servico,
  Status,
  Verificacao,
} from "./tipos";
import { statusParado } from "./tipos";

export const BASE = "http://127.0.0.1:9252";

/** Curto de propósito: o serviço é local. Demorou, é porque caiu. */
const RAPIDO = 2_500;
/** Ações que saem para a internet ou mexem no Discord levam segundos. */
const LENTO = 30_000;

async function pedir<T = unknown>(
  caminho: string,
  init: RequestInit = {},
  timeoutMs = RAPIDO,
): Promise<T | undefined> {
  const aborto = new AbortController();
  const relogio = setTimeout(() => aborto.abort(), timeoutMs);
  try {
    const r = await fetch(BASE + caminho, {
      ...init,
      // `/status` é lido a cada 2 s: um cache do WebView2 congelaria a janela
      // num estado antigo sem nada na tela dizendo que ela parou de ler.
      cache: "no-store",
      signal: aborto.signal,
    });
    if (!r.ok) throw new Error(`${caminho} respondeu ${r.status}`);
    if (r.status === 204) return undefined;
    const texto = await r.text();
    return texto ? (JSON.parse(texto) as T) : undefined;
  } finally {
    clearTimeout(relogio);
  }
}

const post = (corpo?: unknown): RequestInit =>
  corpo === undefined
    ? { method: "POST" }
    : {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(corpo),
      };

// --- conferência do que chega ----------------------------------------------

const campos = (v: unknown): Record<string, unknown> =>
  typeof v === "object" && v !== null && !Array.isArray(v)
    ? (v as Record<string, unknown>)
    : {};

const texto = (v: unknown) => (typeof v === "string" ? v : "");
const frase = (v: unknown) => (typeof v === "string" && v ? v : null);
const numero = (v: unknown) =>
  typeof v === "number" && Number.isFinite(v) ? v : 0;

/** Um instante serve tanto em texto ISO quanto em milissegundos de época. */
const instante = (v: unknown): string | number | null => {
  if (typeof v === "string" && v) return v;
  if (typeof v === "number" && Number.isFinite(v)) return v;
  return null;
};

function proxy(v: unknown): ProxyEmUso | null {
  const c = campos(v);
  // O portão é a região, que é o que a coluna "Em uso" desenha. Barrar por
  // `endereco` — que a janela nunca mostra — apagaria três colunas visíveis
  // por causa de um campo invisível.
  if (!texto(c.regiao)) return null;
  return {
    endereco: texto(c.endereco),
    regiao: texto(c.regiao),
    pais: texto(c.pais),
    latencia_ms: numero(c.latencia_ms),
  };
}

function status(v: unknown): Status {
  const c = campos(v);
  const base = statusParado(texto(c.versao));
  return {
    ...base,
    // O estado passa cru de propósito: um serviço mais novo pode inventar um,
    // e a tela tem uma resposta honesta para isso ("Estado desconhecido").
    estado: (typeof c.estado === "string" ? c.estado : "parado") as Estado,
    autostart: c.autostart === true,
    pac_ligado: c.pac_ligado === true,
    proxies_saudaveis: numero(c.proxies_saudaveis),
    proxy_em_uso: proxy(c.proxy_em_uso),
    // A ponte nativa devolve milissegundos de época; o contrato HTTP prometia
    // texto ISO. Os dois valem — recusar o número apagaria a coluna "Última
    // checagem" só por causa do formato.
    ultima_validacao_utc: instante(c.ultima_validacao_utc),
    // A pastilha mostra a versão; a `url` é lembrete do contrato, porque quem
    // baixa é o `POST /atualizar`. Faltar a `url` não apaga o aviso.
    atualizacao: texto(campos(c.atualizacao).versao)
      ? {
          versao: texto(campos(c.atualizacao).versao),
          url: texto(campos(c.atualizacao).url),
        }
      : null,
    erro_inicializacao: frase(c.erro_inicializacao),
  };
}

function conexao(v: unknown): Conexao | null {
  const c = campos(v);
  const hora = instante(c.hora_utc);
  if (!texto(c.host) || hora === null) return null;
  return {
    hora_utc: hora,
    host: texto(c.host),
    porta: numero(c.porta),
    rota: c.rota === "exterior" ? "exterior" : "direto",
  };
}

function verificacao(v: unknown): Verificacao {
  const c = campos(v);
  return {
    ok: c.ok === true,
    regiao_detectada: frase(c.regiao_detectada),
    proxies_saudaveis: numero(c.proxies_saudaveis),
    mensagem:
      frase(c.mensagem) ?? "O serviço respondeu, mas não disse o que achou.",
  };
}

// --- o serviço --------------------------------------------------------------

export const servicoHttp: Servico = {
  status: async () => status(await pedir("/status")),

  conexoes: async () => {
    const r = campos(await pedir("/conexoes"));
    const lista = Array.isArray(r.conexoes) ? r.conexoes : [];
    return lista.map(conexao).filter((c): c is Conexao => c !== null);
  },

  pausar: async () => {
    await pedir("/pausar", post());
  },

  retomar: async () => {
    await pedir("/retomar", post());
  },

  verificar: async () => verificacao(await pedir("/verificar", post(), LENTO)),

  autostart: async (ligado) => {
    await pedir("/autostart", post({ ligado }));
  },

  reiniciarDiscord: async () => {
    const r = campos(await pedir("/reiniciar-discord", post(), LENTO));
    return r.reiniciado === true;
  },

  atualizar: async () => {
    const resposta = campos(await pedir("/atualizar", post(), LENTO));
    const url = texto(resposta.url);
    if (!url) throw new Error("o serviço não devolveu o download da atualização");
    return url;
  },

  desinstalar: async () => {
    await pedir("/desinstalar", post(), LENTO);
  },
};
