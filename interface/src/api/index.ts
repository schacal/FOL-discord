/**
 * Escolhe com quem a janela conversa.
 *
 * Dentro do Tauri, sempre o serviço de verdade. No navegador, o simulado —
 * é assim que se desenha e se testa a janela sem precisar de proxies.
 */

import { servicoSimulado } from "./simulado";
import { servicoTauri } from "./tauri";
import type { Servico } from "./tipos";

export const dentroDoTauri: boolean =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const modoSimulado = !dentroDoTauri;

export const servico: Servico = dentroDoTauri ? servicoTauri : servicoSimulado;

export * from "./tipos";
