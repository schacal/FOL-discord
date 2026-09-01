import { useState } from "react";
import clsx from "clsx";
import { testes, type Cenario } from "../api/simulado";

/**
 * Não faz parte do produto. Só existe no navegador, com o serviço simulado,
 * para forçar os quatro estados sem depender de proxies reais — que é o que
 * os critérios de aceite pedem para conferir.
 */
const CENARIOS: { valor: Cenario; rotulo: string }[] = [
  { valor: "operacional", rotulo: "Serviço no ar" },
  { valor: "sem_proxies", rotulo: "Piscina seca" },
  { valor: "parado", rotulo: "Serviço caído" },
];

export function BarraDeTeste({ aoMudar }: { aoMudar: () => void }) {
  const [, redesenhar] = useState(0);

  const mexer = (fn: () => void) => () => {
    fn();
    redesenhar((n) => n + 1);
    aoMudar();
  };

  return (
    <div className="flex w-[750px] items-center gap-1.5 overflow-hidden rounded-lg border border-[#C9C4BC] bg-[#EFECE7] px-3 py-2 text-[11px] whitespace-nowrap text-[#57534E]">
      <span className="font-semibold tracking-wide uppercase">Testes</span>
      <span className="text-[#A8A29E]">|</span>

      {CENARIOS.map((c) => (
        <button
          key={c.valor}
          type="button"
          onClick={mexer(() => testes.definirCenario(c.valor))}
          className={clsx(
            "rounded border px-2 py-1 transition-colors duration-150",
            testes.cenario() === c.valor
              ? "border-[#4F46E5] bg-white text-[#4F46E5]"
              : "border-transparent hover:bg-white/70",
          )}
        >
          {c.rotulo}
        </button>
      ))}

      <span className="text-[#A8A29E]">|</span>

      <button
        type="button"
        onClick={mexer(() => testes.alternarAtualizacao())}
        className={clsx(
          "rounded border px-2 py-1 transition-colors duration-150",
          testes.temAtualizacao()
            ? "border-[#4F46E5] bg-white text-[#4F46E5]"
            : "border-transparent hover:bg-white/70",
        )}
      >
        Atualização disponível
      </button>

      <button
        type="button"
        onClick={mexer(() => testes.envelhecerValidacao(73))}
        className="rounded border border-transparent px-2 py-1 transition-colors duration-150 hover:bg-white/70"
      >
        Envelhecer checagem
      </button>

      <span className="flex-1" />
      <span className="shrink-0">Pausado sai no botão Pausar.</span>
    </div>
  );
}
