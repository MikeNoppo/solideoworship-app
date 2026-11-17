#ifndef AppVersion
  #error Pass the version with /DAppVersion=x.y.z
#endif

#ifndef FFmpegBinDir
  #define FFmpegBinDir GetEnv("FFMPEG_BIN_DIR")
#endif

#if FFmpegBinDir == ""
  #error Set FFMPEG_BIN_DIR to the directory containing the packaged FFmpeg DLLs
#endif

#define AppName "SolideoWorship"
#define AppExe "SolideoWorship.exe"
#define AppUrl "https://github.com/MikeNoppo/solideoworship-app"

[Setup]
AppId={{C13C165B-D0C6-4EA6-99B3-F8208FF4BAC0}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=MikeNoppo
AppPublisherURL={#AppUrl}
AppSupportURL={#AppUrl}/issues
AppUpdatesURL={#AppUrl}/releases
DefaultDirName={autopf}\{#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
SetupArchitecture=x64
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
WizardStyle=modern dynamic
WizardImageFile=..\..\assets\logo\png\installer-wizard-202x386.png,..\..\assets\logo\png\installer-wizard-404x772.png
WizardSmallImageFile=..\..\assets\logo\png\app-icon-64.png,..\..\assets\logo\png\app-icon-128.png
SetupIconFile=..\..\assets\logo\solideoworship.ico
Compression=lzma2/max
SolidCompression=yes
CloseApplications=force
RestartApplications=no
SetupMutex=SolideoWorshipSetup,Global\SolideoWorshipSetup
OutputDir=..\..\target\installer
OutputBaseFilename={#AppName}-{#AppVersion}-windows-x64-setup
UninstallDisplayIcon={app}\{#AppExe}

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"; LicenseFile: "privacy-en.txt"
Name: "id"; MessagesFile: "Indonesian.isl"; LicenseFile: "privacy-id.txt"

[Messages]
en.WizardLicense=Privacy Policy & License Agreement
en.LicenseLabel3=Please read the following Privacy Policy and License Agreement. You must accept the terms before continuing with the installation.
id.WizardLicense=Kebijakan Privasi & Kesepakatan Lisensi
id.LicenseLabel3=Silakan baca Kebijakan Privasi dan Kesepakatan Lisensi berikut. Anda harus menyetujuinya sebelum meneruskan instalasi.

[CustomMessages]
en.UpdatePreflightFailed=The updated application could not start. Your previous installation is still available.
id.UpdatePreflightFailed=Aplikasi baru tidak dapat dijalankan. Instalasi sebelumnya masih tersedia.

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\..\target\release\sdw-app.exe"; DestDir: "{app}"; DestName: "{#AppExe}"; Flags: ignoreversion
Source: "..\..\target\release\sdw-updater.exe"; DestDir: "{app}"; DestName: "SolideoWorshipUpdater.exe"; Flags: ignoreversion
Source: "{#FFmpegBinDir}\*.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#FFmpegBinDir}\..\LICENSE.txt"; DestDir: "{app}"; DestName: "LICENSE-FFmpeg.txt"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "..\..\vendor\ac-ffmpeg\LICENSE"; DestDir: "{app}"; DestName: "LICENSE-ac-ffmpeg.txt"; Flags: ignoreversion
Source: "..\..\PRIVACY.md"; DestDir: "{app}"; DestName: "PRIVACY.txt"; Flags: ignoreversion
Source: "..\..\assets\fonts\Inter-LICENSE.txt"; DestDir: "{app}"; DestName: "LICENSE-Inter.txt"; Flags: ignoreversion
Source: "..\..\assets\fonts\JetBrainsMono-LICENSE.txt"; DestDir: "{app}"; DestName: "LICENSE-JetBrainsMono.txt"; Flags: ignoreversion
Source: "..\..\assets\fonts\Lucide-LICENSE.txt"; DestDir: "{app}"; DestName: "LICENSE-Lucide.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: files; Name: "{app}\language"

[Code]
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ProbeDir: String;
  ProbeExitCode: Integer;
begin
  Result := '';
  if ExpandConstant('{param:SDWUpdate|0}') <> '1' then
    Exit;

  try
    ExtractTemporaryFiles('{app}\{#AppExe}');
    ExtractTemporaryFiles('{app}\*.dll');
    ProbeDir := ExpandConstant('{tmp}\') + '{app}';
    if not ExecAsOriginalUser(ProbeDir + '\{#AppExe}', '--update-probe "{#AppVersion}"',
      ProbeDir, SW_HIDE, ewWaitUntilTerminated, ProbeExitCode) or (ProbeExitCode <> 0) then
    begin
      Log(Format('Update preflight exit code: %d', [ProbeExitCode]));
      Result := CustomMessage('UpdatePreflightFailed');
    end;
  except
    Log(GetExceptionMessage);
    Result := CustomMessage('UpdatePreflightFailed');
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    SaveStringToFile(ExpandConstant('{app}\language'), ActiveLanguage, False);
end;
