@echo off

cargo post build --release

if %errorlevel% neq 0 exit /b %errorlevel%

:choice
set /P c=Do you want to compress the release executable[Y/N]?
if /I "%c%" EQU "Y" goto :compress
if /I "%c%" EQU "N" exit
goto :choice

:compress
upx --best "releases/last/Shared Minecraft Server.exe"
