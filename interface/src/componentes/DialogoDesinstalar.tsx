import * as Dialog from "@radix-ui/react-dialog";
import { useState } from "react";
import { Botao } from "./ui/Botao";

/**
 * A única sobreposição da janela inteira.
 *
 * O texto diz o que acontece de verdade — inclusive a parte ruim. Quem
 * desinstala precisa saber que a transmissão de tela pode voltar a falhar, e
 * precisa saber que nenhum arquivo do Discord é tocado.
 */
export function DialogoDesinstalar({
  aberto,
  aoFechar,
  aoConfirmar,
  container,
}: {
  aberto: boolean;
  aoFechar: () => void;
  aoConfirmar: () => Promise<void>;
  container: HTMLElement | null;
}) {
  const [removendo, setRemovendo] = useState(false);
  const [pronto, setPronto] = useState(false);
  const [falhou, setFalhou] = useState(false);

  async function remover() {
    setRemovendo(true);
    setFalhou(false);
    try {
      await aoConfirmar();
      setPronto(true);
    } catch {
      // O aviso do bloco de status fica atrás desta sobreposição, então a falha
      // precisa ser dita aqui. Rodela que para sem explicação, no botão mais
      // destrutivo da janela, é o pior jeito de errar.
      setFalhou(true);
      setRemovendo(false);
    }
  }

  return (
    <Dialog.Root
      open={aberto}
      onOpenChange={(v) => {
        if (!v && !removendo && !pronto) aoFechar();
      }}
    >
      <Dialog.Portal container={container}>
        <Dialog.Overlay className="absolute inset-0 z-40 rounded-xl bg-texto/45" />
        <Dialog.Content
          onOpenAutoFocus={(e) => {
            // Cancelar é quem recebe o foco. Enter não pode remover nada.
            e.preventDefault();
            container
              ?.querySelector<HTMLButtonElement>("[data-foco-cancelar]")
              ?.focus();
          }}
          className="absolute top-1/2 left-1/2 z-50 w-[420px] -translate-x-1/2 -translate-y-1/2 rounded-xl border border-borda bg-superficie p-5 shadow-janela"
        >
          {pronto ? (
            <>
              <Dialog.Title className="text-[16px] font-semibold">
                Removido.
              </Dialog.Title>
              <Dialog.Description className="mt-2 text-[14px] leading-relaxed text-texto2">
                O proxy automático do Windows voltou ao que era antes. Fechando.
              </Dialog.Description>
            </>
          ) : (
            <>
              <Dialog.Title className="text-[16px] font-semibold">
                Remover o FOL-discord?
              </Dialog.Title>
              <Dialog.Description asChild>
                <div className="mt-2 space-y-2 text-[14px] leading-relaxed text-texto2">
                  <p>
                    O proxy automático do Windows volta ao valor que tinha antes, o
                    programa sai do PATH e a pasta é apagada. Nenhum arquivo do
                    Discord é tocado.
                  </p>
                  <p>
                    O Discord será fechado para que a próxima abertura volte a usar
                    sua conexão normal.
                  </p>
                  <p className="text-texto">
                    A transmissão de tela pode voltar a falhar.
                  </p>
                </div>
              </Dialog.Description>

              {falhou && (
                <p className="mt-3 text-[13.5px] text-perigo">
                  O serviço não respondeu. Nada foi removido.
                </p>
              )}

              <div className="mt-5 flex justify-end gap-2">
                <Botao
                  data-foco-cancelar
                  variante="secundario"
                  onClick={aoFechar}
                  disabled={removendo}
                >
                  Cancelar
                </Botao>
                <Botao variante="perigo" ocupado={removendo} onClick={remover}>
                  {removendo ? "Removendo" : "Remover"}
                </Botao>
              </div>
            </>
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
