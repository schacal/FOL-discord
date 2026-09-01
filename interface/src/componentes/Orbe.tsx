import { ThinkingOrb, type OrbState } from "thinking-orbs";

/**
 * O sinal de estado da janela.
 *
 * Os orbes de `thinking-orbs` são monocromáticos de propósito — tinta escura
 * sobre fundo claro. Aqui cada estado precisa da sua cor, então a tinta é
 * trocada por um filtro SVG: pega o alfa do canvas e o inunda com a cor da
 * paleta. Isso preserva o desbotado de cada ponto, que é o que dá profundidade
 * ao orbe; um `hue-rotate` achataria tudo.
 *
 * Nenhum orbe fica congelado. Um orbe parado é lido como interface travada,
 * não como "o serviço parou" — quem diz o que aconteceu é a palavra e a frase
 * ao lado. O que muda entre os estados é qual animação e a que velocidade.
 */

/**
 * A cor sai do tema, não de uma cópia: `feFlood` recebe a variável por estilo
 * embutido, que é onde `var()` resolve com certeza no WebView2. Assim
 * `estilos.css` continua sendo o único lugar que sabe qual indigo é o indigo.
 */
const TINTAS = ["ok", "atencao", "perigo", "neutro", "destaque"] as const;

type Tinta = (typeof TINTAS)[number];

export interface Sinal {
  orbe: OrbState;
  tinta: Tinta;
  /** Multiplicador sobre a velocidade que o preset já traz. */
  ritmo?: number;
}

export function Orbe({ sinal, className }: { sinal: Sinal; className?: string }) {
  return (
    <ThinkingOrb
      state={sinal.orbe}
      size={64}
      theme="light"
      speed={sinal.ritmo ?? 1}
      // Decorativo: quem diz o estado a um leitor de tela é o título e a frase
      // ao lado. Um rótulo aqui repetiria o `<h1>` — e, durante um aviso, o
      // contradiria ("Sem resposta" ao lado de "Procurando saída").
      aria-hidden
      className={className}
      style={{ filter: `url(#tinta-${sinal.tinta})` }}
    />
  );
}

/**
 * Os filtros vivem uma vez no documento.
 *
 * `sRGB` é obrigatório: sem ele o filtro interpola em linearRGB e a cor sai
 * mais clara que a da paleta. E o alfa passa por uma gama antes da cor: sem
 * ela os pontos mais fracos do orbe somem no off-white e o sinal fica lavado
 * — que é o problema que o orbe resolve sozinho, sem precisar de disco atrás.
 */
export function TintasDeOrbe() {
  return (
    <svg
      aria-hidden
      focusable="false"
      width="0"
      height="0"
      style={{ position: "absolute", pointerEvents: "none" }}
    >
      <defs>
        {TINTAS.map((nome) => (
          <filter
            key={nome}
            id={`tinta-${nome}`}
            x="0"
            y="0"
            width="100%"
            height="100%"
            colorInterpolationFilters="sRGB"
          >
            <feComponentTransfer in="SourceAlpha" result="alfa">
              <feFuncA type="gamma" amplitude="1" exponent="0.34" offset="0" />
            </feComponentTransfer>
            <feFlood style={{ floodColor: `var(--color-${nome})` }} result="cor" />
            <feComposite in="cor" in2="alfa" operator="in" />
          </filter>
        ))}
      </defs>
    </svg>
  );
}
