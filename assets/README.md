# Recursos visuais

Arquivos usados para apresentar o FOL-discord no GitHub e na interface.

| Arquivo | Uso |
| --- | --- |
| `banner.png` | Capa no topo do README principal. |
| `demo.gif` | Demonstração curta da janela de gerenciamento. |
| `icons/app.png` | Logo principal. É a mesma imagem do cabeçalho da janela **e** a fonte do ícone do programa no Windows. |
| `icons/tray.png` | Versão de 64 × 64 px para a bandeja do Windows, com fundo transparente. |

Os ícones que a aplicação Tauri usa ficam em `interface/src-tauri/icones/`:
`icon.ico` e os PNGs quadrados saem de `icons/app.png` (`npx tauri icon`); só
os `bandeja-*.png` são desenhados por fórmula, porque mudam de cor por estado.
