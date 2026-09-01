!macro NSIS_HOOK_POSTINSTALL
  FileOpen $0 "$INSTDIR\.fol-discord-instalada" w
  FileWrite $0 "nsis$\r$\n"
  FileClose $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Cada nsExec deixa o código de saída na pilha. Sem o Pop correspondente, o
  ; StrCmp lá embaixo leria o resultado do comando errado.
  nsExec::ExecToLog '"$SYSDIR\taskkill.exe" /F /IM fol-discord-janela.exe'
  Pop $0
  Sleep 800

  ; A tarefa de logon é criada pela janela, não pelo setup, então o
  ; desinstalador do NSIS não a conhece. Deixá-la para trás faria o Windows
  ; tentar abrir um executável apagado a cada login. schtasks devolve erro
  ; quando a tarefa não existe — e isso aqui não é motivo para abortar nada.
  nsExec::ExecToLog '"$SYSDIR\schtasks.exe" /delete /tn "FolDiscord.Bandeja" /f'
  Pop $0

  nsExec::ExecToLog '"$INSTDIR\fol-discord.exe" desinstalar --manter-arquivos'
  Pop $0
  StrCmp $0 "0" cleanup_ok cleanup_failed
cleanup_failed:
  MessageBox MB_ICONSTOP "Não foi possível restaurar as configurações do FOL-discord. A desinstalação foi cancelada para não deixar a correção pela metade."
  Abort
cleanup_ok:
!macroend
