const API_BASE = "http://localhost:8080";

class BattleSimulatorComponent {
    constructor(listContainerId, uiContainerId) {
        this.listContainer = document.getElementById(listContainerId);
        this.uiContainer = document.getElementById(uiContainerId);
        this.battleState = null;
        this.battleScenario = null;
        this.simulationResult = null;
        this.currentAmmoType = 'round_stone';
        this.trebuchetData = null;
        this.onUpdate = null;
    }

    setTrebuchetData(data) {
        this.trebuchetData = data;
    }

    setCurrentAmmoType(type) {
        this.currentAmmoType = type;
    }

    async listBattles() {
        try {
            const res = await fetch(`${API_BASE}/api/battles`);
            const data = await res.json();
            if (data.success && data.data) {
                this.renderBattleList(data.data);
                return data.data;
            }
        } catch (e) {}
        const localBattles = [
            { id: 1, name: '襄阳之战', year: 1267, description: '蒙古军队围攻南宋襄阳城', attacker: '蒙古帝国', defender: '南宋', attacker_trebuchets: [{ trebuchet_id: 1, name: '回回炮-甲', ammo_type: 'RoundStone', available_ammo: 200 }] },
            { id: 2, name: '君士坦丁堡之围', year: 1453, description: '奥斯曼帝国围攻君士坦丁堡', attacker: '奥斯曼帝国', defender: '拜占庭帝国', attacker_trebuchets: [{ trebuchet_id: 8, name: '无敌砲', ammo_type: 'RoundStone', available_ammo: 300 }] },
            { id: 3, name: '太原攻防战', year: 979, description: '北宋太宗亲征北汉太原城', attacker: '北宋', defender: '北汉', attacker_trebuchets: [{ trebuchet_id: 4, name: '人力砲-一号', ammo_type: 'RoundStone', available_ammo: 300 }] },
        ];
        this.renderBattleList(localBattles);
        return localBattles;
    }

    async startBattle(id) {
        try {
            const res = await fetch(`${API_BASE}/api/battles/${id}/start`);
            const data = await res.json();
            if (data.success && data.data) {
                this.battleState = data.data;
                try {
                    const infoResp = await fetch(`${API_BASE}/api/battles/${id}`);
                    const infoData = await infoResp.json();
                    if (infoData.success && infoData.data) this.battleScenario = infoData.data;
                } catch (e) {}
                this.renderBattleUI();
                return this.battleState;
            }
        } catch (e) {}
        this.battleState = {
            scenario_id: id, current_day: 1, wall_damage: 0, total_impacts: 0,
            successful_hits: 0, ammo_remaining: { 1: 200 }, is_victory: false,
            is_defeat: false, score: 0, impact_log: [],
        };
        this.battleScenario = {
            id, name: '战役',
            attacker_trebuchets: [{ trebuchet_id: 1, name: '回回炮-甲', available_ammo: 200 }],
            victory_conditions: { wall_breach_required: 0.6 },
        };
        this.renderBattleUI();
        return this.battleState;
    }

    async fire(trebuchetId) {
        if (!this.battleState || this.battleState.is_victory || this.battleState.is_defeat) return null;
        const ammo = this.battleState.ammo_remaining[trebuchetId] || 0;
        if (ammo <= 0) return null;

        const targetX = 10 + Math.random() * 10;
        const targetY = 2 + Math.random() * 6;
        const damage = 0.02 + Math.random() * 0.08;

        try {
            const res = await fetch(
                `${API_BASE}/api/battles/${this.battleState.scenario_id}/fire?trebuchet_id=${trebuchetId}` +
                `&target_x=${targetX}&target_y=${targetY}&damage_ratio=${damage}` +
                `&ammo_type=${this.currentAmmoType}`
            );
            const data = await res.json();
            if (data.success && data.data) {
                this.battleState = data.data;
                this.renderBattleUI();
                return this.battleState;
            }
        } catch (e) {}

        this.battleState.total_impacts += 1;
        this.battleState.successful_hits += 1;
        this.battleState.wall_damage += damage;
        if (this.battleState.ammo_remaining[trebuchetId]) this.battleState.ammo_remaining[trebuchetId] -= 1;
        this.battleState.impact_log.push({
            day: this.battleState.current_day, trebuchet_id: trebuchetId,
            target_x: targetX, target_y: targetY, ammo_type: 'RoundStone', damage_ratio: damage,
        });
        if (this.battleState.wall_damage >= 0.6) this.battleState.is_victory = true;
        this.renderBattleUI();
        return this.battleState;
    }

