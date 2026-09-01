!macro NSIS_HOOK_POSTINSTALL
  FileOpen $0 "$INSTDIR\.fol-discord-instalada" w
  FileWrite $0 "nsis$\r$\n"
  FileClose $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog '"$SYSDIR\taskkill.exe" /F /IM fol-discord-janela.exe'
  Sleep 800
  nsExec::ExecToLog '"$INSTDIR\fol-discord.exe" desinstalar --manter-arquivos'
  Pop $0
  StrCmp $0 "0" cleanup_ok cleanup_failed
cleanup_failed:
  MessageBox MB_ICONSTOP "Não foi possível restaurar as configurações do FOL-discord. A desinstalação foi cancelada para não deixar a correção pela metade."
  Abort
cleanup_ok:
!macroend
