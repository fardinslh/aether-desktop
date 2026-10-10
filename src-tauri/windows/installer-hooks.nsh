; Tauri stages the complete verified runtime through bundle.resources.
; Native MSVC builds do not need the gnullvm loader DLLs below.
!macro NSIS_HOOK_POSTINSTALL
  !if /FileExists "..\..\libunwind.dll"
    SetOutPath "$INSTDIR"
    File /oname=libunwind.dll "..\..\libunwind.dll"
  !else if /FileExists "..\..\..\libunwind.dll"
    SetOutPath "$INSTDIR"
    File /oname=libunwind.dll "..\..\..\libunwind.dll"
  !endif
  !if /FileExists "..\..\WebView2Loader.dll"
    SetOutPath "$INSTDIR"
    File /oname=WebView2Loader.dll "..\..\WebView2Loader.dll"
  !else if /FileExists "..\..\..\WebView2Loader.dll"
    SetOutPath "$INSTDIR"
    File /oname=WebView2Loader.dll "..\..\..\WebView2Loader.dll"
  !endif

  IfFileExists "$INSTDIR\runtime\aether.exe" check_sb 0
    Goto runtime_missing
  check_sb:
  IfFileExists "$INSTDIR\runtime\sing-box.exe" check_manifest 0
    Goto runtime_missing
  check_manifest:
  IfFileExists "$INSTDIR\runtime\manifest.json" check_psiphon 0
    Goto runtime_missing
  check_psiphon:
  IfFileExists "$INSTDIR\runtime\pt\psiphon-tunnel-core.exe" check_lyrebird 0
    Goto runtime_missing
  check_lyrebird:
  IfFileExists "$INSTDIR\runtime\pt\lyrebird.exe" done_runtime 0
  runtime_missing:
    MessageBox MB_ICONSTOP "The bundled VPN runtime is incomplete. Reinstall Aether Desktop from its official release package."
    Abort
  done_runtime:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Remove legacy 0.1.5 files; Tauri removes the new runtime resource tree.
  Delete "$INSTDIR\aether.exe"
  Delete "$INSTDIR\sing-box.exe"
  Delete "$INSTDIR\libcronet.dll"
  Delete "$INSTDIR\libunwind.dll"
  Delete "$INSTDIR\WebView2Loader.dll"
  Delete "$INSTDIR\wintun.dll"
!macroend
