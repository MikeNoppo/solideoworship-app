Unicode true
SetCompressor /SOLID lzma
!include MUI2.nsh
!include FileFunc.nsh
!include WordFunc.nsh
!include x64.nsh
!define MULTIUSER_MUI
!define MULTIUSER_EXECUTIONLEVEL Highest
!define MULTIUSER_INSTALLMODE_COMMANDLINE
!define MULTIUSER_INSTALLMODE_INSTDIR "SolideoWorship"
!define MULTIUSER_USE_PROGRAMFILES64
!define MULTIUSER_INSTALLMODE_DEFAULT_REGISTRY_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\SolideoWorship"
!define MULTIUSER_INSTALLMODE_DEFAULT_REGISTRY_VALUENAME "CurrentUser"
!define MULTIUSER_INSTALLMODE_FUNCTION RestoreDirectory
!include MultiUser.nsh
Name "SolideoWorship"
OutFile "{{out_file}}"
VIProductVersion "{{version_with_build}}"
VIAddVersionKey "ProductName" "SolideoWorship"
VIAddVersionKey "FileVersion" "{{version}}"
!define MUI_ICON "{{installer_icon}}"
!define MUI_ABORTWARNING
!define MUI_LANGDLL_REGISTRY_ROOT HKCU
!define MUI_LANGDLL_REGISTRY_KEY "Software\MikeNoppo\SolideoWorship"
!define MUI_LANGDLL_REGISTRY_VALUENAME "Installer Language"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "{{license}}"
!insertmacro MULTIUSER_PAGE_INSTALLMODE
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Indonesian"
Var Passive
Function .onInit
  SetRegView 64
  ${GetParameters} $0
  ${GetOptions} $0 "/P" $Passive
  IfErrors +2 0
    SetSilent silent
  !insertmacro MULTIUSER_INIT
  ${IfNot} ${Silent}
    !insertmacro MUI_LANGDLL_DISPLAY
  ${EndIf}
FunctionEnd
Function RestoreDirectory
  ReadRegStr $0 SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\SolideoWorship" "InstallLocation"
  StrCmp $0 "" +2 0
    StrCpy $INSTDIR $0
FunctionEnd
Function un.onInit
  SetRegView 64
  !insertmacro MULTIUSER_UNINIT
FunctionEnd
Section
  ReadRegStr $R0 SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\SolideoWorship" "PackageVersion"
  ${If} $R0 != ""
    ${VersionCompare} "__INSTALL_VERSION__" $R0 $R1
    ${If} $R1 == 2
      SetErrorLevel 1638
      Quit
    ${EndIf}
  ${EndIf}
  InitPluginsDir
  SetOutPath "$PLUGINSDIR\payload"
  File "{{main_binary_path}}"
  {{#each resources}}
  File "/oname={{this}}" "{{@key}}"
  {{/each}}
  {{#each binaries}}
  File "/oname={{this}}" "{{@key}}"
  {{/each}}
  ExecWait '"$PLUGINSDIR\payload\{{main_binary_name}}.exe" --update-probe "{{version}}"' $0
  ${If} $0 != 0
    SetErrorLevel 7
    Quit
  ${EndIf}
  SetOutPath $INSTDIR
  ClearErrors
  CopyFiles /SILENT "$PLUGINSDIR\payload\{{main_binary_name}}.exe" "$INSTDIR\{{main_binary_name}}.exe"
  IfErrors failed
  {{#each resources}}
  CopyFiles /SILENT "$PLUGINSDIR\payload\{{this}}" "$INSTDIR\{{this}}"
  IfErrors failed
  {{/each}}
  {{#each binaries}}
  CopyFiles /SILENT "$PLUGINSDIR\payload\{{this}}" "$INSTDIR\{{this}}"
  IfErrors failed
  {{/each}}
  IfFileExists "$INSTDIR\language" language_done
  FileOpen $0 "$INSTDIR\language" w
  ${If} $LANGUAGE == ${LANG_INDONESIAN}
    FileWrite $0 "id"
  ${Else}
    FileWrite $0 "en"
  ${EndIf}
  FileClose $0
  language_done:
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\SolideoWorship" "DisplayName" "SolideoWorship"
  WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\SolideoWorship" "DisplayVersion" "{{version}}"
  WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\SolideoWorship" "PackageVersion" "__INSTALL_VERSION__"
  WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\SolideoWorship" "InstallLocation" $INSTDIR
  WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\SolideoWorship" "UninstallString" '$\"$INSTDIR\uninstall.exe$\" /$MultiUser.InstallMode'
  WriteRegDWORD SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\SolideoWorship" "$MultiUser.InstallMode" 1
  WriteRegDWORD SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\SolideoWorship" "NoModify" 1
  WriteRegDWORD SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\SolideoWorship" "NoRepair" 1
  CreateShortcut "$SMPROGRAMS\SolideoWorship.lnk" "$INSTDIR\{{main_binary_name}}.exe"
  Goto done
  failed:
  SetErrorLevel 8
  Quit
  done:
SectionEnd
Section Uninstall
  Delete "$INSTDIR\{{main_binary_name}}.exe"
  {{#each resources}}
  Delete "$INSTDIR\{{this}}"
  {{/each}}
  {{#each binaries}}
  Delete "$INSTDIR\{{this}}"
  {{/each}}
  Delete "$INSTDIR\language"
  Delete "$INSTDIR\uninstall.exe"
  Delete "$SMPROGRAMS\SolideoWorship.lnk"
  DeleteRegKey SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\SolideoWorship"
  RMDir "$INSTDIR"
SectionEnd
