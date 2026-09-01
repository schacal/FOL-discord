/**
 * Tempo relativo em português, curto o bastante para caber numa coluna.
 *
 * Nenhum horário absoluto aparece na janela: ninguém quer converter UTC de
 * cabeça para saber se a última checagem foi recente.
 */

export function haQuantoTempo(
  iso: string | number | null | undefined,
  agora = Date.now(),
): string {
  if (!iso) return "—";
  const t = typeof iso === "number" ? iso : Date.parse(iso);
  if (Number.isNaN(t)) return "—";

  // Sempre para baixo: "há 2 h" com 90 minutos de idade envelhece a checagem
  // na frente de quem está justamente conferindo se ela foi recente.
  const s = Math.max(0, Math.floor((agora - t) / 1000));
  if (s < 5) return "agora";
  if (s < 60) return `há ${s} s`;

  const min = Math.floor(s / 60);
  if (min < 60) return `há ${min} min`;

  const h = Math.floor(min / 60);
  if (h < 24) return `há ${h} h`;

  return `há ${Math.floor(h / 24)} d`;
}

/** Uma pausa. Usada tanto pelo produto quanto pelo serviço simulado. */
export const espera = (ms: number) =>
  new Promise<void>((pronto) => setTimeout(pronto, ms));
