; Instalador de Windows (Inno Setup 6). Se corre después de empacar-windows.ps1:
;   ISCC /DVersion=0.6.0 empaque\instalador.iss
#ifndef Version
  #define Version "0.0.0"
#endif
[Setup]
AppId={{7C1F0F0E-6E0B-4B35-9C0B-BEE7C0A7F0C6}
AppName=Beethoven Craft
AppVersion={#Version}
AppPublisher=mixter-ha99
AppPublisherURL=https://github.com/Ismael-kaiser/beethoven-craft
DefaultDirName={autopf}\Beethoven Craft
DefaultGroupName=Beethoven Craft
DisableProgramGroupPage=yes
; Sin permisos de administrador: se instala para el usuario.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=..\LICENSE
SetupIconFile=icono.ico
UninstallDisplayIcon={app}\icono.ico
OutputDir=..\dist
OutputBaseFilename=BeethovenCraft-{#Version}-instalador
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
[Languages]
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"
[Tasks]
Name: "escritorio"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
[Files]
Source: "..\dist\Beethoven Craft\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion
Source: "icono.ico"; DestDir: "{app}"
[Icons]
Name: "{group}\Beethoven Craft"; Filename: "{app}\beethoven-craft.exe"; IconFilename: "{app}\icono.ico"
Name: "{autodesktop}\Beethoven Craft"; Filename: "{app}\beethoven-craft.exe"; IconFilename: "{app}\icono.ico"; Tasks: escritorio
[Run]
Filename: "{app}\beethoven-craft.exe"; Description: "{cm:LaunchProgram,Beethoven Craft}"; Flags: nowait postinstall skipifsilent
