; PastaHandler installer — compile with: iscc /DAppVersion=x.y.z pastahandler.iss
; Produces Output\pastahandler-setup.exe (the name the evergreen release link serves).
; CI passes AppVersion from the git tag; a hand build without it is marked dev.
#ifndef AppVersion
  #define AppVersion "0.0.0-dev"
#endif

[Setup]
AppId={{A3F6B2D1-58C4-4E9A-9B7F-2D1C6E8A4F30}
AppName=PastaHandler
AppVersion={#AppVersion}
AppPublisher=Kwuasimoto
AppPublisherURL=https://github.com/Kwuasimoto/PastaHandler
DefaultDirName={autopf}\PastaHandler
DisableProgramGroupPage=yes
; per-user install: no UAC prompt, {autopf} resolves to %LOCALAPPDATA%\Programs
PrivilegesRequired=lowest
OutputBaseFilename=pastahandler-setup
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\pastahandler.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Files]
Source: "..\target\release\pastahandler.exe"; DestDir: "{app}"

[Tasks]
Name: "startup"; Description: "Start PastaHandler automatically when Windows starts"; Flags: unchecked

[Icons]
; Two front doors, one exe: the tray resident and the settings window
Name: "{autoprograms}\PastaHandler"; Filename: "{app}\pastahandler.exe"
Name: "{autoprograms}\PastaHandler Settings"; Filename: "{app}\pastahandler.exe"; Parameters: "--settings"
Name: "{userstartup}\PastaHandler"; Filename: "{app}\pastahandler.exe"; Tasks: startup

[Run]
Filename: "{app}\pastahandler.exe"; Description: "Launch PastaHandler now"; Flags: nowait postinstall skipifsilent
