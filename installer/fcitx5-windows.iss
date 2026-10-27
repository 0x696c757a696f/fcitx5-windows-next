#define ProductName "Fcitx5 for Windows Next"
#ifndef ProductVersion
  #define ProductVersion "0.1.0"
#endif
#ifndef ReleaseChannel
  #define ReleaseChannel "stable"
#endif
#if ReleaseChannel == "beta"
  #define ChannelSuffix " Beta"
  #define DirectorySuffix "-Beta"
  #define ArtifactSuffix "-beta"
  #define InstallerAppId "{{61C768CC-19D8-4314-9ADE-CA5E70A836B2}"
#elif ReleaseChannel == "nightly"
  #define ChannelSuffix " Nightly"
  #define DirectorySuffix "-Nightly"
  #define ArtifactSuffix "-nightly"
  #define InstallerAppId "{{50F78015-B016-4F62-8D37-A376505333C3}"
#else
  #define ChannelSuffix ""
  #define DirectorySuffix ""
  #define ArtifactSuffix ""
  #define InstallerAppId "{{A57DA7F2-9343-4FD4-8D29-CB68B77B82B1}"
#endif
#ifndef StageDir
  #error StageDir must be passed to ISCC
#endif
#ifndef ArtifactDir
  #error ArtifactDir must be passed to ISCC
#endif
#ifndef ReleaseGeneration
  #define ReleaseGeneration "current"
#endif

