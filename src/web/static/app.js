/* ═══════════════════════════════════════════════════════════════
   THERMAX 6 PULSAR — Hardware Monitor Frontend Controller
   ═══════════════════════════════════════════════════════════════ */

(function () {
    'use strict';

    // ─── Constants ───
    const API_BASE = '';
    const TOTAL_CIRCUMFERENCE = 515.2; // 2 * PI * 82
    const MAX_ARC_LENGTH = 386.4;      // 270 degrees arc length
    const MAX_TEMP_SCALE = 100;        // 100°C = full gauge arc
    const POLL_INTERVAL_MS = 1500;

    // ─── State ───
    let currentPanel = 'monitor';
    let tempUnit = 'C';                // 'C' or 'F'
    let metricsTimer = null;
    let isMockMode = false;
    let latestRawMetrics = {
        cpu: { temp: 36.0, util_percent: 4, freq_mhz: 2294, fan_rpm: 0, power_watts: 17 },
        gpu: { temp: 38.0, util_percent: 4, freq_mhz: 502, fan_rpm: 0, power_watts: 15 }
    };

    // ─── DOM Selector Helpers ───
    const $ = (sel) => document.querySelector(sel);
    const $$ = (sel) => document.querySelectorAll(sel);

    // ═══════════════════════════════════════════════════════════
    //  NAVIGATION
    // ═══════════════════════════════════════════════════════════

    function initNav() {
        $$('.side-nav-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const panel = btn.dataset.panel;
                switchPanel(panel);
            });
        });
    }

    function switchPanel(name) {
        currentPanel = name;

        // Update sidebar button active state
        $$('.side-nav-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.panel === name);
        });

        // Update view panel visibility
        $$('.view-panel').forEach(panel => {
            panel.classList.toggle('active', panel.id === `panel-${name}`);
        });

        // Load specific panel data
        if (name === 'config') {
            loadConfig();
        } else if (name === 'service') {
            loadServiceStatus();
            loadLogs();
        }
    }

    // ═══════════════════════════════════════════════════════════
    //  TEMPERATURE UNIT TOGGLE (°C / °F)
    // ═══════════════════════════════════════════════════════════

    function initTempUnitToggle() {
        $$('.unit-option').forEach(btn => {
            btn.addEventListener('click', () => {
                setTempUnit(btn.dataset.unit);
            });
        });
    }

    function setTempUnit(unit) {
        tempUnit = unit === 'F' ? 'F' : 'C';

        $$('.unit-option').forEach(b => {
            b.classList.toggle('active', b.dataset.unit === tempUnit);
        });

        // Update gauges and texts with current unit
        renderMetrics(latestRawMetrics);
    }

    function toFahrenheit(celsius) {
        return (celsius * 1.8) + 32.0;
    }

    // ═══════════════════════════════════════════════════════════
    //  GAUGES & METRICS RENDERING
    // ═══════════════════════════════════════════════════════════

    function updateGaugeArc(elementId, tempC) {
        const arc = document.getElementById(elementId);
        if (!arc) return;

        // Ratio clamped 0.0 to 1.0 (based on Celsius scale 0 to 100°C)
        const ratio = Math.min(1.0, Math.max(0.0, tempC / MAX_TEMP_SCALE));
        const filled = ratio * MAX_ARC_LENGTH;
        const empty = TOTAL_CIRCUMFERENCE - filled;

        arc.setAttribute('stroke-dasharray', `${filled.toFixed(1)} ${empty.toFixed(1)}`);
    }

    function renderMetrics(data) {
        if (!data || !data.cpu || !data.gpu) return;
        latestRawMetrics = data;

        const cpu = data.cpu;
        const gpu = data.gpu;
        const isF = tempUnit === 'F';

        // ── CPU ──
        const cpuDisplayTemp = isF ? toFahrenheit(cpu.temp) : cpu.temp;
        const cpuTempNumberEl = $('#cpu-temp-val');
        const cpuTempUnitEl = $('#cpu-temp-unit-label');
        if (cpuTempNumberEl) cpuTempNumberEl.textContent = Math.round(cpuDisplayTemp);
        if (cpuTempUnitEl) cpuTempUnitEl.textContent = isF ? '°F' : '°C';

        updateGaugeArc('cpu-gauge-arc', cpu.temp);

        // Utilization (0 - 100%)
        const cpuUtil = Math.round(cpu.util_percent || 0);
        $('#cpu-util-text').textContent = `${cpuUtil} %`;
        $('#cpu-util-fill').style.width = `${Math.min(100, Math.max(1, cpuUtil))}%`;

        // Frequency (e.g. 2294 MHz)
        const cpuFreq = Math.round(cpu.freq_mhz || 0);
        $('#cpu-freq-text').textContent = `${cpuFreq} MHz`;
        const cpuFreqPercent = Math.min(100, Math.max(2, (cpuFreq / 5000.0) * 100.0));
        $('#cpu-freq-fill').style.width = `${cpuFreqPercent.toFixed(1)}%`;

        // Fan (e.g. 0 RPM)
        const cpuFan = Math.round(cpu.fan_rpm || 0);
        $('#cpu-fan-text').textContent = `${cpuFan} RPM`;
        const cpuFanPercent = cpuFan > 0 ? Math.min(100, Math.max(4, (cpuFan / 3000.0) * 100.0)) : 0;
        $('#cpu-fan-fill').style.width = `${cpuFanPercent}%`;

        // Power (e.g. 17 W)
        const cpuPower = Math.round(cpu.power_watts || 0);
        $('#cpu-power-text').textContent = `${cpuPower} W`;
        const cpuPowerPercent = Math.min(100, Math.max(2, (cpuPower / 125.0) * 100.0));
        $('#cpu-power-fill').style.width = `${cpuPowerPercent.toFixed(1)}%`;

        // ── GPU ──
        const gpuDisplayTemp = isF ? toFahrenheit(gpu.temp) : gpu.temp;
        const gpuTempNumberEl = $('#gpu-temp-val');
        const gpuTempUnitEl = $('#gpu-temp-unit-label');
        if (gpuTempNumberEl) gpuTempNumberEl.textContent = Math.round(gpuDisplayTemp);
        if (gpuTempUnitEl) gpuTempUnitEl.textContent = isF ? '°F' : '°C';

        updateGaugeArc('gpu-gauge-arc', gpu.temp);

        // GPU Utilization
        const gpuUtil = Math.round(gpu.util_percent || 0);
        $('#gpu-util-text').textContent = `${gpuUtil} %`;
        $('#gpu-util-fill').style.width = `${Math.min(100, Math.max(1, gpuUtil))}%`;

        // GPU Frequency
        const gpuFreq = Math.round(gpu.freq_mhz || 0);
        $('#gpu-freq-text').textContent = `${gpuFreq} MHz`;
        const gpuFreqPercent = Math.min(100, Math.max(2, (gpuFreq / 2500.0) * 100.0));
        $('#gpu-freq-fill').style.width = `${gpuFreqPercent.toFixed(1)}%`;

        // GPU Fan
        const gpuFan = Math.round(gpu.fan_rpm || 0);
        $('#gpu-fan-text').textContent = `${gpuFan} PRM`; // Using PRM style from original app
        const gpuFanPercent = gpuFan > 0 ? Math.min(100, Math.max(4, (gpuFan / 3000.0) * 100.0)) : 0;
        $('#gpu-fan-fill').style.width = `${gpuFanPercent}%`;

        // GPU Power
        const gpuPower = Math.round(gpu.power_watts || 0);
        $('#gpu-power-text').textContent = `${gpuPower} W`;
        const gpuPowerPercent = Math.min(100, Math.max(2, (gpuPower / 250.0) * 100.0));
        $('#gpu-power-fill').style.width = `${gpuPowerPercent.toFixed(1)}%`;
    }

    // ═══════════════════════════════════════════════════════════
    //  POLLING & OFFLINE WINDOWS DEMO MODE
    // ═══════════════════════════════════════════════════════════

    function startMetricsPolling() {
        fetchMetrics();
        metricsTimer = setInterval(fetchMetrics, POLL_INTERVAL_MS);
    }

    async function fetchMetrics() {
        // If file:// protocol or already in mock mode, use simulation
        if (window.location.protocol === 'file:' || isMockMode) {
            runMockTick();
            return;
        }

        try {
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), 2000);

            const res = await fetch(`${API_BASE}/api/metrics`, { signal: controller.signal });
            clearTimeout(timeoutId);

            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();

            setConnectionStatus(true, 'En línea');
            renderMetrics(data);
        } catch (err) {
            // When running without Linux daemon (e.g. previewing in Windows Explorer),
            // activate realistic live simulation so the user can test the UI anytime!
            console.log('Activating smooth simulated metrics for Windows preview');
            isMockMode = true;
            runMockTick();
        }
    }

    function runMockTick() {
        setConnectionStatus(true, 'Windows Preview');

        // Realistic subtle fluctuation around screenshot values (36°C CPU, 38°C GPU)
        const cpuJitter = (Math.random() - 0.5) * 1.2;
        const gpuJitter = (Math.random() - 0.5) * 1.2;
        const utilJitter = Math.floor(Math.random() * 3);

        const mockData = {
            cpu: {
                temp: parseFloat((36.0 + cpuJitter).toFixed(1)),
                util_percent: 4 + utilJitter,
                freq_mhz: 2294 + Math.floor((Math.random() - 0.5) * 80),
                fan_rpm: 0,
                power_watts: 17 + Math.floor((Math.random() - 0.5) * 2)
            },
            gpu: {
                temp: parseFloat((38.0 + gpuJitter).toFixed(1)),
                util_percent: 4 + utilJitter,
                freq_mhz: 502 + Math.floor((Math.random() - 0.5) * 30),
                fan_rpm: 0,
                power_watts: 15 + Math.floor((Math.random() - 0.5) * 2)
            }
        };

        renderMetrics(mockData);
    }

    function setConnectionStatus(connected, label) {
        const dot = $('#status-indicator-dot');
        const text = $('#status-indicator-label');
        if (dot) dot.className = `status-indicator-dot ${connected ? '' : 'disconnected'}`;
        if (text) text.textContent = label || (connected ? 'En línea' : 'Sin conexión');
    }

    // ═══════════════════════════════════════════════════════════
    //  CONFIGURATION FORM
    // ═══════════════════════════════════════════════════════════

    async function loadConfig() {
        try {
            const res = await fetch(`${API_BASE}/api/config`);
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const cfg = await res.json();
            populateConfigForm(cfg);
        } catch (err) {
            console.warn('Config fetch fallback:', err);
            // Default mock config
            populateConfigForm({
                display_mode: 'cpu',
                temperature_unit: tempUnit,
                update_interval_ms: 1000,
                alternate_interval_secs: 5,
                gpu_driver: 'nvidia',
                cpu_temp_offset: 0,
                gpu_temp_offset: 0
            });
        }
    }

    function populateConfigForm(cfg) {
        $('#cfg-display-mode').value = cfg.display_mode || 'cpu';

        const unit = cfg.temperature_unit || tempUnit;
        $$('#cfg-temp-unit-group .pill-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.value === unit);
        });

        const interval = cfg.update_interval_ms || 1000;
        $('#cfg-interval').value = interval;
        $('#cfg-interval-label').textContent = `${interval} ms`;

        const altInterval = cfg.alternate_interval_secs || 5;
        $('#cfg-alt-interval').value = altInterval;
        $('#cfg-alt-interval-label').textContent = `${altInterval} s`;

        $('#cfg-gpu-driver').value = cfg.gpu_driver || 'auto';
        $('#cfg-cpu-offset').value = cfg.cpu_temp_offset || 0;
        $('#cfg-gpu-offset').value = cfg.gpu_temp_offset || 0;

        toggleAlternateVisibility(cfg.display_mode);
    }

    function toggleAlternateVisibility(mode) {
        const card = $('#alt-interval-card');
        if (card) {
            card.style.opacity = mode === 'alternate' ? '1' : '0.4';
            card.style.pointerEvents = mode === 'alternate' ? 'auto' : 'none';
        }
    }

    function initConfigForm() {
        $('#config-form').addEventListener('submit', async (e) => {
            e.preventDefault();

            let selectedUnit = 'C';
            $$('#cfg-temp-unit-group .pill-btn').forEach(b => {
                if (b.classList.contains('active')) selectedUnit = b.dataset.value;
            });

            const payload = {
                display_mode: $('#cfg-display-mode').value,
                temperature_unit: selectedUnit,
                update_interval_ms: parseInt($('#cfg-interval').value, 10),
                alternate_interval_secs: parseInt($('#cfg-alt-interval').value, 10),
                gpu_driver: $('#cfg-gpu-driver').value,
                cpu_temp_offset: parseFloat($('#cfg-cpu-offset').value) || 0,
                gpu_temp_offset: parseFloat($('#cfg-gpu-offset').value) || 0
            };

            // Also update topbar unit immediately
            setTempUnit(selectedUnit);

            try {
                const res = await fetch(`${API_BASE}/api/config`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });
                if (!res.ok) throw new Error(`HTTP ${res.status}`);
                showToast('Configuración guardada y servicio actualizado.');
            } catch (err) {
                showToast('Guardado local (Windows Preview)');
            }
        });

        $('#btn-reset-cfg').addEventListener('click', loadConfig);

        $$('#cfg-temp-unit-group .pill-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                $$('#cfg-temp-unit-group .pill-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
            });
        });

        $('#cfg-interval').addEventListener('input', (e) => {
            $('#cfg-interval-label').textContent = `${e.target.value} ms`;
        });

        $('#cfg-alt-interval').addEventListener('input', (e) => {
            $('#cfg-alt-interval-label').textContent = `${e.target.value} s`;
        });

        $('#cfg-display-mode').addEventListener('change', (e) => {
            toggleAlternateVisibility(e.target.value);
        });
    }

    // ═══════════════════════════════════════════════════════════
    //  SERVICE MANAGEMENT
    // ═══════════════════════════════════════════════════════════

    async function loadServiceStatus() {
        try {
            const res = await fetch(`${API_BASE}/api/service/status`);
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            renderServiceStatus(data);
        } catch {
            renderServiceStatus({
                status: 'active',
                detail: 'thermax-pulsar.service (Simulado) — Running'
            });
        }
    }

    function renderServiceStatus(data) {
        const isRunning = data.status === 'active' || data.status === 'running';
        const bulb = $('#service-status-bulb');
        const headline = $('#service-status-headline');
        const subtext = $('#service-status-subtext');

        if (bulb) bulb.className = `service-status-bulb ${isRunning ? '' : 'stopped'}`;
        if (headline) headline.textContent = isRunning ? 'Servicio thermax-pulsar Activo' : 'Servicio Detenido';
        if (subtext) subtext.textContent = data.detail || 'thermax-pulsar.service';

        $('#btn-srv-start').disabled = isRunning;
        $('#btn-srv-stop').disabled = !isRunning;
        $('#btn-srv-restart').disabled = !isRunning;
    }

    async function controlService(action) {
        try {
            const res = await fetch(`${API_BASE}/api/service/${action}`, { method: 'POST' });
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            showToast(`Acción de servicio ejecutada: ${action}`);
            setTimeout(loadServiceStatus, 1200);
        } catch {
            showToast(`Servicio ${action} simulado en Windows.`);
        }
    }

    async function loadLogs() {
        const display = $('#service-logs-display');
        try {
            const res = await fetch(`${API_BASE}/api/service/logs`);
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            display.textContent = data.logs || 'Sin registros.';
        } catch {
            display.textContent = `[thermax-pulsar] Vista previa en Windows activa.\nConectado a la interfaz de control THERMAX 6 PULSAR.\nTelemetría y visualizaciones en tiempo real activas.`;
        }
    }

    function initServiceControls() {
        $('#btn-srv-start').addEventListener('click', () => controlService('start'));
        $('#btn-srv-stop').addEventListener('click', () => controlService('stop'));
        $('#btn-srv-restart').addEventListener('click', () => controlService('restart'));
        $('#btn-refresh-srv-logs').addEventListener('click', loadLogs);
    }

    // ─── Toast Notifications ───
    function showToast(message) {
        const toast = $('#toast-popup');
        const text = $('#toast-text');
        if (!toast || !text) return;

        text.textContent = message;
        toast.classList.add('show');

        setTimeout(() => {
            toast.classList.remove('show');
        }, 3200);
    }

    // ─── Window Controls Simulation ───
    function initWindowControls() {
        $('#win-min-btn').addEventListener('click', () => {
            showToast('Ventana minimizada (simulación)');
        });

        $('#win-close-btn').addEventListener('click', () => {
            showToast('Cerrar aplicación (simulación)');
        });
    }

    // ═══════════════════════════════════════════════════════════
    //  INITIALIZATION
    // ═══════════════════════════════════════════════════════════

    function init() {
        initNav();
        initTempUnitToggle();
        initConfigForm();
        initServiceControls();
        initWindowControls();
        startMetricsPolling();
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
