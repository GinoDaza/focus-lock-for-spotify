; Hooks for the Tauri NSIS installer (per-machine, elevated).
; The service registers and removes itself with its own `install` and
; `uninstall` commands. `uninstall` refuses while a locked session runs,
; which stops both updates and uninstalls until the session ends.
; Spicetify and the extension are set up per user by the app (ADR 0007).
; Hooks run inside the Tauri template, which defines $UpdateMode and
; $DeleteAppDataCheckboxState.

!macro NSIS_HOOK_PREINSTALL
  ; Updating: stop and remove the old service so its files can be replaced.
  IfFileExists "$INSTDIR\tunelock-service.exe" 0 tunelock_preinstall_done
    nsExec::ExecToLog '"$INSTDIR\tunelock-service.exe" uninstall'
    Pop $0
    StrCmp $0 "0" tunelock_preinstall_done
      MessageBox MB_ICONSTOP "Tunelock can't be updated right now. If a locked session is running, try again after it ends." /SD IDOK
      Abort
  tunelock_preinstall_done:
!macroend

!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog '"$INSTDIR\tunelock-service.exe" install'
  Pop $0
  StrCmp $0 "0" tunelock_postinstall_done
    MessageBox MB_ICONEXCLAMATION "The Tunelock service couldn't be started (code $0). Run the installer again to repair it." /SD IDOK
  tunelock_postinstall_done:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog '"$INSTDIR\tunelock-service.exe" uninstall'
  Pop $0
  StrCmp $0 "0" tunelock_preuninstall_done
    MessageBox MB_ICONSTOP "Tunelock can't be uninstalled right now. If a locked session is running, try again after it ends." /SD IDOK
    Abort
  tunelock_preuninstall_done:
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; "Delete the application data" also removes the service's data
  ; (allowlist, presets, schedules, log) in C:\ProgramData\Tunelock.
  ; Reaching this point means no locked session was running.
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext all
    RMDir /r "$APPDATA\Tunelock"
    SetShellVarContext current
  ${EndIf}
!macroend
