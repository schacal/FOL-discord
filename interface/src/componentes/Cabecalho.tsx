import { useState } from "react";
import type { Atualizacao } from "../api";
import { Marca } from "./Marca";
import { Rodela } from "./ui/Botao";

/**
 * 56 px. Arrastável — é por aqui que a janela se move, já que ela não tem
 * barra de título do sistema.
 */
export function Cabecalho({
  versao,
  atualizacao,
  aoAtualizar,
  aoMinimizar,
  aoFechar,
}: {
  versao: string;
  atualizacao: Atualizacao | null;
  aoAtualizar: () => Promise<void>;
  aoMinimizar: () => void;
  aoFechar: () => void;
}) {
  const [baixando, setBaixando] = useState(false);

  async function atualizar() {
    setBaixando(true);
    try {
      await aoAtualizar();
    } finally {
      setBaixando(false);
    }
  }

  return (
    <header
      data-tauri-drag-region
      className="flex h-14 shrink-0 items-center gap-2 border-b border-borda px-5"
    >
      <Marca className="size-6 shrink-0 object-contain" />
      <span data-tauri-drag-region className="text-[14px] font-semibold">
        FOL-discord
      </span>
      <span data-tauri-drag-region className="tabular text-[12.5px] text-texto2">
        v{versao}
      </span>

      {atualizacao && (
        <button
          type="button"
          onClick={atualizar}
          disabled={baixando}
          title={`Abrir o download oficial da v${atualizacao.versao}`}
          className="ml-1 inline-flex h-[22px] items-center gap-1 rounded-full border border-destaque/25 bg-destaque/[0.07] px-2.5 text-[11.5px] font-semibold text-destaque transition-colors duration-150 hover:bg-destaque/[0.13] disabled:cursor-default"
        >
          {baixando ? (
            <>
              <Rodela className="size-3" />
              Abrindo a v{atualizacao.versao}
            </>
          ) : (
            <>
              <SetaCima />v{atualizacao.versao} disponível
            </>
          )}
        </button>
      )}

      <div data-tauri-drag-region className="flex-1" />

      <ControleJanela rotulo="Minimizar" aoClicar={aoMinimizar}>
        <path d="M3 8h10" />
      </ControleJanela>
      <ControleJanela rotulo="Fechar para a bandeja" aoClicar={aoFechar}>
        <path d="M4 4l8 8M12 4l-8 8" />
      </ControleJanela>
    </header>
  );
}

function ControleJanela({
  rotulo,
  aoClicar,
  children,
}: {
  rotulo: string;
  aoClicar: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      title={rotulo}
      aria-label={rotulo}
      onClick={aoClicar}
      className="grid size-7 place-items-center rounded-md text-texto2 transition-colors duration-150 hover:bg-[#F0EDE9] hover:text-texto"
    >
      <svg
        viewBox="0 0 16 16"
        className="size-4"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        aria-hidden
      >
        {children}
      </svg>
    </button>
  );
}

function SetaCima() {
  return (
    <svg
      viewBox="0 0 16 16"
      className="size-3"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d="M8 12.5V4M4.5 7.5 8 4l3.5 3.5" />
    </svg>
  );
}
