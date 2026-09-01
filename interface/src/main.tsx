import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { dentroDoTauri } from "./api";
import "./estilos.css";

// Dentro do Tauri a janela é a própria moldura; no navegador ela é desenhada
// no tamanho real sobre um fundo neutro.
if (dentroDoTauri) document.documentElement.classList.add("tauri");

createRoot(document.getElementById("raiz")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
