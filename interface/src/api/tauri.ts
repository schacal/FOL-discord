/**
 * Cliente nativo da janela instalada.
 *
 * O serviço estável não expõe a API HTTP que o protótipo da interface esperava.
 * Dentro do Tauri, portanto, cada ação usa uma ponte Rust que chama os mesmos
 * comandos já validados no PowerShell e que vão embutidos no próprio .exe.
 */

import type { Servico, Status, Conexao, Verificacao } from "./tipos";

async function chamar<T>(comando: string, argumentos?: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(comando, argumentos);
}

export const servicoTauri: Servico = {
  status: () => chamar<Status>("status_servico"),
  conexoes: () => chamar<Conexao[]>("conexoes_servico"),
  pausar: () => chamar<void>("pausar_servico"),
  retomar: () => chamar<void>("retomar_servico"),
  verificar: () => chamar<Verificacao>("verificar_servico"),
  autostart: (ligado) => chamar<void>("definir_autostart", { ligado }),
  reiniciarDiscord: () => chamar<boolean>("reiniciar_discord"),
  atualizar: () => chamar<void>("atualizar_servico"),
  desinstalar: () => chamar<void>("iniciar_desinstalacao"),
};
