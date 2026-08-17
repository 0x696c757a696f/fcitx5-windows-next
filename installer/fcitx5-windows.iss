#define ProductName "Fcitx5 for Windows"
#ifndef ProductVersion
  #define ProductVersion "0.1.0"
#endif
#ifndef StageDir
  #error StageDir must be passed to ISCC
#endif
#ifndef ArtifactDir
  #error ArtifactDir must be passed to ISCC
#endif

[Setup]
AppId={{A57DA7F2-9343-4FD4-8D29-CB68B77B82B1}
AppName={#ProductName}
AppVersion={#ProductVersion}
DefaultDirName={autopf}\Fcitx5
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin
OutputDir={#ArtifactDir}
OutputBaseFilename=fcitx5-windows-{#ProductVersion}-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\bin\fcitx5-config.exe
ChangesEnvironment=no
CloseApplications=yes
RestartApplications=no
SetupLogging=yes

[Files]
Source: "{#StageDir}\*"; DestDir: "{app}"; Excludes: "portable.flag"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Fcitx5 Settings"; Filename: "{app}\bin\fcitx5-config.exe"
Name: "{autodesktop}\Fcitx5 Settings"; Filename: "{app}\bin\fcitx5-config.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Run]
Filename: "{app}\bin\fcitx5-register.exe"; Parameters: "--register --dll ""{app}\tsf\x64\fcitx5-tsf.dll"""; Flags: runhidden waituntilterminated
Filename: "{app}\bin\fcitx5-register-x86.exe"; Parameters: "--register --dll ""{app}\tsf\x86\fcitx5-tsf.dll"""; Flags: runhidden waituntilterminated; Check: IsWin64
Filename: "{app}\bin\fcitx5-config.exe"; Description: "Open Fcitx5 settings"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{app}\bin\fcitx5-register-x86.exe"; Parameters: "--unregister --dll ""{app}\tsf\x86\fcitx5-tsf.dll"""; Flags: runhidden waituntilterminated; Check: IsWin64; RunOnceId: "unregister-x86-tsf"
Filename: "{app}\bin\fcitx5-register.exe"; Parameters: "--unregister --dll ""{app}\tsf\x64\fcitx5-tsf.dll"""; Flags: runhidden waituntilterminated; RunOnceId: "unregister-x64-tsf"
