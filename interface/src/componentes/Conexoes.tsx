import type { Conexao } from "../api";
import { apelidoDeHost } from "../util/hosts";
import { haQuantoTempo } from "../util/tempo";

/**
 * A prova de que o desvio acontece na abertura e acaba depois dela: as
 * linhas mais antigas saem pelo exterior, as mais novas saem direto.
 *
 * A lista atende duas pessoas ao mesmo tempo, e por isso cada linha tem dois
 * textos: o que aquela conexão **é**, em português, para quem só quer saber se
 * está funcionando; e o endereço cru ao lado, menor, para quem vai colar numa
 * issue. Nenhum dos dois pode faltar — só o endereço não diz nada a um leigo,
 * e só o apelido não serve para depurar.
 */
export function Conexoes({
  conexoes,
  agora,
  parado,
}: {
  conexoes: Conexao[];
  agora: number;
  parado: boolean;
}) {
  const porFora = conexoes.filter((c) => c.rota === "exterior").length;
  const diretas = conexoes.length - porFora;

  return (
    <section className="flex min-h-0 flex-1 flex-col border-b border-borda">
      <div className="flex shrink-0 items-baseline gap-3 px-5 pt-2.5 pb-1.5">
        <span className="rotulo">Atividade do Discord</span>
        {conexoes.length > 0 && (
          <span
            className="tabular ml-auto text-[12.5px]"
            title="Contagem das últimas conexões registradas"
          >
            <span className="font-medium text-destaque">
              {porFora} pelo exterior
            </span>
            <span className="text-texto3"> · {diretas} direto</span>
          </span>
        )}
      </div>

      <div className="rolagem min-h-0 flex-1 overflow-y-auto pb-2">
        {conexoes.length === 0 ? (
          <p className="px-5 pt-1 text-[14px] text-texto2">
            {parado
              ? "O serviço não está rodando, então não há o que registrar."
              : "Nada ainda. Abra o Discord e a atividade aparece aqui."}
          </p>
        ) : (
          <ul>
            {/* A chave é a posição de propósito. Esta lista é um log rolante
                de tamanho fixo, sem estado por linha: cada conexão nova empurra
                todas as outras e mudaria toda chave derivada do conteúdo,
                fazendo o React destruir e recriar as 50 linhas a cada 2 s. Pela
                posição, ele remenda o texto e não toca no DOM. */}
            {conexoes.map((c, i) => (
              <Linha key={i} conexao={c} agora={agora} />
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

function Linha({ conexao: c, agora }: { conexao: Conexao; agora: number }) {
  const apelido = apelidoDeHost(c.host);
  const cru = c.porta === 443 ? c.host : `${c.host}:${c.porta}`;

  return (
    <li className="flex h-[31px] items-center gap-3 px-5">
      <Rota rota={c.rota} />

      {/* Sem apelido, o endereço cru assume o lugar dele: repetir o mesmo
          texto duas vezes na linha não ajuda nenhum dos dois leitores. */}
      <span className="min-w-0 flex-1 truncate text-[14px]">
        {apelido ? (
          <>
            {apelido}
            <span className="selecionavel ml-2 text-[12.5px] text-texto3">
              {cru}
            </span>
          </>
        ) : (
          <span className="selecionavel">{cru}</span>
        )}
      </span>

      <span className="tabular w-16 shrink-0 text-right text-[13px] text-texto2">
        {haQuantoTempo(c.hora_utc, agora)}
      </span>
    </li>
  );
}

/**
 * A seta é para quem não vai ler a palavra: uma sobe e sai, a outra segue reta.
 */
function Rota({ rota }: { rota: Conexao["rota"] }) {
  const exterior = rota === "exterior";
  return (
    <span
      className={`flex w-[74px] shrink-0 items-center gap-1.5 text-[13px] ${
        exterior ? "font-semibold text-destaque" : "text-texto2"
      }`}
      title={
        exterior
          ? "Saiu por um proxy estrangeiro — é o que corrige a região"
          : "Saiu pela sua internet, como sempre"
      }
    >
      <svg
        viewBox="0 0 12 12"
        className="size-3 shrink-0"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden
      >
        {exterior ? (
          <path d="M2.5 9.5 9.5 2.5M4.5 2.5h5v5" />
        ) : (
          <path d="M1.5 6h9M7.5 3l3 3-3 3" />
        )}
      </svg>
      {rota}
    </span>
  );
}
