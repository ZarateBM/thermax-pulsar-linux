# ==============================================================================
# THERMAX PULSAR - Servidor Local de Previsualizacion para Windows
# Permite ver y probar la interfaz web localmente en tiempo real sin publicar.
# No requiere instalar Node, Python ni Rust. Usa PowerShell y .NET nativo.
# ==============================================================================

param(
    [int]$Port = 9876
)

$ErrorActionPreference = "Stop"
$rootDir = Join-Path $PSScriptRoot "src\web\static"

if (-not (Test-Path $rootDir)) {
    Write-Host "Error: No se encontro el directorio $rootDir" -ForegroundColor Red
    exit 1
}

$url = "http://127.0.0.1:$Port/"
$listener = New-Object System.Net.HttpListener
$listener.Prefixes.Add($url)

try {
    $listener.Start()
} catch {
    Write-Host "Error al iniciar el servidor en $url : $_" -ForegroundColor Red
    exit 1
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  THERMAX 6 PULSAR - Servidor de Desarrollo Windows      " -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  Servidor activo en: $url" -ForegroundColor Yellow
Write-Host "  Carpeta estatica:   $rootDir" -ForegroundColor Gray
Write-Host "  Presiona Ctrl+C en esta consola para detener." -ForegroundColor Gray
Write-Host "==========================================================" -ForegroundColor Cyan

# Abrir el navegador automaticamente
Start-Process $url

# Estado simulado en memoria para las pruebas
$mockConfig = @{
    display_mode = "cpu"
    temperature_unit = "C"
    update_interval_ms = 1000
    alternate_interval_secs = 5
    gpu_driver = "nvidia"
    cpu_temp_offset = 0.0
    gpu_temp_offset = 0.0
}

function Get-MimeType($path) {
    $ext = [System.IO.Path]::GetExtension($path).ToLower()
    switch ($ext) {
        ".html" { return "text/html; charset=utf-8" }
        ".css"  { return "text/css; charset=utf-8" }
        ".js"   { return "application/javascript; charset=utf-8" }
        ".json" { return "application/json; charset=utf-8" }
        ".png"  { return "image/png" }
        ".jpg"  { return "image/jpeg" }
        ".jpeg" { return "image/jpeg" }
        ".ico"  { return "image/x-icon" }
        ".svg"  { return "image/svg+xml" }
        ".webp" { return "image/webp" }
        default { return "application/octet-stream" }
    }
}

$rnd = New-Object System.Random

try {
    while ($listener.IsListening) {
        $context = $listener.GetContext()
        $request = $context.Request
        $response = $context.Response

        $path = $request.Url.AbsolutePath
        $method = $request.HttpMethod

        # CORS
        $response.Headers.Add("Access-Control-Allow-Origin", "*")
        $response.Headers.Add("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        $response.Headers.Add("Access-Control-Allow-Headers", "Content-Type")

        if ($method -eq "OPTIONS") {
            $response.StatusCode = 200
            $response.Close()
            continue
        }

        # Rutas de API Mock
        if ($path -eq "/api/metrics") {
            $cpuTemp = 36.0 + ($rnd.Next(-10, 15) / 10.0)
            $gpuTemp = 38.0 + ($rnd.Next(-10, 15) / 10.0)
            $cpuUtil = [Math]::Max(1, [Math]::Min(100, 4 + $rnd.Next(-1, 3)))
            $gpuUtil = [Math]::Max(1, [Math]::Min(100, 4 + $rnd.Next(-1, 3)))
            $cpuFreq = 2294 + $rnd.Next(-40, 50)
            $gpuFreq = 502 + $rnd.Next(-15, 20)
            $cpuFan = 0
            $gpuFan = 0
            $cpuPower = 17 + $rnd.Next(-2, 2)
            $gpuPower = 15 + $rnd.Next(-1, 2)

            $data = @{
                cpu = @{
                    temp = [Math]::Round($cpuTemp, 1)
                    util_percent = $cpuUtil
                    freq_mhz = $cpuFreq
                    fan_rpm = $cpuFan
                    power_watts = $cpuPower
                }
                gpu = @{
                    temp = [Math]::Round($gpuTemp, 1)
                    util_percent = $gpuUtil
                    freq_mhz = $gpuFreq
                    fan_rpm = $gpuFan
                    power_watts = $gpuPower
                }
            }

            $jsonStr = $data | ConvertTo-Json
            $bytes = [System.Text.Encoding]::UTF8.GetBytes($jsonStr)
            $response.ContentType = "application/json; charset=utf-8"
            $response.ContentLength64 = $bytes.Length
            $response.OutputStream.Write($bytes, 0, $bytes.Length)
            $response.Close()
            continue
        }
        elseif ($path -eq "/api/config") {
            if ($method -eq "POST") {
                $reader = New-Object System.IO.StreamReader($request.InputStream, $request.ContentEncoding)
                $body = $reader.ReadToEnd()
                try {
                    $posted = $body | ConvertFrom-Json
                    if ($posted.display_mode) { $mockConfig.display_mode = $posted.display_mode }
                    if ($posted.temperature_unit) { $mockConfig.temperature_unit = $posted.temperature_unit }
                    if ($posted.update_interval_ms) { $mockConfig.update_interval_ms = $posted.update_interval_ms }
                    if ($posted.alternate_interval_secs) { $mockConfig.alternate_interval_secs = $posted.alternate_interval_secs }
                    if ($posted.gpu_driver) { $mockConfig.gpu_driver = $posted.gpu_driver }
                    if ($posted.cpu_temp_offset) { $mockConfig.cpu_temp_offset = $posted.cpu_temp_offset }
                    if ($posted.gpu_temp_offset) { $mockConfig.gpu_temp_offset = $posted.gpu_temp_offset }
                } catch {}

                $resObj = @{ ok = $true; service_restarted = $true }
                $jsonStr = $resObj | ConvertTo-Json
                $bytes = [System.Text.Encoding]::UTF8.GetBytes($jsonStr)
                $response.ContentType = "application/json; charset=utf-8"
                $response.ContentLength64 = $bytes.Length
                $response.OutputStream.Write($bytes, 0, $bytes.Length)
                $response.Close()
                continue
            } else {
                $jsonStr = $mockConfig | ConvertTo-Json
                $bytes = [System.Text.Encoding]::UTF8.GetBytes($jsonStr)
                $response.ContentType = "application/json; charset=utf-8"
                $response.ContentLength64 = $bytes.Length
                $response.OutputStream.Write($bytes, 0, $bytes.Length)
                $response.Close()
                continue
            }
        }
        elseif ($path -eq "/api/service/status") {
            $statusObj = @{
                status = "active"
                detail = "thermax-pulsar.service (Simulado en Windows) - Running"
            }
            $jsonStr = $statusObj | ConvertTo-Json
            $bytes = [System.Text.Encoding]::UTF8.GetBytes($jsonStr)
            $response.ContentType = "application/json; charset=utf-8"
            $response.ContentLength64 = $bytes.Length
            $response.OutputStream.Write($bytes, 0, $bytes.Length)
            $response.Close()
            continue
        }
        elseif ($path -match "^/api/service/(start|stop|restart)$") {
            $resObj = @{ ok = $true }
            $jsonStr = $resObj | ConvertTo-Json
            $bytes = [System.Text.Encoding]::UTF8.GetBytes($jsonStr)
            $response.ContentType = "application/json; charset=utf-8"
            $response.ContentLength64 = $bytes.Length
            $response.OutputStream.Write($bytes, 0, $bytes.Length)
            $response.Close()
            continue
        }
        elseif ($path -eq "/api/service/logs") {
            $logLines = @(
                "[SIMULACION WINDOWS] thermax-pulsar daemon running mock preview",
                "Iniciado correctamente en http://localhost:9876",
                "Lecturas de sensores simuladas para diseno."
            )
            $logsObj = @{ logs = ($logLines -join "`n") }
            $jsonStr = $logsObj | ConvertTo-Json
            $bytes = [System.Text.Encoding]::UTF8.GetBytes($jsonStr)
            $response.ContentType = "application/json; charset=utf-8"
            $response.ContentLength64 = $bytes.Length
            $response.OutputStream.Write($bytes, 0, $bytes.Length)
            $response.Close()
            continue
        }

        # Servir archivos estaticos
        $relPath = $path.TrimStart('/')
        if ($relPath -eq "" -or $relPath -eq "index.html") {
            $relPath = "index.html"
        }
        elseif ($relPath.StartsWith("static/")) {
            $relPath = $relPath.Substring(7)
        }
        elseif ($relPath -eq "favicon.ico") {
            $relPath = "AppLogo.ico"
        }

        $filePath = Join-Path $rootDir $relPath

        if (Test-Path $filePath -PathType Leaf) {
            $bytes = [System.IO.File]::ReadAllBytes($filePath)
            $response.ContentType = Get-MimeType $filePath
            $response.ContentLength64 = $bytes.Length
            $response.OutputStream.Write($bytes, 0, $bytes.Length)
        } else {
            $response.StatusCode = 404
            $msg = [System.Text.Encoding]::UTF8.GetBytes("404 Not Found")
            $response.OutputStream.Write($msg, 0, $msg.Length)
        }
        $response.Close()
    }
} finally {
    $listener.Stop()
    $listener.Close()
}
