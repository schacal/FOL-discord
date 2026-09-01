import * as Switch from "@radix-ui/react-switch";
import clsx from "clsx";

/**
 * O estado vem sempre do serviço, nunca de dentro da interface: a chave `Run`
 * pode ser mexida por fora e a janela precisa refletir isso.
 */
export function Interruptor({
  ligado,
  aoMudar,
  ocupado = false,
  rotulo,
}: {
  ligado: boolean;
  aoMudar: (v: boolean) => void;
  ocupado?: boolean;
  rotulo: string;
}) {
  return (
    <Switch.Root
      checked={ligado}
      onCheckedChange={aoMudar}
      disabled={ocupado}
      aria-label={rotulo}
      className={clsx(
        "relative h-[22px] w-[40px] shrink-0 rounded-full border transition-colors duration-150",
        "disabled:cursor-not-allowed disabled:opacity-50",
        ligado
          ? "border-destaque bg-destaque"
          : "border-[#D6D1CA] bg-[#DEDAD4] hover:bg-[#D2CDC5]",
      )}
    >
      <Switch.Thumb
        className={clsx(
          "block size-[18px] rounded-full bg-white shadow-sm",
          "transition-transform duration-150 will-change-transform",
          "translate-x-[1px] data-[state=checked]:translate-x-[19px]",
        )}
      />
    </Switch.Root>
  );
}
