import { useCallback, useEffect, useRef, useState } from "react";
import clsx from "clsx";

import {
  dentroDoTauri,
  modoSimulado,
  servico,
  statusParado,
  type Conexao,
  type Status,
} from "./api";
import { nomeDaRegiao } from "./util/lugares";
import {
  abrirNoNavegador,
  definirEstadoBandeja,
  esconder,
  minimizar,
  ouvirBandeja,
} from "./util/janela";

import { Cabecalho } from "./componentes/Cabecalho";
import { BlocoStatus, type Aviso, type Fase } from "./componentes/BlocoStatus";
import { Metricas } from "./componentes/Metricas";
import { Acoes } from "./componentes/Acoes";
import { Conexoes } from "./componentes/Conexoes";
import { Rodape } from "./componentes/Rodape";
import { DialogoDesinstalar } from "./componentes/DialogoDesinstalar";
import { BarraDeTeste } from "./componentes/BarraDeTeste";
import { TintasDeOrbe } from "./componentes/Orbe";

/** Enquanto o serviço não responde, é a versão que a janela assume. */
const VERSAO_PADRAO = "0.2.6";
const INTERVALO_MS = 2000;
/** Quanto tempo o resultado de uma ação fica no bloco de status. */
const AVISO_MS = 7000;

const SEM_RESPOSTA: Aviso = {
  tom: "atencao",
  titulo: "Sem resposta",
  frase: "O serviço não respondeu. Clique em Verificar agora para religar.",
};

/** Qual das quatro ações está em curso. Só uma por vez, e é assim de propósito. */
type Ocupacao = "pausa" | "verificar" | "reiniciar" | "autostart";

