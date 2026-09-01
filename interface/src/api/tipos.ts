/**
 * Contrato da API de controle local — 127.0.0.1:9252.
 *
 * Estes tipos são a cópia fiel do que o serviço promete devolver. A interface
 * não lê o `fol.log` nem reimplementa nada: tudo o que ela mostra passa por
 * aqui. Mudou o serviço, muda este arquivo primeiro.
 */

export type Estado = "operacional" | "pausado" | "sem_proxies" | "parado";

export type Rota = "exterior" | "direto";

export interface ProxyEmUso {
  endereco: string;
  /** Região devolvida pela sonda do Discord, em minúsculas: "rotterdam". */
  regiao: string;
  /** ISO 3166-1 alfa-2: "NL". */
  pais: string;
  latencia_ms: number;
}

export interface Atualizacao {
  versao: string;
  url: string;
}

export interface Status {
  versao: string;
  estado: Estado;
  autostart: boolean;
  pac_ligado: boolean;
  proxies_saudaveis: number;
  proxy_em_uso: ProxyEmUso | null;
  ultima_validacao_utc: string | number | null;
  atualizacao: Atualizacao | null;
  erro_inicializacao: string | null;
}

export interface Conexao {
  hora_utc: string | number;
  host: string;
  porta: number;
  rota: Rota;
}

export interface Verificacao {
  ok: boolean;
  regiao_detectada: string | null;
  proxies_saudaveis: number;
  mensagem: string;
}

/**
 * Tudo o que a janela precisa do serviço. Uma implementação fala HTTP com o
 * serviço de verdade; a outra é simulada, para desenhar e testar sem backend.
 */
export interface Servico {
  status(): Promise<Status>;
  conexoes(): Promise<Conexao[]>;
  pausar(): Promise<void>;
  retomar(): Promise<void>;
  verificar(): Promise<Verificacao>;
  autostart(ligado: boolean): Promise<void>;
  reiniciarDiscord(): Promise<boolean>;
  /** Devolve o download oficial que a pessoa escolheu abrir. */
  atualizar(): Promise<string>;
  desinstalar(): Promise<void>;
}

/** Estado desconhecido: o serviço não respondeu. Vira "parado" na tela. */
export function statusParado(versao: string): Status {
  return {
    versao,
    estado: "parado",
    autostart: false,
    pac_ligado: false,
    proxies_saudaveis: 0,
    proxy_em_uso: null,
    ultima_validacao_utc: null,
    atualizacao: null,
    erro_inicializacao: null,
  };
}
