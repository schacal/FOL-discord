import type { Status } from "../api";
import { nomeDaRegiao } from "../util/lugares";
import { Orbe, type Sinal } from "./Orbe";
import { Botao } from "./ui/Botao";

export type Aviso = {
  tom: "ok" | "atencao";
  titulo: string;
  frase: string;
};

/** `carregando` só existe até a primeira resposta do serviço. */
export type Fase = "carregando" | "verificando" | "normal";

/**
 * Seis orbes, um por situação, escolhidos pelo que o nome descreve — e o
 * ritmo faz o resto do trabalho. Nenhum fica congelado: orbe parado é lido
 * como interface travada, e não como serviço parado.
 */
const SINAIS = {
  /** A constelação se ligando: é literalmente o que está acontecendo. */
  conectando: {
    orbe: "connecting",
    tinta: "destaque",
  },
  /** As faixas embaralham e "clicam" resolvidas — a verificação em curso. */
  verificando: {
    orbe: "solving",
    tinta: "destaque",
    ritmo: 1.15,
  },
  /** Partículas em órbita: o tráfego saindo e voltando, sem pressa. */
  operacional: {
    orbe: "working",
    tinta: "ok",
  },
  /** Um anel respirando devagar: vivo, de propósito sem fazer nada. */
  pausado: {
    orbe: "breathing",
    tinta: "neutro",
    ritmo: 0.5,
  },
  /** A meridiana varrendo o globo, procurando. O nome já diz. */
  sem_proxies: {
    orbe: "searching",
    tinta: "atencao",
  },
  /** Um contorno que troca de forma sem assentar em nenhuma. */
  parado: {
    orbe: "shaping",
    tinta: "perigo",
    ritmo: 0.85,
  },
} as const satisfies Record<string, Sinal>;

/**
 * O coração da janela: o orbe, uma palavra e uma frase que diz o que aquilo
 * significa na prática. A frase importa mais que o rótulo — "Funcionando" não
 * responde nada a quem só quer compartilhar a tela.
 */
export function BlocoStatus({
  status,
  fase,
  aviso,
  ocupado,
  aoAlternarPausa,
}: {
  status: Status;
  fase: Fase;
  aviso: Aviso | null;
  ocupado: boolean;
  aoAlternarPausa: () => void;
}) {
  const { titulo, frase, sinal } = descrever(status, fase, aviso);
  const pausado = status.estado === "pausado";

  return (
    <section className="flex h-[92px] shrink-0 items-center gap-4 border-b border-borda px-5">
      <Orbe sinal={sinal} className="shrink-0" />

      <div className="min-w-0 flex-1">
        <h1 className="truncate text-[22px] leading-tight font-semibold tracking-[-0.015em]">
          {titulo}
        </h1>
        {/* A frase é o que o usuário realmente lê. Ela quebra em duas linhas
            antes de ser cortada: a mais longa mede 528 px e a coluna tem 504,
            e encolher a fonte para caber tiraria justamente a legibilidade. */}
        <p className="mt-0.5 line-clamp-2 text-[14px] leading-[1.35] text-texto2">
          {frase}
        </p>
      </div>

      <Botao
        variante="primario"
        tamanho="grande"
        className="w-[100px]"
        disabled={status.estado === "parado" || fase === "carregando"}
        ocupado={ocupado}
        onClick={aoAlternarPausa}
        title={
          pausado
            ? "Voltar a mandar a abertura do Discord pelo exterior"
            : "Mandar tudo direto, como se o programa não existisse"
        }
      >
        {pausado ? "Retomar" : "Pausar"}
      </Botao>
    </section>
  );
}

function descrever(
  status: Status,
  fase: Fase,
  aviso: Aviso | null,
): { titulo: string; frase: string; sinal: Sinal } {
  if (fase === "carregando") {
    return {
      titulo: "Conectando",
      frase: "Falando com o serviço.",
      sinal: SINAIS.conectando,
    };
  }

  if (fase === "verificando") {
    return {
      titulo: "Verificando",
      frase: "Consultando o Discord pelo proxy em uso. Leva alguns segundos.",
      sinal: SINAIS.verificando,
    };
  }

  if (aviso) {
    return {
      titulo: aviso.titulo,
      frase: aviso.frase,
      sinal: aviso.tom === "ok" ? SINAIS.operacional : SINAIS.sem_proxies,
    };
  }

  switch (status.estado) {
    case "operacional": {
      const cidade = nomeDaRegiao(status.proxy_em_uso?.regiao);
      return {
        titulo: "Funcionando",
        frase: `Sua sessão está saindo por ${cidade}. Tela e câmera liberadas.`,
        sinal: SINAIS.operacional,
      };
    }
    case "pausado":
      return {
        titulo: "Pausado",
        frase:
          "O Discord está saindo pelo seu IP normal. A correção está desligada.",
        sinal: SINAIS.pausado,
      };
    case "sem_proxies":
      return {
        titulo: "Procurando saída",
        frase:
          "Nenhum proxy respondeu. O Discord continua funcionando, só sem a correção.",
        sinal: SINAIS.sem_proxies,
      };
    case "parado":
      return {
        titulo: "Parado",
        frase:
          "O serviço não está rodando. Clique em Verificar agora para religar.",
        sinal: SINAIS.parado,
      };
  }

  // Um serviço mais novo que esta janela pode inventar um estado. Melhor dizer
  // isso do que fingir que está tudo bem.
  return {
    titulo: "Estado desconhecido",
    frase: `O serviço respondeu "${status.estado}", que esta janela não conhece.`,
    sinal: SINAIS.sem_proxies,
  };
}
