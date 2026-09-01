import clsx from "clsx";
import type { ButtonHTMLAttributes, ReactNode } from "react";

type Variante = "primario" | "secundario" | "perigo" | "texto";
type Tamanho = "normal" | "grande";

const VARIANTES: Record<Variante, string> = {
  primario:
    "bg-destaque text-white border border-transparent hover:bg-[#4338CA] active:bg-[#3730A3] shadow-cartao",
  secundario:
    "bg-superficie text-texto border border-borda hover:bg-[#F5F3F0] active:bg-[#EFECE8] shadow-cartao",
  perigo:
    "bg-perigo text-white border border-transparent hover:bg-[#B91C1C] active:bg-[#991B1B] shadow-cartao",
  texto:
    "bg-transparent text-texto2 border border-transparent hover:text-texto hover:bg-[#F0EDE9]",
};

const TAMANHOS: Record<Tamanho, string> = {
  normal: "h-9 px-4 text-[13.5px]",
  grande: "h-10 px-5 text-[14px]",
};

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variante?: Variante;
  tamanho?: Tamanho;
  ocupado?: boolean;
  children: ReactNode;
}

export function Botao({
  variante = "secundario",
  tamanho = "normal",
  ocupado = false,
  className,
  children,
  disabled,
  ...resto
}: Props) {
  return (
    <button
      type="button"
      disabled={disabled || ocupado}
      className={clsx(
        "inline-flex items-center justify-center gap-1.5 rounded-lg",
        "font-medium whitespace-nowrap",
        "transition-colors duration-150",
        "disabled:cursor-not-allowed disabled:opacity-45",
        TAMANHOS[tamanho],
        VARIANTES[variante],
        className,
      )}
      {...resto}
    >
      {ocupado && <Rodela />}
      {children}
    </button>
  );
}

/** Indicador de ocupado. Só aparece enquanto algo de verdade está em curso. */
export function Rodela({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 16 16"
      aria-hidden
      className={clsx("girando size-3.5 shrink-0", className)}
    >
      <circle
        cx="8"
        cy="8"
        r="6.25"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeDasharray="28"
        strokeDashoffset="10"
        opacity="0.9"
      />
    </svg>
  );
}
