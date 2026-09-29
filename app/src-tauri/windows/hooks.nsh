; DualBridge installer hooks (included by Tauri's NSIS template).
;
; After installing the app, offer to install the drivers it needs if they are
; missing. The release workflow downloads the driver installers into
; src-tauri/drivers/ so they ship inside the DualBridge installer.
;
; - ViGEmBus (BSD-3-Clause, Nefarius Software Solutions): virtual Xbox
;   controller, needed for games to see the controller.
; - HidHide (Nefarius Software Solutions): optional "exclusive mode" that hides
;   the physical controller from games so they don't see it twice.

!macro DUALBRIDGE_INSTALL_VIGEMBUS
  ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Services\ViGEmBus" "ImagePath"
  ${If} $0 == ""
  ${AndIf} ${FileExists} "$INSTDIR\drivers\ViGEmBus_Setup.exe"
    ${If} $LANGUAGE == 1036
      MessageBox MB_OK|MB_ICONINFORMATION "DualBridge va maintenant installer le pilote ViGEmBus, qui permet aux jeux de voir votre manette comme une manette Xbox. Suivez les étapes de son installateur." /SD IDOK
    ${Else}
      MessageBox MB_OK|MB_ICONINFORMATION "DualBridge will now install the ViGEmBus driver, which lets games see your controller as an Xbox controller. Follow the steps of its installer." /SD IDOK
    ${EndIf}
    ExecWait '"$INSTDIR\drivers\ViGEmBus_Setup.exe"' $1
  ${EndIf}
!macroend

!macro DUALBRIDGE_INSTALL_HIDHIDE
  ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Services\HidHide" "ImagePath"
  ${If} $0 == ""
  ${AndIf} ${FileExists} "$INSTDIR\drivers\HidHide_Setup.exe"
    ${If} $LANGUAGE == 1036
      MessageBox MB_YESNO|MB_ICONQUESTION "Installer aussi HidHide (recommandé) ?$\r$\n$\r$\nIl évite que certains jeux voient votre manette en double. Un redémarrage peut être nécessaire." /SD IDNO IDNO dualbridge_skip_hidhide
    ${Else}
      MessageBox MB_YESNO|MB_ICONQUESTION "Also install HidHide (recommended)?$\r$\n$\r$\nIt stops some games from seeing your controller twice. A restart may be required." /SD IDNO IDNO dualbridge_skip_hidhide
    ${EndIf}
    ExecWait '"$INSTDIR\drivers\HidHide_Setup.exe"' $1
    dualbridge_skip_hidhide:
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro DUALBRIDGE_INSTALL_VIGEMBUS
  !insertmacro DUALBRIDGE_INSTALL_HIDHIDE
!macroend
