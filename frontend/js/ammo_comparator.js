const API_BASE = "http://localhost:8080";

class AmmoComparatorComponent {
    constructor(containerId) {
        this.container = document.getElementById(containerId);
        this.result = null;
        this.params = {
            velocity: 50,
            angle: 45,
            mass: 90,
            air_density: 1.225,
            launch_height: 0,
            wind_speed: 0,
            wind_direction: 0,
        };
    }

    setParams(params) {
        Object.assign(this.params, params);
    }

    async compare() {
        const query = new URLSearchParams({
            velocity: this.params.velocity,
            angle: this.params.angle,
            mass: this.params.mass,
            air_density: this.params.air_density,
        });

        try {
            const res = await fetch(`${API_BASE}/api/calc/ammo-compare?${query}`);
            const data = await res.json();
            if (data.success) {
                this.result = data.data;
                this.render();
                return this.result;
            } else {
                this.showError(data.error);
                return null;
            }
        } catch (err) {
            this.showError(err.message);
            return null;
        }
    }

    render() {
        if (!this.result) return;

        const { config, round_stone, gunpowder_bomb, corpse_shell, summary } = this.result;

        const ammoTypes = [
            { key: 'round_stone', name: '圆石弹', icon: '🪨', data: round_stone },
            { key: 'gunpowder_bomb', name: '火药弹', icon: '💥', data: gunpowder_bomb },
            { key: 'corpse_shell', name: '腐尸弹', icon: '☠️', data: corpse_shell },
        ];

        this.container.innerHTML = `
            <div class="ammo-compare-container">
                <h3>📊 弹药对比分析</h3>
                <div class="config-summary">
                    <span>初速: ${config.initial_velocity_mps.toFixed(1)} m/s</span>
                    <span>发射角: ${config.launch_angle_deg.toFixed(1)}°</span>
                    <span>弹重: ${config.projectile_mass_kg} kg</span>
                </div>

                <div class="ammo-grid">
                    ${ammoTypes.map(a => this.renderAmmoCard(a)).join('')}
                </div>

                <div class="summary-section">
                    <h4>综合评估</h4>
                    <div class="summary-grid">
                        <div class="summary-item">
                            <span class="label">综合最佳</span>
                            <span class="value best">${this.getAmmoName(summary.best_overall)}</span>
                        </div>
                        <div class="summary-item">
                            <span class="label">射程最佳</span>
                            <span class="value">${this.getAmmoName(summary.best_range)}</span>
                        </div>
                        <div class="summary-item">
                            <span class="label">伤害最佳</span>
                            <span class="value">${this.getAmmoName(summary.best_damage)}</span>
                        </div>
                        <div class="summary-item">
                            <span class="label">精度最佳</span>
                            <span class="value">${this.getAmmoName(summary.best_accuracy)}</span>
                        </div>
                    </div>
                    <div class="recommendation">
                        <strong>推荐：</strong>${summary.recommendation}
                    </div>
                </div>

                ${summary.notes && summary.notes.length > 0 ? `
                <div class="notes-section">
                    <h4>技术备注</h4>
                    <ul>${summary.notes.map(n => `<li>${n}</li>`).join('')}</ul>
                </div>
                ` : ''}
            </div>
        `;
    }

    renderAmmoCard(ammo) {
        const d = ammo.data;
        const ballistics = d.ballistics;
        const effectiveness = d.effectiveness;

        return `
            <div class="ammo-card">
                <div class="ammo-header">
                    <span class="ammo-icon">${ammo.icon}</span>
                    <span class="ammo-name">${ammo.name}</span>
                </div>
                <div class="ammo-stats">
                    <div class="stat-row">
                        <span class="label">射程</span>
                        <span class="value">${ballistics.range_m.toFixed(1)} m</span>
                    </div>
                    <div class="stat-row">
                        <span class="label">最大高度</span>
                        <span class="value">${ballistics.max_height_m.toFixed(1)} m</span>
                    </div>
                    <div class="stat-row">
                        <span class="label">飞行时间</span>
                        <span class="value">${ballistics.flight_time_s.toFixed(2)} s</span>
                    </div>
                    <div class="stat-row">
                        <span class="label">落地速度</span>
                        <span class="value">${ballistics.impact_velocity_mps.toFixed(1)} m/s</span>
                    </div>
                    <div class="stat-row">
                        <span class="label">动能</span>
                        <span class="value">${ballistics.impact_energy_j.toFixed(0)} J</span>
                    </div>
                    <div class="stat-row">
                        <span class="label">侵彻深度</span>
                        <span class="value">${effectiveness.penetration_depth_m.toFixed(3)} m</span>
                    </div>
                    <div class="stat-row">
                        <span class="label">毁伤面积</span>
                        <span class="value">${effectiveness.damage_area_m2.toFixed(2)} m²</span>
                    </div>
                    <div class="stat-row">
                        <span class="label">效能评分</span>
                        <span class="value score">${effectiveness.effectiveness_score.toFixed(2)}</span>
                    </div>
                </div>
            </div>
        `;
    }

    getAmmoName(type) {
        const names = {
            round_stone: '🪨 圆石弹',
            gunpowder_bomb: '💥 火药弹',
            corpse_shell: '☠️ 腐尸弹',
        };
        return names[type] || type;
    }

    showError(msg) {
        this.container.innerHTML = `
            <div class="error-message">❌ ${msg}</div>
        `;
    }
}
