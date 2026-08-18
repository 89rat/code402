@echo off
setlocal EnableExtensions
REM Kimi-to-Claude headless dispatcher.
REM Usage: dispatch.cmd ^<taskfile^>   (taskfile may be a full path or a name in collab\inbox)

set "WS=%USERPROFILE%\Documents\kimi\workspace"
set "PROJ=%WS%\code402"
set "COLLAB=%PROJ%\collab"
REM Force the npm global dir onto PATH so this works even from shells
REM that inherited a stale PATH (e.g. a long-running desktop app).
set "PATH=%APPDATA%\npm;%PATH%"

if "%~1"=="" (
  echo Usage: dispatch.cmd ^<taskfile^>
  exit /b 2
)
set "TASK=%~1"
if not exist "%TASK%" set "TASK=%COLLAB%\inbox\%~1"
if not exist "%TASK%" (
  echo Task file not found: %~1
  exit /b 2
)

set "BASE=%~n1"
set "OUT=%COLLAB%\outbox\%BASE%.result.md"
set "LOG=%COLLAB%\logs\%BASE%.log"

cd /d "%PROJ%"

REM Combined prompt = standing rules + the task.
set "PROMPTFILE=%TEMP%\claude-prompt-%BASE%.txt"
copy /y "%COLLAB%\AGENT-BRIEF.md" "%PROMPTFILE%" >nul
echo. >> "%PROMPTFILE%"
echo ---- TASK ---- >> "%PROMPTFILE%"
type "%TASK%" >> "%PROMPTFILE%"

REM One-shot headless run. acceptEdits lets it edit files inside the
REM project without prompting; max-turns caps runtime and cost.
REM If your CLI version rejects these flags, edit this line only.
call claude -p --permission-mode acceptEdits --max-turns 25 < "%PROMPTFILE%" > "%OUT%" 2> "%LOG%"
set "RC=%ERRORLEVEL%"
echo exit_code=%RC% >> "%LOG%"
echo %RC%> "%COLLAB%\outbox\%BASE%.exitcode"

endlocal & exit /b %RC%
