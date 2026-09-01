import type { Status } from "../api";
import { nomeDaRegiao } from "../util/lugares";
import { haQuantoTempo } from "../util/tempo";

/**
 * 64 px, quatro colunas. Informação de conferência, não painel: nada de
 * gráfico, e todo número tem um rótulo que um leigo entende.
 */
export function Metricas({ status, agora }: { status: Status; agora: number }) {
  const p = status.proxy_em_uso;

  return (
    <section className="grid h-16 shrink-0 grid-cols-4 items-center gap-4 border-b border-borda px-5">
      <Coluna rotulo="Proxies" valor={proxies(status.proxies_saudaveis)} />
      <Coluna
        rotulo="Em uso"
        valor={p ? emUso(p) : "nenhum"}
        apagado={!p}
      />
      <Coluna
        rotulo="Latência"
        valor={p ? `${p.latencia_ms} ms` : "—"}
        apagado={!p}
      />
      <Coluna
        rotulo="Última checagem"
        valor={haQuantoTempo(status.ultima_validacao_utc, agora)}
        apagado={!status.ultima_validacao_utc}
      />
    </section>
  );
}

function emUso(p: { regiao: string; pais: string }) {
  const lugar = nomeDaRegiao(p.regiao);
  return p.pais ? `${lugar} (${p.pais})` : lugar;
}

function proxies(n: number) {
  if (n === 0) return "nenhum";
  return `${n} ${n === 1 ? "saudável" : "saudáveis"}`;
}

function Coluna({
  rotulo,
  valor,
  apagado = false,
}: {
  rotulo: string;
  valor: string;
  apagado?: boolean;
}) {
  return (
    <div className="min-w-0">
      <div className="rotulo">{rotulo}</div>
      <div
        title={valor}
        className={`tabular mt-0.5 truncate text-[15px] ${
          apagado ? "text-texto2" : "text-texto"
        }`}
      >
        {valor}
      </div>
    </div>
  );
}
