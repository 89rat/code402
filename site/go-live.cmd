@echo off
REM ============================================================
REM Code402 go-live: starts Paperclip, hires the fleet, opens dashboard
REM Safe to re-run: onboarding and hiring are idempotent.
REM ============================================================
cd /d %~dp0
echo [Code402] Starting Paperclip server (first run downloads + sets up, 2-3 min)...
start "Paperclip Server - leave open" cmd /k "npx -y paperclipai onboard --yes --no-install-service"

echo [Code402] Waiting for server on 127.0.0.1:3100 ...
:wait
timeout /t 5 /nobreak >nul
powershell -NoProfile -Command "try{Invoke-RestMethod http://127.0.0.1:3100/api/health -TimeoutSec 3|Out-Null;exit 0}catch{exit 1}"
if errorlevel 1 goto wait

echo [Code402] Server is up. Hiring the fleet (39 seats)...
node hire-fleet.mjs

echo [Code402] Opening dashboard...
start http://127.0.0.1:3100
echo.
echo [Code402] LIVE. Keep the "Paperclip Server" window open.
echo Next steps: add your LLM provider key (npx paperclipai configure), then arm heartbeats pod by pod.
pause
