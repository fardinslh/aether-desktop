!macro NSIS_HOOK_POSTINSTALL
  ; 1. Extract bundled binaries if present at build time
  !if /FileExists "..\..\..\..\..\runtime\aether.exe"
    SetOutPath "$INSTDIR"
    File /oname=aether.exe "..\..\..\..\..\runtime\aether.exe"
  !else if /FileExists "..\..\..\..\..\..\runtime\aether.exe"
    SetOutPath "$INSTDIR"
    File /oname=aether.exe "..\..\..\..\..\..\runtime\aether.exe"
  !endif

  !if /FileExists "..\..\..\..\..\runtime\sing-box.exe"
    SetOutPath "$INSTDIR"
    File /oname=sing-box.exe "..\..\..\..\..\runtime\sing-box.exe"
  !else if /FileExists "..\..\..\..\..\..\runtime\sing-box.exe"
    SetOutPath "$INSTDIR"
    File /oname=sing-box.exe "..\..\..\..\..\..\runtime\sing-box.exe"
  !endif

  !if /FileExists "..\..\..\..\..\runtime\libcronet.dll"
    SetOutPath "$INSTDIR"
    File /oname=libcronet.dll "..\..\..\..\..\runtime\libcronet.dll"
  !else if /FileExists "..\..\..\..\..\..\runtime\libcronet.dll"
    SetOutPath "$INSTDIR"
    File /oname=libcronet.dll "..\..\..\..\..\..\runtime\libcronet.dll"
  !endif

  ; 1b. Runtime DLLs required by the gnullvm-built application binary.
  ;     libunwind.dll: Rust x86_64-pc-windows-gnullvm runtime dependency.
  ;     WebView2Loader.dll: dynamically loaded by wry on non-MSVC toolchains.
  ;     Without these the installed app dies at process start with
  ;     "DLL not found" on any machine without a Rust/LLVM-MinGW dev PATH.
  !if /FileExists "..\..\libunwind.dll"
    SetOutPath "$INSTDIR"
    File /oname=libunwind.dll "..\..\libunwind.dll"
  !else if /FileExists "..\..\..\libunwind.dll"
    SetOutPath "$INSTDIR"
    File /oname=libunwind.dll "..\..\..\libunwind.dll"
  !else
    !error "libunwind.dll was not found in release target directory."
  !endif

  !if /FileExists "..\..\WebView2Loader.dll"
    SetOutPath "$INSTDIR"
    File /oname=WebView2Loader.dll "..\..\WebView2Loader.dll"
  !else if /FileExists "..\..\..\WebView2Loader.dll"
    SetOutPath "$INSTDIR"
    File /oname=WebView2Loader.dll "..\..\..\WebView2Loader.dll"
  !else
    !error "WebView2Loader.dll was not found in release target directory."
  !endif

  IfFileExists "$INSTDIR\libunwind.dll" check_wv2 0
    DetailPrint "FATAL: libunwind.dll was not staged into the installer."
    MessageBox MB_ICONSTOP "Installer is incomplete: libunwind.dll is missing. The application would not start. Please report this build."
    Abort
  check_wv2:
  IfFileExists "$INSTDIR\WebView2Loader.dll" done_rt 0
    DetailPrint "FATAL: WebView2Loader.dll was not staged into the installer."
    MessageBox MB_ICONSTOP "Installer is incomplete: WebView2Loader.dll is missing. The application would not start. Please report this build."
    Abort
  done_rt:

  ; 2. Runtime fallback: verify dependencies exist in $INSTDIR; if missing, run download script
  IfFileExists "$INSTDIR\aether.exe" check_sb 0
    Goto run_fallback
  check_sb:
  IfFileExists "$INSTDIR\sing-box.exe" done_deps 0

  run_fallback:
    DetailPrint "Core dependencies missing. Checking and downloading from GitHub..."
    !if /FileExists "..\..\..\..\windows\download-dependencies.ps1"
      File /oname=download-dependencies.ps1 "..\..\..\..\windows\download-dependencies.ps1"
      nsExec::ExecToLog 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\download-dependencies.ps1" -InstallDir "$INSTDIR"'
      Delete "$INSTDIR\download-dependencies.ps1"
    !else if /FileExists "..\..\..\..\..\windows\download-dependencies.ps1"
      File /oname=download-dependencies.ps1 "..\..\..\..\..\windows\download-dependencies.ps1"
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
  Delete "$INSTDIR\WebView2Loader.dll"
  Delete "$INSTDIR\wintun.dll"
!macroend
