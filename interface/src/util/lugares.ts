/**
 * O nome de um lugar, em português, para os dois jeitos de perguntar.
 *
 * O Discord nomeia a mesma coisa de duas formas: a **região** que a sonda
 * devolve (`rotterdam`, `us-east`) e o **código de aeroporto** dentro do host
 * do servidor de voz (`c-gru18-…`). As duas viram texto na mesma tela, a 200 px
 * de distância — "Sua sessão está saindo por Rotterdam" no bloco de status e
 * "Servidor de voz — São Paulo" na lista de atividade.
 *
 * Por isso as duas tabelas moram no mesmo arquivo: corrigir uma grafia num lado
 * e esquecer o outro não quebra nada, não acende teste nenhum, e produz dois
 * nomes para o mesmo lugar na mesma janela.
 */

/** Código de aeroporto → cidade. Alimenta os hosts `c-<cidade><n>-<hash>`. */
export const CIDADES: Record<string, string> = {
  gru: "São Paulo",
  gig: "Rio de Janeiro",
  scl: "Santiago",
  eze: "Buenos Aires",
  bog: "Bogotá",
  mia: "Miami",
  atl: "Atlanta",
  iad: "Virgínia",
  dca: "Virgínia",
  jfk: "Nova York",
  ewr: "Nova York",
  ord: "Chicago",
  dfw: "Dallas",
  sea: "Seattle",
  sfo: "Califórnia",
  sjc: "Califórnia",
  yyz: "Toronto",
  ams: "Amsterdã",
  rtm: "Rotterdam",
  fra: "Frankfurt",
  lhr: "Londres",
  cdg: "Paris",
  mad: "Madri",
  mil: "Milão",
  waw: "Varsóvia",
  arn: "Estocolmo",
  hel: "Finlândia",
  otp: "Bucareste",
  dxb: "Dubai",
  bom: "Mumbai",
  sin: "Singapura",
  hkg: "Hong Kong",
  nrt: "Tóquio",
  icn: "Seul",
  syd: "Sydney",
  jnb: "Joanesburgo",
};

/**
 * Região devolvida pela sonda → nome próprio. Elas vêm em minúsculas e com
 * hífen: `us-east`, `rotterdam`.
 */
const NOMES: Record<string, string> = {
  brazil: "Brasil",
  rotterdam: "Rotterdam",
  frankfurt: "Frankfurt",
  madrid: "Madri",
  milan: "Milão",
  stockholm: "Estocolmo",
  warsaw: "Varsóvia",
  bucharest: "Bucareste",
  finland: "Finlândia",
  russia: "Rússia",
  india: "Índia",
  japan: "Japão",
  singapore: "Singapura",
  sydney: "Sydney",
  dubai: "Dubai",
  "south-korea": "Coreia do Sul",
  "south-africa": "África do Sul",
  "hong-kong": "Hong Kong",
  "buenos-aires": "Buenos Aires",
  santiago: "Santiago",
  "us-east": "Leste dos EUA",
  "us-west": "Oeste dos EUA",
  "us-south": "Sul dos EUA",
  "us-central": "Centro dos EUA",
  atlanta: "Atlanta",
  newark: "Newark",
  oregon: "Oregon",
  "santa-clara": "Santa Clara",
};

export function nomeDaRegiao(regiao: string | null | undefined): string {
  if (!regiao) return "—";
  const chave = regiao.toLowerCase();
  if (NOMES[chave]) return NOMES[chave];
  return chave
    .split("-")
    .map((p) => p.charAt(0).toUpperCase() + p.slice(1))
    .join(" ");
}
