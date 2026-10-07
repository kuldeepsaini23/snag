; Snag installer (Inno Setup 6). Build with installer\build.sh, which passes the version.
; Installs for the current user only (no admin prompt) into %LOCALAPPDATA%\Programs\Snag.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{6F0D2B8E-4C1A-4E7B-9C55-3A1F2E9B7D41}
AppName=Snag
AppVersion={#AppVersion}
AppVerName=Snag {#AppVersion}
AppPublisher=Kuldeep Saini
AppPublisherURL=https://github.com/kuldeepsaini23/snag
AppSupportURL=https://github.com/kuldeepsaini23/snag/issues
AppUpdatesURL=https://github.com/kuldeepsaini23/snag/releases
PrivilegesRequired=lowest
DefaultDirName={autopf}\Snag
DisableProgramGroupPage=yes
DisableDirPage=auto
LicenseFile=..\LICENSE
SetupIconFile=..\crates\app\assets\logo\snag.ico
UninstallDisplayIcon={app}\snag.exe
UninstallDisplayName=Snag
OutputDir=..\target\installer
OutputBaseFilename=Snag-Setup-{#AppVersion}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0

[Tasks]
Name: "desktopicon"; Description: "Put a Snag icon on the desktop"; Flags: unchecked
Name: "autostart"; Description: "Start Snag when I sign in to Windows (it waits quietly in the tray)"; Flags: unchecked

[Files]
Source: "..\target\release\snag.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
; The browser extension, for "Load unpacked" until it is in the stores.
Source: "..\extension\dist\chrome\*"; DestDir: "{app}\browser-extension"; Flags: ignoreversion recursesubdirs

[Icons]
Name: "{autoprograms}\Snag"; Filename: "{app}\snag.exe"
Name: "{autodesktop}\Snag"; Filename: "{app}\snag.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Snag"; ValueData: """{app}\snag.exe"" --background"; Tasks: autostart; Flags: uninsdeletevalue

[Run]
Filename: "{app}\snag.exe"; Description: "Open Snag now"; Flags: nowait postinstall skipifsilent

[Code]
// Asks a running Snag to pause its downloads, save and quit, then waits for it to be gone, so
// its files can be replaced (update) or removed (uninstall).
procedure QuitSnag(Exe: String);
var
  Code, Waited: Integer;
begin
  if not FileExists(Exe) then
    Exit;
  Exec(Exe, '--quit', '', SW_HIDE, ewWaitUntilTerminated, Code);
  Waited := 0;
  // The exe can be replaced once Snag has exited.
  while Waited < 10000 do
  begin
    if RenameFile(Exe, Exe + '.check') then
    begin
      RenameFile(Exe + '.check', Exe);
      Exit;
    end;
    Sleep(250);
    Waited := Waited + 250;
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  QuitSnag(ExpandConstant('{app}\snag.exe'));
  Result := '';
end;

function InitializeUninstall(): Boolean;
begin
  QuitSnag(ExpandConstant('{app}\snag.exe'));
  Result := True;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Data: String;
begin
  if CurUninstallStep <> usPostUninstall then
    Exit;
  Data := ExpandConstant('{userappdata}\Snag');
  if not DirExists(Data) then
    Exit;
  // Downloaded files are never touched: they live in your Downloads folder.
  if SuppressibleMsgBox('Also remove your Snag settings and download history?' + #13#10 + #13#10 +
       'Your downloaded files stay where they are either way.', mbConfirmation, MB_YESNO or MB_DEFBUTTON2, IDNO) = IDYES then
    DelTree(Data, True, True, True);
end;