    async advanceDay() {
        if (!this.battleState || this.battleState.is_victory || this.battleState.is_defeat) return null;
        try {
            const res = await fetch(`${API_BASE}/api/battles/${this.battleState.scenario_id}/advance`);
            const data = await res.json();
            if (data.success && data.data) {
                this.battleState = data.data;
                this.renderBattleUI();
                return this.battleState;
            }
        } catch (e) {}
        this.battleState.current_day += 1;
        if (this.battleState.current_day > 30 && !this.battleState.is_victory) this.battleState.is_defeat = true;
        this.renderBattleUI();
        return this.battleState;
    }

    async simulateFull(scenarioId, useBehaviorTree, maxDays) {
        try {
            const params = new URLSearchParams({
                use_bt: useBehaviorTree !== false ? 'true' : 'false',
                max_days: maxDays || 30,
            });
            const res = await fetch(`${API_BASE}/api/battles/${scenarioId}/simulate?${params}`);
            const data = await res.json();
            if (data.success && data.data) {
                this.simulationResult = data.data;
                return this.simulationResult;
            }
        } catch (e) {}
        return null;
    }

    renderBattleList(battles) {
        if (!this.listContainer) return;
        let html = '';
        battles.forEach(b => {
            html += `<div class="battle-card" data-battle-id="${b.id}">
                <div class="battle-name">${b.name}</div>
                <div class="battle-year">${b.year}年</div>
                <div class="battle-desc">${b.description}</div>
            </div>`;
        });
        this.listContainer.innerHTML = html;
        this.listContainer.style.display = '';
        if (this.uiContainer) this.uiContainer.style.display = 'none';
    }

    renderBattleUI() {
        if (!this.uiContainer) return;

        this.listContainer.style.display = 'none';
        this.uiContainer.style.display = '';

        const s = this.battleState;
        const sc = this.battleScenario;
        const damagePct = Math.min(100, s.wall_damage * 100).toFixed(1);
        const victoryPct = sc ? (sc.victory_conditions ? sc.victory_conditions.wall_breach_required * 100 : 60) : 60;

        let html = `
            <div style="display:flex;justify-content:space-between;align-items:center;">
                <span style="font-size:14px;font-weight:500;color:#e2e8f0;">${sc ? sc.name : '战役'}</span>
                <span style="font-size:12px;color:#d4af37;">第 ${s.current_day} 天</span>
            </div>
            <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:8px;margin-top:10px;">
                <div class="stat-card"><span class="label">城墙损伤</span><span class="value">${damagePct}%</span></div>
                <div class="stat-card"><span class="label">命中次数</span><span class="value">${s.successful_hits}</span></div>
                <div class="stat-card"><span class="label">评分</span><span class="value">${s.score.toFixed(0)}</span></div>
            </div>
            <div class="progress-bar"><div class="progress-fill" style="width:${damagePct}%"></div></div>
            <div style="text-align:center;font-size:10px;color:#64748b;margin-top:4px;">胜利条件: ${victoryPct}% 城墙损伤</div>
            ${s.is_victory ? '<div style="text-align:center;font-size:16px;color:#22c55e;font-weight:700;margin-top:8px;">⚔️ 胜利！城墙已突破！</div>' : ''}
            ${s.is_defeat ? '<div style="text-align:center;font-size:16px;color:#ef4444;font-weight:700;margin-top:8px;">💀 战败！攻城失败</div>' : ''}
        `;
        this.uiContainer.querySelector('.battle-info').innerHTML = html;

        const trebs = sc ? sc.attacker_trebuchets : [];
        let controlsHtml = '';
        trebs.forEach(t => {
            const ammo = s.ammo_remaining[t.trebuchet_id] || 0;
            controlsHtml += `<button class="btn-primary" style="font-size:10px;padding:6px 10px;"
                data-fire-id="${t.trebuchet_id}" ${ammo <= 0 || s.is_victory || s.is_defeat ? 'disabled' : ''}>
                ${t.name} (${ammo})
            </button>`;
        });
        controlsHtml += `<button class="btn-secondary" style="font-size:10px;padding:6px 10px;" data-action="advance">⏩ 下一天</button>`;
        this.uiContainer.querySelector('.battle-controls').innerHTML = controlsHtml;

        let logHtml = '';
        (s.impact_log || []).slice(-20).reverse().forEach(l => {
            logHtml += `<div class="log-entry hit">Day${l.day} | #${l.trebuchet_id} → (${l.target_x.toFixed(1)},${l.target_y.toFixed(1)}) | 损伤 ${(l.damage_ratio * 100).toFixed(1)}%</div>`;
        });
        this.uiContainer.querySelector('.battle-log').innerHTML = logHtml || '<div class="log-entry">等待发射...</div>';

        if (this.onUpdate) this.onUpdate(this.battleState);
    }

    showError(msg) {
        if (this.uiContainer) {
            this.uiContainer.innerHTML = `<div class="error-message">❌ ${msg}</div>`;
        }
    }
}