[Setup]
AppId={#InstallerAppId}
AppName={#ProductName}{#ChannelSuffix}
AppVersion={#ProductVersion}
DefaultDirName={autopf}\Fcitx5{#DirectorySuffix}
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin
OutputDir={#ArtifactDir}
OutputBaseFilename=fcitx5-windows-{#ProductVersion}{#ArtifactSuffix}-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
SetupIconFile=..\resources\icons\fcitx5.ico
UninstallDisplayIcon={app}\bin\fcitx5-config.exe
ChangesEnvironment=no
CloseApplications=no
RestartApplications=no
SetupLogging=yes
#ifdef SignToolName
; Release builds sign the uninstaller as well; an unsigned uninstaller left in
; Program Files is a common source of antivirus warnings.
SignTool={#SignToolName}
SignedUninstaller=yes
#endif

#define RuntimeStageDir StageDir + "\runtime\" + ReleaseGeneration

[Files]
; The top-level bin, lib and share trees are byte-identical copies of the
; runtime generation. Source them from the runtime tree so the compiler stores
; each file only once instead of compressing the payload twice.
Source: "{#StageDir}\*"; DestDir: "{app}"; Excludes: "portable.flag,\bin,\bin\*,\lib,\lib\*,\share,\share\*,tsf\x64\fcitx5-tsf.dll,tsf\x64\fcitx5-tsf.generation,tsf\x86\fcitx5-tsf.dll,tsf\x86\fcitx5-tsf.generation"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#RuntimeStageDir}\bin\*"; DestDir: "{app}\bin"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#RuntimeStageDir}\lib\*"; DestDir: "{app}\lib"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#RuntimeStageDir}\share\*"; DestDir: "{app}\share"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#StageDir}\tsf\x64\fcitx5-tsf.dll"; DestDir: "{tmp}\Fcitx5Tsf\x64"; Flags: deleteafterinstall
Source: "{#StageDir}\tsf\x86\fcitx5-tsf.dll"; DestDir: "{tmp}\Fcitx5Tsf\x86"; Flags: deleteafterinstall

[Icons]
Name: "{group}\Fcitx5 Settings"; Filename: "{app}\bin\fcitx5-config.exe"
Name: "{autodesktop}\Fcitx5 Settings"; Filename: "{app}\bin\fcitx5-config.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Run]
Filename: "{app}\bin\fcitx5-updater.exe"; Parameters: "--install-tsf-dll ""{app}\tsf\x64\fcitx5-tsf.dll"" ""{tmp}\Fcitx5Tsf\x64\fcitx5-tsf.dll"" ""{#ReleaseGeneration}"""; Flags: runhidden waituntilterminated
Filename: "{app}\bin\fcitx5-updater.exe"; Parameters: "--install-tsf-dll ""{app}\tsf\x86\fcitx5-tsf.dll"" ""{tmp}\Fcitx5Tsf\x86\fcitx5-tsf.dll"" ""{#ReleaseGeneration}"""; Flags: runhidden waituntilterminated; Check: IsWin64
Filename: "{app}\bin\fcitx5-register.exe"; Parameters: "--register --dll ""{app}\tsf\x64\fcitx5-tsf.dll"""; Flags: runhidden waituntilterminated
Filename: "{app}\bin\fcitx5-register-x86.exe"; Parameters: "--register --dll ""{app}\tsf\x86\fcitx5-tsf.dll"""; Flags: runhidden waituntilterminated; Check: IsWin64
Filename: "{app}\bin\fcitx5-config.exe"; Description: "Open Fcitx5 settings"; Flags: nowait postinstall skipifsilent runasoriginaluser

[UninstallRun]
Filename: "{app}\bin\fcitx5-register-x86.exe"; Parameters: "--unregister --dll ""{app}\tsf\x86\fcitx5-tsf.dll"""; Flags: runhidden waituntilterminated; Check: IsWin64; RunOnceId: "unregister-x86-tsf"
Filename: "{app}\bin\fcitx5-register.exe"; Parameters: "--unregister --dll ""{app}\tsf\x64\fcitx5-tsf.dll"""; Flags: runhidden waituntilterminated; RunOnceId: "unregister-x64-tsf"

[Code]
var
  RestartFcitxAfterInstall: Boolean;
  UpgradingExistingInstall: Boolean;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Control: String;
  ResultCode: Integer;
begin
  Result := '';
  { An update replaces bin while the user's launcher, engine and UI may still
    run. Stop them first: applications that keep the old TSF DLL loaded then
    follow the new generation instead of talking to a second engine. }
  Control := ExpandConstant('{app}\bin\fcitx5-control.exe');
  UpgradingExistingInstall := FileExists(Control);
  if FileExists(Control) and
     ExecAsOriginalUser(Control, '--shutdown', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) and
     (ResultCode = 0) then
  begin
    RestartFcitxAfterInstall := True;
    Sleep(2000);
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Owner: String;
  ResultCode: Integer;
begin
  if CurStep = ssPostInstall then
  begin
    Owner := ExpandConstant('{param:UPDATEOWNER|builtin}');
    if (Owner <> 'builtin') and (Owner <> 'chocolatey') and (Owner <> 'winget') and
       (Owner <> 'enterprise') and (Owner <> 'manual') then
      RaiseException('Invalid UPDATEOWNER value');
    SaveStringToFile(ExpandConstant('{app}\update-owner.json'),
      '{"format_version":1,"update_owner":"' + Owner + '"}' + #10, False);
    SaveStringToFile(ExpandConstant('{app}\install-ownership.json'),
      '{"format_version":1,"machine_artifacts":"installer","system_registration":"register-helper","per_user_startup":"user-plane","per_user_session":"user-plane","per_user_config":"user-plane"}' + #10,
      False);
    { Start the launcher at sign-in so packaged (UWP) apps, which cannot
      start it themselves, have an engine from the first keystroke. Only a
      fresh install turns this on; an update keeps the user's choice. }
    if not UpgradingExistingInstall then
      ExecAsOriginalUser(ExpandConstant('{app}\bin\fcitx5-control.exe'),
        '--set-startup enabled', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    if RestartFcitxAfterInstall then
      ExecAsOriginalUser(ExpandConstant('{app}\bin\fcitx5-launcher.exe'), '--background', '',
        SW_HIDE, ewNoWait, ResultCode);
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  ResultCode: Integer;
begin
  { Remove the sign-in entry while fcitx5-control still exists. }
  if CurUninstallStep = usUninstall then
    Exec(ExpandConstant('{app}\bin\fcitx5-control.exe'), '--set-startup disabled', '',
      SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;
