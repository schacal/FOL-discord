import logoPrincipal from "../assets/app.png";

/** Logo principal da janela; a bandeja mantém seus próprios ícones por estado. */
export function Marca({ className }: { className?: string }) {
  return (
    <img
      src={logoPrincipal}
      alt=""
      aria-hidden
      draggable={false}
      className={className}
    />
  );
}
