import { useState } from "react";
import clsx from "clsx";
import { Interruptor } from "./ui/Interruptor";
import { Botao } from "./ui/Botao";

/**
 * 56 px. Três ações e nada mais.
 *
 * "Reiniciar Discord" confirma antes, porque pode derrubar uma chamada em
 * andamento — mas confirma **na própria linha**, e não numa sobreposição: a
 * janela só admite uma, e ela é a de desinstalar.
 */
export function Acoes({
  verificando,
  reiniciando,
  autostart,
  autostartOcupado,
  desabilitado,
  aoVerificar,
  aoReiniciarDiscord,
  aoMudarAutostart,
}: {
  verificando: boolean;
  reiniciando: boolean;
  autostart: boolean;
  autostartOcupado: boolean;
  desabilitado: boolean;
  aoVerificar: () => void;
  aoReiniciarDiscord: () => void;
  aoMudarAutostart: (v: boolean) => void;
}) {
  // A confirmação espera uma resposta. Ela não pode sumir sozinha: o
  // interruptor de autostart ocupa este mesmo lugar, e um clique atrasado em
  // "Reiniciar" acabaria desligando o autostart em vez de reiniciar nada.
  const [confirmando, setConfirmando] = useState(false);
  const perguntando = confirmando && !reiniciando;

  // Os 56 px são orçamento de layout da janela, não decoração: eles ficam na
  // seção, uma vez, para os dois estados não poderem divergir de altura.
  return (
    <section
      className={clsx(
        "flex h-14 shrink-0 items-center border-b border-borda px-5",
        perguntando ? "gap-3 bg-[#FDFCFB]" : "gap-2",
      )}
    >
      {perguntando ? (
        <>
          <span className="flex-1 truncate text-[14px] text-texto">
            Reiniciar o Discord agora?{" "}
            <span className="text-texto2">
              Uma chamada em andamento é encerrada.
            </span>
          </span>
          <Botao variante="texto" onClick={() => setConfirmando(false)}>
            Agora não
          </Botao>
          <Botao
            variante="primario"
            onClick={() => {
              setConfirmando(false);
              aoReiniciarDiscord();
            }}
          >
            Reiniciar
          </Botao>
        </>
      ) : (
        <>
      <Botao
        variante="secundario"
        ocupado={verificando}
        onClick={aoVerificar}
        title="Revalida a piscina e consulta o Discord de verdade pelo proxy em uso"
      >
        {verificando ? "Verificando" : "Verificar agora"}
      </Botao>

      <Botao
        variante="secundario"
        ocupado={reiniciando}
        disabled={desabilitado}
        onClick={() => setConfirmando(true)}
        title="A correção só passa a valer numa sessão nova do Discord"
      >
        {reiniciando ? "Reiniciando" : "Reiniciar Discord"}
      </Botao>

      <div className="flex-1" />

      {/* O autostart tem o mesmo peso visual dos botões: é a única coisa desta
          linha que muda o comportamento do PC, e some da vista se for só um
          rótulo cinza solto na direita. */}
      {desabilitado ? (
        // Com o serviço parado não dá para ler a chave `Run`. Um interruptor
        // desligado seria uma resposta inventada; o traço é a resposta certa.
        <span
          className="flex h-9 items-center gap-3 rounded-lg border border-dashed border-borda px-3.5 text-[13.5px] font-medium text-texto2"
          title="Não dá para saber com o serviço parado"
        >
          Iniciar com o PC
          <span className="w-[40px] text-center text-texto3">—</span>
        </span>
      ) : (
        <label
          className="flex h-9 cursor-pointer items-center gap-3 rounded-lg border border-borda bg-superficie pr-2 pl-3.5 shadow-cartao transition-colors duration-150 hover:bg-[#F5F3F0]"
          title={
            autostart
              ? "A correção sobe junto com o sistema"
              : "A correção só vale depois que você abrir o programa"
          }
        >
          <span className="text-[13.5px] font-medium text-texto">
            Iniciar com o PC
          </span>
          <Interruptor
            rotulo="Iniciar com o PC"
            ligado={autostart}
            ocupado={autostartOcupado}
            aoMudar={aoMudarAutostart}
          />
        </label>
      )}
        </>
      )}
    </section>
  );
}
