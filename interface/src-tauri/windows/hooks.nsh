!macro NSIS_HOOK_POSTINSTALL
  FileOpen $0 "$INSTDIR\.fol-discord-instalada" w
  FileWrite $0 "nsis$\r$\n"
  FileClose $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; O macro padrão verifica a janela antes da limpeza. Se a pessoa cancelar,
  ; o backup do PAC e o autostart ainda não foram tocados.
  !insertmacro CheckIfAppIsRunning "fol-discord-janela.exe" "FOL-discord"

  ; O serviço é o único dono da limpeza: restaura o PAC, valida e remove o
  ; autostart e fecha o Discord sem duplicar comandos externos no setup.
  nsExec::ExecToLog '"$INSTDIR\fol-discord.exe" desinstalar --manter-arquivos'
  Pop $0
  StrCmp $0 "0" cleanup_ok cleanup_failed
cleanup_failed:
  MessageBox MB_ICONSTOP "Não foi possível restaurar as configurações do FOL-discord. A desinstalação foi cancelada para não deixar a correção pela metade."
  Abort
cleanup_ok:
!macroend