export default function App() {
  const [status, setStatus] = useState<Status>(() => statusParado(VERSAO_PADRAO));
  const [conexoes, setConexoes] = useState<Conexao[]>([]);
  const [carregando, setCarregando] = useState(true);
  const [agora, setAgora] = useState(() => Date.now());

  const [ocupado, setOcupado] = useState<Ocupacao | null>(null);
  const [aviso, setAviso] = useState<Aviso | null>(null);
  const [dialogoAberto, setDialogoAberto] = useState(false);
  const [janela, setJanela] = useState<HTMLElement | null>(null);

  const versaoConhecida = useRef(VERSAO_PADRAO);
  const erroInicializacao = useRef<string | null>(null);
  /**
   * O laço bate a cada 2 s e as ações leem de novo quando terminam, então duas
   * leituras podem estar no ar ao mesmo tempo. Sem este contador, a mais velha
   * chega por último e desfaz na tela o que o usuário acabou de fazer — clicar
   * em Pausar e ver "Funcionando" voltar por dois segundos.
   */
  const leitura = useRef(0);

  // --- leitura -------------------------------------------------------------

  const atualizar = useCallback(async () => {
    const minha = ++leitura.current;
    const atual = () => minha === leitura.current;

    // As duas perguntas são independentes: em série o pior caso é o dobro do
    // tempo limite (5 s) contra um laço de 2 s, e as leituras se empilham.
    const [s, c] = await Promise.allSettled([
      servico.status(),
      servico.conexoes(),
    ]);
    if (!atual()) return;

    if (s.status === "fulfilled") {
      // Versão vazia é serviço mudo, não versão nova: mantém a última conhecida.
      const lido = s.value.versao
        ? s.value
        : { ...s.value, versao: versaoConhecida.current };
      versaoConhecida.current = lido.versao;
      setStatus(lido);
      if (lido.erro_inicializacao !== erroInicializacao.current) {
        erroInicializacao.current = lido.erro_inicializacao;
        if (lido.erro_inicializacao) {
          setAviso({
            tom: "atencao",
            titulo: "Inicialização automática indisponível",
            frase: lido.erro_inicializacao,
          });
        }
      }
      // Conexões são acessório: um erro só nelas não muda o estado da janela.
      setConexoes(c.status === "fulfilled" ? c.value : []);
    } else {
      // Sem resposta na 9252, o serviço não está de pé. É o estado `parado`.
      setStatus(statusParado(versaoConhecida.current));
      setConexoes([]);
    }
    setCarregando(false);
  }, []);

  useEffect(() => {
    let vivo = true;
    const bater = () => {
      if (vivo && !document.hidden) void atualizar();
    };
    bater();
    const id = setInterval(bater, INTERVALO_MS);
    document.addEventListener("visibilitychange", bater);
    return () => {
      vivo = false;
      clearInterval(id);
      document.removeEventListener("visibilitychange", bater);
    };
  }, [atualizar]);

  // "há 12 s" precisa envelhecer sozinho, senão congela na tela. Escondida na
  // bandeja não há o que envelhecer: o relógio para, e volta acertado.
  useEffect(() => {
    const acertar = () => setAgora(Date.now());
    let id = 0;
    const reger = () => {
      clearInterval(id);
      if (document.hidden) return;
      acertar();
      id = setInterval(acertar, 1000);
    };
    reger();
    document.addEventListener("visibilitychange", reger);
    return () => {
      clearInterval(id);
      document.removeEventListener("visibilitychange", reger);
    };
  }, []);

  useEffect(() => {
    if (!aviso) return;
    const id = setTimeout(() => setAviso(null), AVISO_MS);
    return () => clearTimeout(id);
  }, [aviso]);

  // O ícone da bandeja carrega a cor do estado: dá a resposta sem abrir nada.
  useEffect(() => {
    if (carregando) return;
    void definirEstadoBandeja(status.estado, status.estado === "pausado");
  }, [status.estado, carregando]);

  // "Pausar" no menu da bandeja cai aqui, porque quem fala com o serviço é a
  // janela — o processo do Tauri não tem cliente HTTP nenhum.
  const pausaRef = useRef<() => void>(() => {});
  // Depois do render, nunca durante: um render descartado pelo React deixaria
  // aqui um callback preso a um estado que nunca chegou à tela.
  useEffect(() => {
    pausaRef.current = () => void alternarPausa();
  });

  useEffect(() => {
    const parar = ouvirBandeja((acao) => {
      if (acao === "alternar-pausa") pausaRef.current();
    });
    return () => {
      void parar.then((f) => f());
    };
  }, []);

  // --- ações ---------------------------------------------------------------

  /**
   * A receita que toda ação segue.
   *
   * Marca quem está ocupado, tenta, e — dê no que der — relê o serviço antes de
   * liberar o botão: a verdade é o que ele responde, nunca o clique. Uma falha
   * sempre vira uma frase em português no bloco de status; ação que falha calada
   * é o pior defeito que esta janela pode ter.
   */
  async function executar(qual: Ocupacao, fazer: () => Promise<Aviso | null>) {
    setOcupado(qual);
    setAviso(null);
    try {
      setAviso(await fazer());
    } catch {
      setAviso(SEM_RESPOSTA);
    } finally {
      await atualizar();
      setOcupado(null);
    }
  }

  const alternarPausa = () =>
    executar("pausa", async () => {
      if (status.estado === "pausado") await servico.retomar();
      else await servico.pausar();
      return null;
    });

  const verificar = () =>
    executar("verificar", async () => {
      // A ponte já religa quando necessário. Chamar duas vezes poderia criar
      // duas instalações concorrentes na primeira abertura.
      const v = await servico.verificar();
      return {
        tom: v.ok ? "ok" : "atencao",
        titulo: v.ok ? "Verificado" : "Ainda não",
        frase:
          v.ok && v.regiao_detectada
            ? `O Discord está vendo você em ${nomeDaRegiao(
                v.regiao_detectada,
              )}. Tela e câmera liberadas.`
            : v.mensagem,
      };
    });

  const reiniciarDiscord = () =>
    executar("reiniciar", async () =>
      (await servico.reiniciarDiscord())
        ? {
            tom: "ok",
            titulo: "Discord reiniciado",
            frase: "A sessão nova abriu com a correção já valendo.",
          }
        : {
            tom: "atencao",
            titulo: "Discord não encontrado",
            frase: "A correção vale na próxima vez que você abrir o Discord.",
          },
    );

  // A verdade é a chave `Run`, não o clique — e é o `atualizar()` da receita
  // que relê antes de o interruptor voltar a aceitar toque.
  const mudarAutostart = (ligado: boolean) =>
    executar("autostart", async () => {
      await servico.autostart(ligado);
      return null;
    });

  async function aplicarAtualizacao() {
    try {
      const url = await servico.atualizar();
      const abriu = await abrirNoNavegador(url);
      if (!abriu) throw new Error("o navegador não abriu");
      setAviso({
        tom: "ok",
        titulo: "Download aberto",
        frase: "Quando terminar, abra o instalador para concluir a atualização.",
      });
    } catch {
      setAviso({
        tom: "atencao",
        titulo: "Não deu para abrir a atualização",
        frase: "Tente de novo em alguns instantes.",
      });
    }
  }

  async function desinstalar() {
    await servico.desinstalar();
  }

  // --- desenho -------------------------------------------------------------

  const fase: Fase = carregando
    ? "carregando"
    : ocupado === "verificar"
      ? "verificando"
      : "normal";

  return (
    <>
      <TintasDeOrbe />

      <div
        id="janela"
        ref={setJanela}
        className={clsx(
          "relative flex flex-col overflow-hidden rounded-xl border border-borda bg-fundo",
          dentroDoTauri ? "h-full w-full" : "h-[475px] w-[750px] shadow-janela",
        )}
      >
        <Cabecalho
          versao={status.versao}
          atualizacao={status.atualizacao}
          aoAtualizar={aplicarAtualizacao}
          aoMinimizar={() => void minimizar()}
          aoFechar={() => void esconder()}
        />

        <BlocoStatus
          status={status}
          fase={fase}
          aviso={aviso}
          ocupado={ocupado === "pausa"}
          aoAlternarPausa={() => void alternarPausa()}
        />

        <Metricas status={status} agora={agora} />

        <Acoes
          verificando={ocupado === "verificar"}
          reiniciando={ocupado === "reiniciar"}
          autostart={status.autostart}
          autostartOcupado={ocupado === "autostart"}
          desabilitado={status.estado === "parado"}
          aoVerificar={() => void verificar()}
          aoReiniciarDiscord={() => void reiniciarDiscord()}
          aoMudarAutostart={(v) => void mudarAutostart(v)}
        />

        <Conexoes
          conexoes={conexoes}
          agora={agora}
          parado={status.estado === "parado"}
        />

        <Rodape
          aoAbrirRepo={(url) => void abrirNoNavegador(url)}
          aoDesinstalar={() => setDialogoAberto(true)}
        />

        <DialogoDesinstalar
          aberto={dialogoAberto}
          aoFechar={() => setDialogoAberto(false)}
          aoConfirmar={desinstalar}
          container={janela}
        />
      </div>

      {modoSimulado && <BarraDeTeste aoMudar={() => void atualizar()} />}
    </>
  );
}
