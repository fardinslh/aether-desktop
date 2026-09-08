!macro NSIS_HOOK_POSTINSTALL
  ; 1. Extract bundled binaries if present at build time
  !if /FileExists "E:\MyProjects\aether-desktop\runtime\aether.exe"
    SetOutPath "$INSTDIR"
    File /oname=aether.exe "E:\MyProjects\aether-desktop\runtime\aether.exe"
  !else if /FileExists "..\..\..\..\..\runtime\aether.exe"
    SetOutPath "$INSTDIR"
    File /oname=aether.exe "..\..\..\..\..\runtime\aether.exe"
  !endif

  !if /FileExists "E:\MyProjects\aether-desktop\runtime\sing-box.exe"
    SetOutPath "$INSTDIR"
    File /oname=sing-box.exe "E:\MyProjects\aether-desktop\runtime\sing-box.exe"
  !else if /FileExists "..\..\..\..\..\runtime\sing-box.exe"
    SetOutPath "$INSTDIR"
    File /oname=sing-box.exe "..\..\..\..\..\runtime\sing-box.exe"
  !endif

  !if /FileExists "E:\MyProjects\aether-desktop\runtime\libcronet.dll"
    SetOutPath "$INSTDIR"
    File /oname=libcronet.dll "E:\MyProjects\aether-desktop\runtime\libcronet.dll"
  !else if /FileExists "..\..\..\..\..\runtime\libcronet.dll"
    SetOutPath "$INSTDIR"
    File /oname=libcronet.dll "..\..\..\..\..\runtime\libcronet.dll"
  !endif

  ; 2. Runtime fallback: verify dependencies exist in $INSTDIR; if missing, run download script
  IfFileExists "$INSTDIR\aether.exe" check_sb 0
    Goto run_fallback
  check_sb:
  IfFileExists "$INSTDIR\sing-box.exe" done_deps 0

  run_fallback:
    DetailPrint "Core dependencies missing. Checking and downloading from GitHub..."
    !if /FileExists "E:\MyProjects\aether-desktop\src-tauri\windows\download-dependencies.ps1"
      File /oname=download-dependencies.ps1 "E:\MyProjects\aether-desktop\src-tauri\windows\download-dependencies.ps1"
      nsExec::ExecToLog 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\download-dependencies.ps1" -InstallDir "$INSTDIR"'
      Delete "$INSTDIR\download-dependencies.ps1"
    !endif

  done_deps:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\aether.exe"
  Delete "$INSTDIR\sing-box.exe"
  Delete "$INSTDIR\libcronet.dll"
  Delete "$INSTDIR\libunwind.dll"
  Delete "$INSTDIR\wintun.dll"
!macroend
