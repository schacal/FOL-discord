/**
 * As poucas coisas que só o Tauri sabe fazer.
 *
 * Duas regras valem para o arquivo inteiro, e por isso ficam num lugar só em
 * vez de repetidas em cada função:
 *
 * 1. **Tudo degrada para o navegador**, porque é lá que a janela é desenhada e
 *    testada. Fora do Tauri não há moldura, e não haver moldura não é erro.
 * 2. **Nada aqui explode.** Quem chama solta a promessa (`void minimizar()`),
 *    então uma rejeição vira erro sem dono. E o que está aqui é moldura: a cor
 *    de um ícone na bandeja não pode derrubar a janela que ela emoldura.
 */

import { dentroDoTauri } from "../api";
import { espera } from "./tempo";

async function naMoldura(fazer: () => Promise<unknown>) {
  if (!dentroDoTauri) return;
  try {
    await fazer();
  } catch {
    // A janela continua inteira, e é ela quem responde a pergunta do usuário.
  }
}

async function comandar(nome: string, argumentos?: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke(nome, argumentos);
}

export const minimizar = () =>
  naMoldura(async () => {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  });

/** Fechar esconde para a bandeja. Sair de verdade é só pelo menu da bandeja. */
export const esconder = () =>
  naMoldura(async () => {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().hide();
  });

export const encerrarInterface = () =>
  naMoldura(async () => {
    const { exit } = await import("@tauri-apps/plugin-process");
    await exit(0);
  });

export async function abrirNoNavegador(url: string): Promise<boolean> {
  if (!dentroDoTauri) {
    return Boolean(window.open(url, "_blank", "noopener,noreferrer"));
  }
  try {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
    return true;
  } catch {
    return false;
  }
}

/**
 * Sobe o serviço quando ele não está rodando, e dá a ele o tempo de abrir a
 * porta. É o que faz "Verificar agora" religar de verdade, e não só reclamar.
 *
 * A espera mora aqui, junto do que a exige: quem chama pede "religa", não
 * "religa e conta até um e meio".
 */
export const religarServico = () =>
  naMoldura(async () => {
    await comandar("religar_servico");
    await espera(1500);
  });

/**
 * Avisa a bandeja de que o estado mudou, para o ícone trocar de cor e o item
 * do menu alternar entre Pausar e Retomar.
 */
export const definirEstadoBandeja = (estado: string, pausado: boolean) =>
  naMoldura(() => comandar("definir_estado_bandeja", { estado, pausado }));

/**
 * O menu da bandeja não fala com o serviço — ele pede aqui, porque o cliente
 * HTTP mora num lugar só.
 */
export async function ouvirBandeja(
  aoPedir: (acao: string) => void,
): Promise<() => void> {
  if (!dentroDoTauri) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  return listen<string>("bandeja", (e) => aoPedir(e.payload));
}
