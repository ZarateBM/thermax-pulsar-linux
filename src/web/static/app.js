/* ═══════════════════════════════════════════════════════════════
   THERMAX PULSAR — Control Panel JavaScript
   Real-time hardware monitoring, configuration, and service control
   ═══════════════════════════════════════════════════════════════ */

(function () {
    'use strict';

    // ─── Constants ───
    const API_BASE = '';
    const GAUGE_ARC_LENGTH = 401;   // stroke-dasharray arc (270° of full circle r=85)
    const MAX_TEMP = 100;           // max temperature for gauge scale
    const POLL_INTERVAL_MS = 1500;

    // ─── State ───
    let currentPanel = 'monitor';
    let metricsTimer = null;
    let tempUnit = 'C';

    // ═══════════════════════════════════════════════════════════
    //  DOM HELPERS
    // ═══════════════════════════════════════════════════════════

    const $ = (sel) => document.querySelector(sel);
    const $$ = (sel) => document.querySelectorAll(sel);

    // ═══════════════════════════════════════════════════════════
    //  NAVIGATION
    // ═══════════════════════════════════════════════════════════

    function initNav() {
        $$('.nav-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const panel = btn.dataset.panel;
                switchPanel(panel);
            });
        });
    }

    function switchPanel(name) {
        currentPanel = name;

        // Update nav buttons
        $$('.nav-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.panel === name);
        });

        // Update panels
        $$('.panel').forEach(panel => {
            panel.classList.toggle('active', panel.id === `panel-${name}`);
        });

        // Load panel data
        if (name === 'config') {
            loadConfig();
        } else if (name === 'service') {
            loadServiceStatus();
            loadLogs();
        }
    }

    // ═══════════════════════════════════════════════════════════
    //  HARDWARE MONITOR
    // ═══════════════════════════════════════════════════════════

    function startMetricsPolling() {
        fetchMetrics();
        metricsTimer = setInterval(fetchMetrics, POLL_INTERVAL_MS);
    }

    async function fetchMetrics() {
        try {
            const res = await fetch(`${API_BASE}/api/metrics`);
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();

            updateConnectionStatus('connected');
            renderMetrics(data);
        } catch (err) {
            console.warn('Metrics fetch error:', err);
            updateConnectionStatus('disconnected');
        }
    }

    function renderMetrics(data) {
        const cpu = data.cpu || {};
        const gpu = data.gpu || {};
        const isFahrenheit = tempUnit === 'F';

        // CPU
        const cpuTemp = isFahrenheit ? toFahrenheit(cpu.temp || 0) : (cpu.temp || 0);
        const cpuUnitStr = isFahrenheit ? '°F' : '°C';
        $('#cpu-temp-value').textContent = Math.round(cpuTemp);
        $('#cpu-temp-unit').textContent = cpuUnitStr;
        $('#cpu-util-value').textContent = `${Math.round(cpu.util_percent || 0)}%`;
        $('#cpu-util-bar').style.width = `${Math.min(100, cpu.util_percent || 0)}%`;
        $('#cpu-freq-value').textContent = `${cpu.freq_mhz || 0} MHz`;
        $('#cpu-fan-value').textContent = `${cpu.fan_rpm || 0} RPM`;
        $('#cpu-power-value').textContent = `${cpu.power_watts || 0} W`;
        updateGauge('cpu', cpu.temp || 0);

        // GPU
        const gpuTemp = isFahrenheit ? toFahrenheit(gpu.temp || 0) : (gpu.temp || 0);
        $('#gpu-temp-value').textContent = Math.round(gpuTemp);
        $('#gpu-temp-unit').textContent = cpuUnitStr;
        $('#gpu-util-value').textContent = `${Math.round(gpu.util_percent || 0)}%`;
        $('#gpu-util-bar').style.width = `${Math.min(100, gpu.util_percent || 0)}%`;
        $('#gpu-freq-value').textContent = `${gpu.freq_mhz || 0} MHz`;
        $('#gpu-fan-value').textContent = `${gpu.fan_rpm || 0} RPM`;
        $('#gpu-power-value').textContent = `${gpu.power_watts || 0} W`;
        updateGauge('gpu', gpu.temp || 0);
    }

    function updateGauge(type, tempC) {
        const fillEl = $(`#${type}-gauge-fill`);
        if (!fillEl) return;

        // Clamp between 0 and MAX_TEMP for gauge visual
        const ratio = Math.min(1, Math.max(0, tempC / MAX_TEMP));
        const filled = ratio * GAUGE_ARC_LENGTH;
        const empty = 534 - filled;  // total circumference ~534 for r=85

        fillEl.setAttribute('stroke-dasharray', `${filled} ${empty}`);

        // Color temperature (cool → warm → hot)
        const valueEl = $(`#${type}-temp-value`);
        if (tempC < 45) {
            valueEl.style.color = '#f0f0f5';   // cool: white
        } else if (tempC < 70) {
            valueEl.style.color = '#ffcc00';    // warm: yellow
        } else {
            valueEl.style.color = '#ff3b30';    // hot: red
        }
    }

    function toFahrenheit(c) {
        return (c * 1.8) + 32;
    }

    function initTempUnitToggle() {
        $$('.unit-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                tempUnit = btn.dataset.unit;
                $$('.unit-btn').forEach(b => b.classList.toggle('active', b.dataset.unit === tempUnit));
            });
        });
    }

    function updateConnectionStatus(status) {
        const el = $('#connection-status');
        el.className = `connection-status ${status}`;
        const text = el.querySelector('.status-text');
        if (status === 'connected') {
            text.textContent = 'Conectado';
        } else if (status === 'disconnected') {
            text.textContent = 'Sin conexión';
        } else {
            text.textContent = 'Conectando...';
        }
    }

    // ═══════════════════════════════════════════════════════════
    //  CONFIGURATION
    // ═══════════════════════════════════════════════════════════

    async function loadConfig() {
        try {
            const res = await fetch(`${API_BASE}/api/config`);
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const cfg = await res.json();
            populateConfigForm(cfg);
        } catch (err) {
            console.error('Failed to load config:', err);
            showToast('Error al cargar configuración', true);
        }
    }

    function populateConfigForm(cfg) {
        // Display mode
        $('#cfg-display-mode').value = cfg.display_mode || 'cpu';

        // Temperature unit
        const unit = cfg.temperature_unit || 'C';
        $$('#cfg-temp-unit-group .toggle-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.value === unit);
        });

        // Update interval
        const interval = cfg.update_interval_ms || 1000;
        $('#cfg-interval').value = interval;
        $('#cfg-interval-label').textContent = `${interval} ms`;

        // Alternate interval
        const altInterval = cfg.alternate_interval_secs || 5;
        $('#cfg-alt-interval').value = altInterval;
        $('#cfg-alt-interval-label').textContent = `${altInterval} s`;

        // GPU driver
        $('#cfg-gpu-driver').value = cfg.gpu_driver || 'auto';

        // Offsets
        $('#cfg-cpu-offset').value = cfg.cpu_temp_offset || 0;
        $('#cfg-gpu-offset').value = cfg.gpu_temp_offset || 0;

        // Show/hide alternate interval card
        toggleAlternateCard(cfg.display_mode);
    }

    function collectConfigForm() {
        // Get active temperature unit
        let selectedUnit = 'C';
        $$('#cfg-temp-unit-group .toggle-btn').forEach(btn => {
            if (btn.classList.contains('active')) {
                selectedUnit = btn.dataset.value;
            }
        });

        return {
            display_mode: $('#cfg-display-mode').value,
            temperature_unit: selectedUnit,
            update_interval_ms: parseInt($('#cfg-interval').value, 10),
            alternate_interval_secs: parseInt($('#cfg-alt-interval').value, 10),
            gpu_driver: $('#cfg-gpu-driver').value,
            cpu_temp_offset: parseFloat($('#cfg-cpu-offset').value) || 0,
            gpu_temp_offset: parseFloat($('#cfg-gpu-offset').value) || 0,
        };
    }

    async function saveConfig(e) {
        e.preventDefault();
        const cfg = collectConfigForm();

        try {
            const res = await fetch(`${API_BASE}/api/config`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(cfg),
            });

            if (!res.ok) {
                const errData = await res.json().catch(() => ({}));
                throw new Error(errData.error || `HTTP ${res.status}`);
            }

            showToast('Configuración guardada exitosamente. Servicio reiniciado.');
        } catch (err) {
            console.error('Save config error:', err);
            showToast(`Error al guardar: ${err.message}`, true);
        }
    }

    function toggleAlternateCard(mode) {
        const card = $('#alternate-interval-card');
        if (card) {
            card.style.opacity = mode === 'alternate' ? '1' : '0.4';
            card.style.pointerEvents = mode === 'alternate' ? 'auto' : 'none';
        }
    }

    function initConfigForm() {
        // Form submit
        $('#config-form').addEventListener('submit', saveConfig);

        // Reset button
        $('#btn-reset-config').addEventListener('click', loadConfig);

        // Toggle buttons for temp unit
        $$('#cfg-temp-unit-group .toggle-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                $$('#cfg-temp-unit-group .toggle-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
            });
        });

        // Range input live labels
        $('#cfg-interval').addEventListener('input', (e) => {
            $('#cfg-interval-label').textContent = `${e.target.value} ms`;
        });

        $('#cfg-alt-interval').addEventListener('input', (e) => {
            $('#cfg-alt-interval-label').textContent = `${e.target.value} s`;
        });

        // Display mode change → toggle alternate card
        $('#cfg-display-mode').addEventListener('change', (e) => {
            toggleAlternateCard(e.target.value);
        });
    }

    // ─── Toast ───

    function showToast(message, isError = false) {
        const toast = $('#config-toast');
        const msgEl = $('#toast-message');
        const iconEl = toast.querySelector('.toast-icon');

        msgEl.textContent = message;
        toast.classList.toggle('error', isError);
        iconEl.textContent = isError ? '✕' : '✓';
        toast.classList.add('show');

        setTimeout(() => {
            toast.classList.remove('show');
        }, 3500);
    }

    // ═══════════════════════════════════════════════════════════
    //  SERVICE CONTROL
    // ═══════════════════════════════════════════════════════════

    async function loadServiceStatus() {
        try {
            const res = await fetch(`${API_BASE}/api/service/status`);
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            renderServiceStatus(data);
        } catch (err) {
            console.warn('Service status error:', err);
            renderServiceStatus({ status: 'unknown', detail: 'No se pudo verificar' });
        }
    }

    function renderServiceStatus(data) {
        const indicator = $('#service-indicator');
        const statusText = $('#service-status-text');
        const statusDetail = $('#service-status-detail');
        const btnStart = $('#btn-service-start');
        const btnStop = $('#btn-service-stop');
        const btnRestart = $('#btn-service-restart');

        const isRunning = data.status === 'active' || data.status === 'running';
        const isStopped = data.status === 'inactive' || data.status === 'dead' || data.status === 'stopped';

        indicator.className = `service-indicator ${isRunning ? 'running' : isStopped ? 'stopped' : ''}`;

        if (isRunning) {
            statusText.textContent = '● Servicio Activo';
            statusText.style.color = 'var(--success)';
        } else if (isStopped) {
            statusText.textContent = '○ Servicio Detenido';
            statusText.style.color = 'var(--danger)';
        } else {
            statusText.textContent = `◌ ${data.status || 'Desconocido'}`;
            statusText.style.color = 'var(--warning)';
        }

        statusDetail.textContent = data.detail || 'thermax-pulsar.service';

        btnStart.disabled = isRunning;
        btnStop.disabled = isStopped;
        btnRestart.disabled = isStopped;
    }

    async function controlService(action) {
        try {
            const res = await fetch(`${API_BASE}/api/service/${action}`, { method: 'POST' });
            if (!res.ok) {
                const errData = await res.json().catch(() => ({}));
                throw new Error(errData.error || `HTTP ${res.status}`);
            }
            showToast(`Servicio: ${action} ejecutado correctamente`);
            // Wait a beat for systemd to process
            setTimeout(loadServiceStatus, 1500);
        } catch (err) {
            showToast(`Error en ${action}: ${err.message}`, true);
        }
    }

    async function loadLogs() {
        try {
            const res = await fetch(`${API_BASE}/api/service/logs`);
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            const container = $('#logs-content');
            container.textContent = data.logs || 'Sin registros disponibles';
            // Scroll to bottom
            const scrollContainer = $('#logs-container');
            scrollContainer.scrollTop = scrollContainer.scrollHeight;
        } catch (err) {
            $('#logs-content').textContent = 'Error al cargar registros: ' + err.message;
        }
    }

    function initServiceControls() {
        $('#btn-service-start').addEventListener('click', () => controlService('start'));
        $('#btn-service-stop').addEventListener('click', () => controlService('stop'));
        $('#btn-service-restart').addEventListener('click', () => controlService('restart'));
        $('#btn-refresh-logs').addEventListener('click', loadLogs);
    }

    // ═══════════════════════════════════════════════════════════
    //  INIT
    // ═══════════════════════════════════════════════════════════

    function init() {
        initNav();
        initTempUnitToggle();
        initConfigForm();
        initServiceControls();
        startMetricsPolling();

        // Keyboard shortcuts
        document.addEventListener('keydown', (e) => {
            if (e.key === '1') switchPanel('monitor');
            if (e.key === '2') switchPanel('config');
            if (e.key === '3') switchPanel('service');
        });
    }

    // Start when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
