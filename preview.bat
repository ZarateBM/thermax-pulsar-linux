@echo off
title THERMAX 6 PULSAR - Web Preview Server
echo ========================================================
echo   Iniciando servidor de previsualizacion local...
echo   Abriendo en http://127.0.0.1:9876
echo ========================================================
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0preview.ps1"
pause
