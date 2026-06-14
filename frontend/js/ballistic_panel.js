const API_BASE = "http://localhost:8080";

const MOCK_TREBUCHETS = [
    { id: 1,  name: "回回炮-甲",   type_: "配重式",     counterweight_kg: 3000, projectile_kg: 90,  arm_length_m: 12.0, max_angle_deg: 50.0 },
    { id: 2,  name: "回回炮-乙",   type_: "配重式",     counterweight_kg: 5000, projectile_kg: 150, arm_length_m: 15.0, max_angle_deg: 55.0 },
    { id: 3,  name: "襄阳砲-壹",   type_: "配重式",     counterweight_kg: 4000, projectile_kg: 120, arm_length_m: 13.5, max_angle_deg: 52.0 },
    { id: 4,  name: "人力砲-一号", type_: "人力牵引式", counterweight_kg: 0,    projectile_kg: 30,  arm_length_m: 8.0,  max_angle_deg: 45.0 },
    { id: 5,  name: "人力砲-二号", type_: "人力牵引式", counterweight_kg: 0,    projectile_kg: 25,  arm_length_m: 7.5,  max_angle_deg: 42.0 },
    { id: 6,  name: "旋风砲",     type_: "人力牵引式", counterweight_kg: 0,    projectile_kg: 20,  arm_length_m: 6.0,  max_angle_deg: 48.0 },
    { id: 7,  name: "虎蹲砲",     type_: "配重式",     counterweight_kg: 1500, projectile_kg: 50,  arm_length_m: 9.0,  max_angle_deg: 47.0 },
    { id: 8,  name: "无敌砲",     type_: "配重式",     counterweight_kg: 6000, projectile_kg: 200, arm_length_m: 18.0, max_angle_deg: 58.0 },
    { id: 9,  name: "飞云砲",     type_: "人力牵引式", counterweight_kg: 0,    projectile_kg: 15,  arm_length_m: 5.5,  max_angle_deg: 40.0 },
    { id: 10, name: "震天雷砲",   type_: "配重式",     counterweight_kg: 8000, projectile_kg: 300, arm_length_m: 20.0, max_angle_deg: 60.0 },
];

const MOCK_WALLS = [
    { id: 1, name: "夯土墙",       material: "rammed_earth",         thickness_m: 3.0, density_kgm3: 1800, compressive_strength_pa: 2000000,  tensile_strength_pa: 200000  },
    { id: 2, name: "包砖墙",       material: "brick_veneer",         thickness_m: 2.5, density_kgm3: 2000, compressive_strength_pa: 10000000, tensile_strength_pa: 800000  },
    { id: 3, name: "石砌墙",       material: "stone_masonry",        thickness_m: 4.0, density_kgm3: 2400, compressive_strength_pa: 25000000, tensile_strength_pa: 2000000 },
    { id: 4, name: "双层夯土墙",   material: "double_rammed_earth",  thickness_m: 6.0, density_kgm3: 1700, compressive_strength_pa: 1800000,  tensile_strength_pa: 180000  },
    { id: 5, name: "糯米灰浆墙",   material: "sticky_rice_lime",     thickness_m: 3.5, density_kgm3: 2100, compressive_strength_pa: 15000000, tensile_strength_pa: 1200000 },
];

class BallisticPanelController {
    constructor(sceneManager, particleSystem) {
        this.scene = sceneManager;
        this.particles = particleSystem;

        this.currentTrebuchetId = 1;
        this.currentWallId = 1;
        this.wallTypes = [];
        this.currentAmmoType = 'round_stone';
        this.battleState = null;
        this.battleScenario = null;

        this.ballisticsResult = null;
        this.siegeResult = null;

        this.params = {
            velocity: 50,
            angle: 45,
            windSpeed: 0,
            windDirection: 0,
        };

        this.trebuchetData = null;

        this.bindEvents();
        this.loadInitialData();
    }

    bindEvents() {
        window.addEventListener('resize', () => this.scene.onWindowResize());

        document.getElementById('fireBtn').addEventListener('click', () => this.onFire());
        document.getElementById('optimizeBtn').addEventListener('click', () => this.onOptimize());
        document.getElementById('ammoCompareBtn').addEventListener('click', () => this.onAmmoCompare());
        document.getElementById('analyzeWallBtn').addEventListener('click', () => this.onAnalyzeWall());
        document.getElementById('findWeakBtn').addEventListener('click', () => this.onFindWeakPoint());
        document.getElementById('coordinateBtn').addEventListener('click', () => this.onCoordinate());
        document.getElementById('battleBtn').addEventListener('click', () => this.onBattle());

        this.bindSlider('velocitySlider', 'velocityValue', 'velocity');
        this.bindSlider('angleSlider', 'angleValue', 'angle', () => this.syncArmAngle());
        this.bindSlider('windSlider', 'windValue', 'windSpeed');

        document.querySelectorAll('.ammo-item').forEach(el => {
            el.addEventListener('click', () => {
                document.querySelectorAll('.ammo-item').forEach(e => e.classList.remove('active'));
                el.classList.add('active');
                this.currentAmmoType = el.dataset.ammo;
            });
        });

        ['viewTop', 'viewSide', 'view3d', 'viewReset'].forEach(id => {
            document.getElementById(id).addEventListener('click', () => {
                this.scene.setView(id.replace('view', '').toLowerCase());
            });
        });

        document.getElementById('popupClose').addEventListener('click', () => {
            document.getElementById('infoPopup').classList.remove('active');
        });
        document.getElementById('feaPopupClose').addEventListener('click', () => {
            document.getElementById('feaPopup').classList.remove('active');
        });
        document.getElementById('gaPopupClose').addEventListener('click', () => {
            document.getElementById('gaPopup').classList.remove('active');
        });
        document.getElementById('coordinatePopupClose').addEventListener('click', () => {
            document.getElementById('coordinatePopup').classList.remove('active');
        });
        document.getElementById('battlePopupClose').addEventListener('click', () => {
            document.getElementById('battlePopup').classList.remove('active');
        });
        document.getElementById('ammoComparePopupClose').addEventListener('click', () => {
            document.getElementById('ammoComparePopup').classList.remove('active');
        });

        this.scene.getCanvasDomElement().addEventListener('click', (e) => this.onCanvasClick(e));
    }

    bindSlider(sliderId, valueId, paramKey, onChangeExtra) {
        const slider = document.getElementById(sliderId);
        const valueEl = document.getElementById(valueId);
        slider.addEventListener('input', (e) => {
            this.params[paramKey] = parseFloat(e.target.value);
            valueEl.textContent = e.target.value;
            if (onChangeExtra) onChangeExtra();
        });
    }

    syncArmAngle() {
        this.scene.setTrebuchetArmAngle(this.currentTrebuchetId, this.params.angle);
    }

    async loadInitialData() {
        let trebuchetsLoaded = false;
        try {
            const response = await fetch(`${API_BASE}/api/trebuchets`);
            const data = await response.json();
            if (data.success && data.data) {
                this.trebuchetData = data.data;
                this.scene.createTrebuchets(data.data);
                this.renderTrebuchetList(data.data);
                this.updateConnectionStatus(true);
                trebuchetsLoaded = true;
            }
        } catch (e) {
            console.warn('Failed to load trebuchets from API, using mock data');
        }

        if (!trebuchetsLoaded) {
            this.updateConnectionStatus(false);
            this.trebuchetData = MOCK_TREBUCHETS;
            this.scene.createTrebuchets(MOCK_TREBUCHETS);
            this.renderTrebuchetList(MOCK_TREBUCHETS);
        }

        try {
            const response = await fetch(`${API_BASE}/api/walls`);
            const data = await response.json();
            if (data.success && data.data) {
                this.wallTypes = data.data;
                this.renderWallList(data.data);
            }
        } catch (e) {
            this.wallTypes = MOCK_WALLS;
            this.renderWallList(MOCK_WALLS);
        }
    }

    renderTrebuchetList(data) {
        const list = document.getElementById('trebuchetList');
        list.innerHTML = '';
        data.forEach(t => {
            const item = document.createElement('div');
            item.className = 'trebuchet-item' + (t.id === this.currentTrebuchetId ? ' active' : '');
            item.innerHTML = `
                <div class="name">${t.name}</div>
                <div class="type">${t.type_ || t.type}</div>
                <div class="stats">
                    <span>弹重: ${t.projectile_kg}kg</span>
                    <span>臂长: ${t.arm_length_m}m</span>
                </div>
            `;
            item.addEventListener('click', () => this.selectTrebuchet(t.id));
            list.appendChild(item);
        });
    }

    renderWallList(walls) {
        const selector = document.getElementById('wallSelector');
        selector.innerHTML = '';
        walls.forEach(w => {
            const item = document.createElement('div');
            item.className = 'wall-item' + (w.id === this.currentWallId ? ' active' : '');
            item.innerHTML = `
                <div>${w.name}</div>
                <div style="font-size: 10px; color: #64748b; margin-top: 2px;">
                    厚度: ${w.thickness_m}m
                </div>
            `;
            item.addEventListener('click', () => this.selectWall(w.id));
            selector.appendChild(item);
        });
    }

    selectTrebuchet(id) {
        this.currentTrebuchetId = id;
        document.querySelectorAll('.trebuchet-item').forEach((el, idx) => {
            el.classList.toggle('active', this.trebuchetData[idx].id === id);
        });
        const t = this.trebuchetData.find(t => t.id === id);
        if (t) {
            document.getElementById('velocitySlider').max = Math.max(100, t.max_angle_deg * 2);
            this.syncArmAngle();
        }
    }

    selectWall(id) {
        this.currentWallId = id;
        document.querySelectorAll('.wall-item').forEach((el, idx) => {
            el.classList.toggle('active', this.wallTypes[idx].id === id);
        });
    }

    async onFire() {
        this.particles.clearAll();
        const tData = this.scene.findTrebuchetDataById(this.currentTrebuchetId);
        if (!tData) return;

        this.scene.animateTrebuchetFire(this.currentTrebuchetId, 500);

        setTimeout(async () => {
            try {
                const response = await fetch(
                    `${API_BASE}/api/calc/ballistics?velocity=${this.params.velocity}` +
                    `&angle=${this.params.angle}&mass=${tData.projectile_kg}` +
                    `&wind_speed=${this.params.windSpeed}&wind_direction=0`
                );
                const data = await response.json();
                if (data.success && data.data) {
                    this.ballisticsResult = data.data;
                    this.showTrajectory(data.data, tData);
                    this.updateHUD(data.data);
                    await this.calculateSiegeRemote(data.data, tData);
                    return;
                }
            } catch (e) {}

            const trajectory = this.calcLocalTrajectory(tData);
            this.showTrajectory(trajectory, tData);
            this.updateHUD(trajectory);
            this.calcLocalSiege(trajectory, tData);
        }, 300);
    }

    calcLocalTrajectory(tData) {
        const g = 9.81;
        const angleRad = this.params.angle * Math.PI / 180;
        const v0 = this.params.velocity;
        const points = [];
        const dt = 0.05;
        let t = 0;

        while (true) {
            const x = v0 * Math.cos(angleRad) * t;
            const y = 5 + v0 * Math.sin(angleRad) * t - 0.5 * g * t * t;
            if (y < 0 || t > 30) break;
            const velocity = Math.sqrt(
                Math.pow(v0 * Math.cos(angleRad), 2) +
                Math.pow(v0 * Math.sin(angleRad) - g * t, 2)
            );
            points.push({ x, y, z: 0, velocity, time_s: t });
            t += dt;
        }

        const last = points[points.length - 1];
        const impactVelocity = last ? last.velocity : v0;
        return {
            max_height_m: Math.max(...points.map(p => p.y)),
            range_m: last ? last.x : 0,
            flight_time_s: last ? last.time_s : 0,
            impact_velocity_mps: impactVelocity,
            impact_kinetic_energy_j: 0.5 * tData.projectile_kg * impactVelocity * impactVelocity,
            trajectory: points,
            impact_angle_deg: this.params.angle,
        };
    }

    calcLocalSiege(ballistics, tData) {
        const wall = this.wallTypes.find(w => w.id === this.currentWallId);
        if (!wall) return;

        const energy = ballistics.impact_kinetic_energy_j;
        const craterDepth = Math.min(wall.thickness_m * 0.8, (energy / 1000000) * 0.5);
        const damageRatio = Math.min(1, craterDepth / wall.thickness_m);
        const score = Math.min(100, damageRatio * 80 + (energy / 1000000) * 20);

        this.siegeResult = {
            crater_depth_m: craterDepth,
            crater_diameter_m: craterDepth * 2.5,
            damage_ratio: damageRatio,
            effectiveness_score: score,
            structural_damage: this.classifyDamage(damageRatio),
        };
        this.updateSiegeHUD(this.siegeResult);
    }

    async calculateSiegeRemote(ballistics, tData) {
        const wall = this.wallTypes.find(w => w.id === this.currentWallId);
        if (!wall) return;
        try {
            const response = await fetch(
                `${API_BASE}/api/calc/siege?impact_energy=${ballistics.impact_kinetic_energy_j}` +
                `&projectile_mass=${tData.projectile_kg}` +
                `&projectile_diameter=0.4` +
                `&impact_angle=${ballistics.impact_angle_deg}` +
                `&wall_thickness=${wall.thickness_m}` +
                `&wall_density=${wall.density_kgm3}` +
                `&wall_compressive_strength=${wall.compressive_strength_pa}` +
                `&wall_tensile_strength=${wall.tensile_strength_pa}` +
                `&ammo_type=${this.currentAmmoType}`
            );
            const data = await response.json();
            if (data.success && data.data) {
                this.siegeResult = data.data;
                this.updateSiegeHUD(data.data);
            }
        } catch (e) {
            this.calcLocalSiege(ballistics, tData);
        }
    }

    async onOptimize() {
        const tData = this.scene.findTrebuchetDataById(this.currentTrebuchetId);
        const wall = this.wallTypes.find(w => w.id === this.currentWallId);
        if (!tData || !wall) return;

        try {
            const response = await fetch(
                `${API_BASE}/api/calc/optimize?projectile_mass=${tData.projectile_kg}` +
                `&wall_thickness=${wall.thickness_m}` +
                `&wall_density=${wall.density_kgm3}` +
                `&wall_compressive_strength=${wall.compressive_strength_pa}` +
                `&min_velocity=20&max_velocity=80` +
                `&min_angle=30&max_angle=60`
            );
            const data = await response.json();
            if (data.success && data.data) {
                this.applyOptimal(data.data);
            }
        } catch (e) {
            console.log('Optimize not available offline');
        }
    }

    applyOptimal(opt) {
        document.getElementById('angleSlider').value = opt.optimal_angle_deg.toFixed(1);
        document.getElementById('angleValue').textContent = opt.optimal_angle_deg.toFixed(1);
        document.getElementById('velocitySlider').value = opt.optimal_velocity_mps.toFixed(1);
        document.getElementById('velocityValue').textContent = opt.optimal_velocity_mps.toFixed(1);
        this.params.angle = opt.optimal_angle_deg;
        this.params.velocity = opt.optimal_velocity_mps;
        this.syncArmAngle();
    }

    classifyDamage(ratio) {
        if (ratio >= 0.9) return "完全摧毁";
        if (ratio >= 0.7) return "严重破坏";
        if (ratio >= 0.5) return "中等破坏";
        if (ratio >= 0.3) return "轻度破坏";
        if (ratio >= 0.1) return "表面损伤";
        return "无明显损伤";
    }

    showTrajectory(result, tData) {
        const trebuchet = this.scene.findTrebuchetById(this.currentTrebuchetId);
        if (!trebuchet || !result.trajectory) return;

        const startPos = trebuchet.group.position;

        const adjustedPoints = result.trajectory.map(p => ({
            x: startPos.x + p.x * Math.sin(trebuchet.rotation),
            y: startPos.y + p.y + 2,
            z: startPos.z - p.x * Math.cos(trebuchet.rotation),
        }));

        this.particles.createTrajectoryLine(adjustedPoints, { color: 0xffd700, speed: 0.8 });
        this.particles.createTrajectory(adjustedPoints, {
            color: 0xffa500,
            particleSize: 0.4,
            speed: 0.8,
        });
    }

    updateHUD(r) {
        document.getElementById('hudRange').textContent = r.range_m.toFixed(1) + ' m';
        document.getElementById('hudHeight').textContent = r.max_height_m.toFixed(1) + ' m';
        document.getElementById('hudTime').textContent = r.flight_time_s.toFixed(2) + ' s';
        document.getElementById('hudImpactVel').textContent = r.impact_velocity_mps.toFixed(1) + ' m/s';
        document.getElementById('hudEnergy').textContent = (r.impact_kinetic_energy_j / 1000).toFixed(1) + ' kJ';
    }

    updateSiegeHUD(r) {
        document.getElementById('hudCrater').textContent = r.crater_depth_m.toFixed(2) + ' m';
        document.getElementById('hudDamage').textContent = (r.damage_ratio * 100).toFixed(1) + ' %';
        document.getElementById('hudScore').textContent = r.effectiveness_score.toFixed(1);
        document.getElementById('damageFill').style.width = (r.damage_ratio * 100) + '%';
        document.getElementById('damageLabel').textContent = r.structural_damage || '--';
    }

    updateConnectionStatus(connected) {
        const badge = document.getElementById('connectionStatus');
        if (connected) {
            badge.classList.remove('disconnected');
            badge.querySelector('span:last-child').textContent = '已连接';
        } else {
            badge.classList.add('disconnected');
            badge.querySelector('span:last-child').textContent = '离线模式';
        }
    }

    onCanvasClick(event) {
        const rect = this.scene.getCanvasDomElement().getBoundingClientRect();
        const mouse = this.scene.getMouseVector();
        mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
        mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;

        const raycaster = this.scene.getRaycaster();
        raycaster.setFromCamera(mouse, this.scene.camera);

        const meshes = this.scene.getRaycastTrebuchetMeshes();
        const intersects = raycaster.intersectObjects(meshes);

        if (intersects.length > 0) {
            const id = this.scene.resolveTrebuchetFromIntersect(intersects[0].object);
            if (id !== null) {
                this.showTrebuchetInfo(id);
            }
        }
    }

    async onAmmoCompare() {
        const tData = this.scene.findTrebuchetDataById(this.currentTrebuchetId);
        if (!tData) return;
        try {
            const response = await fetch(
                `${API_BASE}/api/calc/ammo-compare?velocity=${this.params.velocity}` +
                `&angle=${this.params.angle}&mass=${tData.projectile_kg}`
            );
            const data = await response.json();
            if (data.success && data.data) {
                this.renderAmmoCompare(data.data);
                document.getElementById('ammoComparePopup').classList.add('active');
                return;
            }
        } catch (e) {}
        this.renderAmmoCompare(this.calcLocalAmmoCompare(tData));
        document.getElementById('ammoComparePopup').classList.add('active');
    }

    calcLocalAmmoCompare(tData) {
        const g = 9.81;
        const angleRad = this.params.angle * Math.PI / 180;
        const v0 = this.params.velocity;
        const range = v0 * v0 * Math.sin(2 * angleRad) / g;
        const maxH = v0 * v0 * Math.sin(angleRad) * Math.sin(angleRad) / (2 * g);
        const impactE = 0.5 * tData.projectile_kg * v0 * v0 * 0.7;
        return {
            round_stone: { ammo_type: 'RoundStone', estimated_range_m: range, estimated_max_height_m: maxH, estimated_impact_energy_j: impactE, explosive_energy_j: 0, total_damage_potential: impactE, blast_radius_m: 0, contamination_radius_m: 0 },
            gunpowder_bomb: { ammo_type: 'GunpowderBomb', estimated_range_m: range * 0.85, estimated_max_height_m: maxH * 0.85, estimated_impact_energy_j: impactE * 0.8, explosive_energy_j: tData.projectile_kg * 0.3 * 3000000, total_damage_potential: impactE * 0.8 + tData.projectile_kg * 0.3 * 3000000 * 0.3, blast_radius_m: Math.pow(tData.projectile_kg * 0.3, 1/3) * 2, contamination_radius_m: 0 },
            corpse_shell: { ammo_type: 'CorpseShell', estimated_range_m: range * 0.7, estimated_max_height_m: maxH * 0.7, estimated_impact_energy_j: impactE * 0.6, explosive_energy_j: 0, total_damage_potential: impactE * 0.6 + Math.pow(tData.projectile_kg, 0.4) * 0.5 * 1000, blast_radius_m: 0, contamination_radius_m: Math.pow(tData.projectile_kg, 0.4) * 0.5 },
        };
    }

    renderAmmoCompare(comp) {
        const names = { RoundStone: '圆石弹', GunpowderBomb: '火药弹', CorpseShell: '腐尸弹' };
        const keys = ['round_stone', 'gunpowder_bomb', 'corpse_shell'];
        let html = '<div class="ammo-compare-grid">';
        keys.forEach(k => {
            const d = comp[k];
            html += `<div class="ammo-compare-card">
                <h4>${names[d.ammo_type] || k}</h4>
                <div class="stat-row"><span class="label">射程</span><span class="value">${d.estimated_range_m.toFixed(1)} m</span></div>
                <div class="stat-row"><span class="label">最大高度</span><span class="value">${d.estimated_max_height_m.toFixed(1)} m</span></div>
                <div class="stat-row"><span class="label">冲击动能</span><span class="value">${(d.estimated_impact_energy_j / 1000).toFixed(1)} kJ</span></div>
                <div class="stat-row"><span class="label">爆炸能量</span><span class="value">${(d.explosive_energy_j / 1000).toFixed(1)} kJ</span></div>
                <div class="stat-row"><span class="label">综合破坏</span><span class="value">${(d.total_damage_potential / 1000).toFixed(1)} kJ</span></div>
                <div class="stat-row"><span class="label">爆炸半径</span><span class="value">${d.blast_radius_m.toFixed(2)} m</span></div>
                <div class="stat-row"><span class="label">污染半径</span><span class="value">${d.contamination_radius_m.toFixed(2)} m</span></div>
            </div>`;
        });
        html += '</div>';
        document.getElementById('ammoCompareBody').innerHTML = html;
    }

    async onAnalyzeWall() {
        const wall = this.wallTypes.find(w => w.id === this.currentWallId);
        if (!wall) return;
        try {
            const response = await fetch(
                `${API_BASE}/api/calc/wall-stress?wall_thickness=${wall.thickness_m}` +
                `&wall_density=${wall.density_kgm3}` +
                `&wall_compressive_strength=${wall.compressive_strength_pa}` +
                `&wall_tensile_strength=${wall.tensile_strength_pa}`
            );
            const data = await response.json();
            if (data.success && data.data) {
                this.renderFEAResult(data.data);
                document.getElementById('feaPopup').classList.add('active');
                return;
            }
        } catch (e) {}
        const localResult = this.calcLocalFEA(wall);
        this.renderFEAResult(localResult);
        document.getElementById('feaPopup').classList.add('active');
    }

    calcLocalFEA(wall) {
        const nx = 20, ny = 15;
        const w = 30, h = 10;
        const stress_field = [];
        const damage_field = [];
        const maxStress = wall.density_kgm3 * 9.81 * h * wall.thickness_m * 0.5;
        for (let i = 0; i < nx; i++) {
            stress_field[i] = [];
            damage_field[i] = [];
            for (let j = 0; j < ny; j++) {
                const cy = (j + 0.5) * h / ny;
                const cx = (i + 0.5) * w / nx;
                const gravity = wall.density_kgm3 * 9.81 * (h - cy) * wall.thickness_m * 0.5;
                const gateFactor = Math.abs(cx - w / 2) < 2 ? 1.5 : 1.0;
                stress_field[i][j] = gravity * gateFactor;
                damage_field[i][j] = Math.min(1, Math.pow(gravity * gateFactor / wall.compressive_strength_pa, 3) * 0.1);
            }
        }
        return { mesh: { width_m: w, height_m: h, thickness_m: wall.thickness_m, nx, ny }, max_stress_pa: maxStress, min_safety_factor: wall.compressive_strength_pa / maxStress, weak_points: [{ x_m: w / 2, y_m: h * 0.3, stress_pa: maxStress * 1.5, safety_factor: wall.compressive_strength_pa / (maxStress * 1.5), priority: 0.9 }], stress_field, damage_field };
    }

    renderFEAResult(result) {
        const canvas = document.getElementById('feaHeatmap');
        const ctx = canvas.getContext('2d');
        const mesh = result.mesh;
        canvas.width = 600;
        canvas.height = 200;

        const cellW = canvas.width / mesh.nx;
        const cellH = canvas.height / mesh.ny;

        let maxS = 0;
        for (let i = 0; i < mesh.nx; i++) for (let j = 0; j < mesh.ny; j++) maxS = Math.max(maxS, result.stress_field[i][j]);

        for (let i = 0; i < mesh.nx; i++) {
            for (let j = 0; j < mesh.ny; j++) {
                const ratio = result.stress_field[i][j] / (maxS || 1);
                const r = Math.floor(255 * ratio);
                const g = Math.floor(255 * (1 - ratio) * 0.8);
                const b = Math.floor(50 * (1 - ratio));
                ctx.fillStyle = `rgb(${r},${g},${b})`;
                ctx.fillRect(i * cellW, canvas.height - (j + 1) * cellH, cellW + 1, cellH + 1);
            }
        }

        if (result.weak_points) {
            result.weak_points.forEach(wp => {
                const px = (wp.x_m / mesh.width_m) * canvas.width;
                const py = canvas.height - (wp.y_m / mesh.height_m) * canvas.height;
                ctx.strokeStyle = '#ffd700';
                ctx.lineWidth = 2;
                ctx.beginPath();
                ctx.arc(px, py, 8, 0, Math.PI * 2);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(px - 5, py);
                ctx.lineTo(px + 5, py);
                ctx.moveTo(px, py - 5);
                ctx.lineTo(px, py + 5);
                ctx.stroke();
            });
        }

        document.getElementById('feaInfo').innerHTML = `
            <div class="stat-card"><span class="label">最大应力</span><span class="value">${(result.max_stress_pa / 1e6).toFixed(2)} MPa</span></div>
            <div class="stat-card"><span class="label">最小安全系数</span><span class="value">${result.min_safety_factor.toFixed(2)}</span></div>
            <div class="stat-card"><span class="label">弱点数量</span><span class="value">${(result.weak_points || []).length}</span></div>
        `;

        let wpHtml = '<h4>弱点列表</h4>';
        (result.weak_points || []).slice(0, 5).forEach(wp => {
            const cls = wp.safety_factor < 1 ? 'priority-high' : wp.safety_factor < 2 ? 'priority-medium' : 'priority-low';
            wpHtml += `<div class="weak-point-item">
                <span>位置: (${wp.x_m.toFixed(1)}, ${wp.y_m.toFixed(1)}) m</span>
                <span>安全系数: ${wp.safety_factor.toFixed(2)}</span>
                <span class="priority-badge ${cls}">${wp.safety_factor < 1 ? '危险' : wp.safety_factor < 2 ? '警告' : '安全'}</span>
            </div>`;
        });
        document.getElementById('feaWeakPoints').innerHTML = wpHtml;
    }

    async onFindWeakPoint() {
        const tData = this.scene.findTrebuchetDataById(this.currentTrebuchetId);
        const wall = this.wallTypes.find(w => w.id === this.currentWallId);
        if (!tData || !wall) return;
        try {
            const impactE = 0.5 * tData.projectile_kg * this.params.velocity * this.params.velocity;
            const response = await fetch(
                `${API_BASE}/api/calc/weak-point?projectile_mass=${tData.projectile_kg}` +
                `&impact_energy=${impactE}` +
                `&wall_thickness=${wall.thickness_m}` +
                `&wall_density=${wall.density_kgm3}` +
                `&wall_compressive_strength=${wall.compressive_strength_pa}` +
                `&ammo_type=${this.currentAmmoType}`
            );
            const data = await response.json();
            if (data.success && data.data) {
                this.renderGAResult(data.data);
                document.getElementById('gaPopup').classList.add('active');
                return;
            }
        } catch (e) {}
        this.renderGAResult({ best: { x_m: 15.0, y_m: 3.0, fitness: 75.0 }, convergence_data: Array.from({length: 50}, (_, i) => i * 1.5), total_generations: 50 });
        document.getElementById('gaPopup').classList.add('active');
    }

    renderGAResult(result) {
        const canvas = document.getElementById('gaConvergence');
        const ctx = canvas.getContext('2d');
        canvas.width = 600;
        canvas.height = 200;

        ctx.fillStyle = '#1e293b';
        ctx.fillRect(0, 0, canvas.width, canvas.height);

        const data = result.convergence_data || [];
        if (data.length > 1) {
            const maxV = Math.max(...data);
            ctx.strokeStyle = '#d4af37';
            ctx.lineWidth = 2;
            ctx.beginPath();
            data.forEach((v, i) => {
                const x = (i / (data.length - 1)) * canvas.width;
                const y = canvas.height - (v / (maxV || 1)) * (canvas.height - 20) - 10;
                if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
            });
            ctx.stroke();
        }

        const best = result.best || {};
        document.getElementById('gaResult').innerHTML = `
            <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:10px;">
                <div class="result-card"><div class="big-value">${(best.x_m || 0).toFixed(1)} m</div><div class="label">最佳打击 X</div></div>
                <div class="result-card"><div class="big-value">${(best.y_m || 0).toFixed(1)} m</div><div class="label">最佳打击 Y</div></div>
                <div class="result-card"><div class="big-value">${(best.fitness || 0).toFixed(1)}</div><div class="label">适应度得分</div></div>
            </div>
            <p style="text-align:center;color:#64748b;font-size:11px;margin-top:10px;">
                经 ${result.total_generations || 0} 代遗传算法搜索，建议打击城墙坐标 (${(best.x_m || 0).toFixed(1)}, ${(best.y_m || 0).toFixed(1)}) 处
            </p>
        `;
    }

    async onCoordinate() {
        const wall = this.wallTypes.find(w => w.id === this.currentWallId);
        if (!wall) return;
        try {
            const response = await fetch(
                `${API_BASE}/api/calc/coordinate?wall_thickness=${wall.thickness_m}` +
                `&wall_density=${wall.density_kgm3}` +
                `&wall_compressive_strength=${wall.compressive_strength_pa}`
            );
            const data = await response.json();
            if (data.success && data.data) {
                this.renderCoordinateResult(data.data);
                document.getElementById('coordinatePopup').classList.add('active');
                return;
            }
        } catch (e) {}
        const localResult = { assignments: (this.trebuchetData || MOCK_TREBUCHETS).slice(0, 5).map((t, i) => ({ trebuchet_id: t.id, target_x_m: 12 + i * 3, target_y_m: 5, ammo_type: 'RoundStone', expected_damage: 0.1 + i * 0.02, priority: 1.0 - i * 0.1 })), expected_total_damage: 0.7, coordination_efficiency: 0.14, q_table_size: 0, episodes_trained: 0 };
        this.renderCoordinateResult(localResult);
        document.getElementById('coordinatePopup').classList.add('active');
    }

    renderCoordinateResult(result) {
        const names = { RoundStone: '圆石弹', GunpowderBomb: '火药弹', CorpseShell: '腐尸弹' };
        let html = `<div style="margin-bottom:12px;">
            <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;">
                <div class="stat-card"><span class="label">预期总伤害</span><span class="value">${(result.expected_total_damage * 100).toFixed(1)}%</span></div>
                <div class="stat-card"><span class="label">协同效率</span><span class="value">${(result.coordination_efficiency * 100).toFixed(1)}%</span></div>
            </div>
        </div>`;
        html += '<h4 style="font-size:12px;color:#94a3b8;margin-bottom:8px;">射击分配方案</h4>';
        (result.assignments || []).forEach(a => {
            const t = (this.trebuchetData || MOCK_TREBUCHETS).find(t => t.id === a.trebuchet_id);
            html += `<div class="coordinate-assignment">
                <span class="t-name">${t ? t.name : '#' + a.trebuchet_id}</span>
                <span class="t-target">目标: (${a.target_x_m.toFixed(1)}, ${a.target_y_m.toFixed(1)}) m</span>
                <span class="t-damage">${(a.expected_damage * 100).toFixed(1)}%</span>
            </div>`;
        });
        html += `<p style="text-align:center;color:#64748b;font-size:11px;margin-top:10px;">
            Q表大小: ${result.q_table_size} | 训练轮次: ${result.episodes_trained}
        </p>`;
        document.getElementById('coordinateBody').innerHTML = html;
    }

    async onBattle() {
        try {
            const response = await fetch(`${API_BASE}/api/battles`);
            const data = await response.json();
            if (data.success && data.data) {
                this.renderBattleList(data.data);
                document.getElementById('battlePopup').classList.add('active');
                return;
            }
        } catch (e) {}
        const localBattles = [
            { id: 1, name: '襄阳之战', year: 1267, description: '蒙古军队围攻南宋襄阳城', attacker: '蒙古帝国', defender: '南宋', attacker_trebuchets: [{ trebuchet_id: 1, name: '回回炮-甲', ammo_type: 'RoundStone', available_ammo: 200 }] },
            { id: 2, name: '君士坦丁堡之围', year: 1453, description: '奥斯曼帝国围攻君士坦丁堡', attacker: '奥斯曼帝国', defender: '拜占庭帝国', attacker_trebuchets: [{ trebuchet_id: 8, name: '无敌砲', ammo_type: 'RoundStone', available_ammo: 300 }] },
            { id: 3, name: '太原攻防战', year: 979, description: '北宋太宗亲征北汉太原城', attacker: '北宋', defender: '北汉', attacker_trebuchets: [{ trebuchet_id: 4, name: '人力砲-一号', ammo_type: 'RoundStone', available_ammo: 300 }] },
        ];
        this.renderBattleList(localBattles);
        document.getElementById('battlePopup').classList.add('active');
    }

    renderBattleList(battles) {
        let html = '';
        battles.forEach(b => {
            html += `<div class="battle-card" onclick="window.sim.panel.startBattle(${b.id})">
                <div class="battle-name">${b.name}</div>
                <div class="battle-year">${b.year}年</div>
                <div class="battle-desc">${b.description}</div>
            </div>`;
        });
        document.getElementById('battleList').innerHTML = html;
        document.getElementById('battleUI').style.display = 'none';
        document.getElementById('battleList').style.display = '';
    }

    async startBattle(id) {
        try {
            const response = await fetch(`${API_BASE}/api/battles/${id}/start`);
            const data = await response.json();
            if (data.success && data.data) {
                this.battleState = data.data;
                try {
                    const infoResp = await fetch(`${API_BASE}/api/battles/${id}`);
                    const infoData = await infoResp.json();
                    if (infoData.success && infoData.data) this.battleScenario = infoData.data;
                } catch(e) {}
                this.renderBattleUI();
                return;
            }
        } catch (e) {}
        this.battleState = { scenario_id: id, current_day: 1, wall_damage: 0, total_impacts: 0, successful_hits: 0, ammo_remaining: { 1: 200 }, is_victory: false, is_defeat: false, score: 0, impact_log: [] };
        this.battleScenario = { id, name: '战役', attacker_trebuchets: [{ trebuchet_id: 1, name: '回回炮-甲', available_ammo: 200 }], victory_conditions: { wall_breach_required: 0.6 } };
        this.renderBattleUI();
    }

    renderBattleUI() {
        document.getElementById('battleList').style.display = 'none';
        document.getElementById('battleUI').style.display = '';
        const s = this.battleState;
        const sc = this.battleScenario;
        const damagePct = Math.min(100, s.wall_damage * 100).toFixed(1);
        const victoryPct = sc ? (sc.victory_conditions ? sc.victory_conditions.wall_breach_required * 100 : 60) : 60;

        document.getElementById('battleInfo').innerHTML = `
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

        const trebs = sc ? sc.attacker_trebuchets : [];
        let controlsHtml = '';
        trebs.forEach(t => {
            const ammo = s.ammo_remaining[t.trebuchet_id] || 0;
            controlsHtml += `<button class="btn-primary" style="font-size:10px;padding:6px 10px;" 
                onclick="window.sim.panel.battleFire(${t.trebuchet_id})" ${ammo <= 0 || s.is_victory || s.is_defeat ? 'disabled' : ''}>
                ${t.name} (${ammo})
            </button>`;
        });
        controlsHtml += `<button class="btn-secondary" style="font-size:10px;padding:6px 10px;" onclick="window.sim.panel.battleAdvanceDay()">⏩ 下一天</button>`;
        document.getElementById('battleControls').innerHTML = controlsHtml;

        let logHtml = '';
        (s.impact_log || []).slice(-20).reverse().forEach(l => {
            logHtml += `<div class="log-entry hit">Day${l.day} | #${l.trebuchet_id} → (${l.target_x.toFixed(1)},${l.target_y.toFixed(1)}) | 损伤 ${(l.damage_ratio * 100).toFixed(1)}%</div>`;
        });
        document.getElementById('battleLog').innerHTML = logHtml || '<div class="log-entry">等待发射...</div>';
    }

    async battleFire(trebuchetId) {
        if (!this.battleState || this.battleState.is_victory || this.battleState.is_defeat) return;
        const ammo = this.battleState.ammo_remaining[trebuchetId] || 0;
        if (ammo <= 0) return;
        const targetX = 10 + Math.random() * 10;
        const targetY = 2 + Math.random() * 6;
        const damage = 0.02 + Math.random() * 0.08;
        try {
            const response = await fetch(
                `${API_BASE}/api/battles/${this.battleState.scenario_id}/fire?trebuchet_id=${trebuchetId}` +
                `&target_x=${targetX}&target_y=${targetY}&damage_ratio=${damage}` +
                `&ammo_type=${this.currentAmmoType}`
            );
            const data = await response.json();
            if (data.success && data.data) {
                this.battleState = data.data;
                this.renderBattleUI();
                return;
            }
        } catch (e) {}
        this.battleState.total_impacts += 1;
        this.battleState.successful_hits += 1;
        this.battleState.wall_damage += damage;
        if (this.battleState.ammo_remaining[trebuchetId]) this.battleState.ammo_remaining[trebuchetId] -= 1;
        this.battleState.impact_log.push({ day: this.battleState.current_day, trebuchet_id: trebuchetId, target_x: targetX, target_y: targetY, ammo_type: 'RoundStone', damage_ratio: damage });
        if (this.battleState.wall_damage >= 0.6) this.battleState.is_victory = true;
        this.renderBattleUI();
    }

    async battleAdvanceDay() {
        if (!this.battleState || this.battleState.is_victory || this.battleState.is_defeat) return;
        try {
            const response = await fetch(`${API_BASE}/api/battles/${this.battleState.scenario_id}/advance`);
            const data = await response.json();
            if (data.success && data.data) {
                this.battleState = data.data;
                this.renderBattleUI();
                return;
            }
        } catch (e) {}
        this.battleState.current_day += 1;
        if (this.battleState.current_day > 30 && !this.battleState.is_victory) this.battleState.is_defeat = true;
        this.renderBattleUI();
    }

    showTrebuchetInfo(id) {
        const t = this.trebuchetData ? this.trebuchetData.find(t => t.id === id)
            : this.scene.findTrebuchetDataById(id);
        if (!t) return;

        document.getElementById('popupTitle').textContent = t.name;

        let ballisticsInfo = '';
        let siegeInfo = '';

        if (this.ballisticsResult && this.currentTrebuchetId === id) {
            const r = this.ballisticsResult;
            ballisticsInfo = `
                <div class="popup-section">
                    <h4>弹道参数</h4>
                    <div class="detail-row"><span class="label">射程</span><span class="value">${r.range_m.toFixed(2)} m</span></div>
                    <div class="detail-row"><span class="label">最大高度</span><span class="value">${r.max_height_m.toFixed(2)} m</span></div>
                    <div class="detail-row"><span class="label">飞行时间</span><span class="value">${r.flight_time_s.toFixed(2)} s</span></div>
                    <div class="detail-row"><span class="label">着速</span><span class="value">${r.impact_velocity_mps.toFixed(2)} m/s</span></div>
                    <div class="detail-row"><span class="label">冲击动能</span><span class="value">${r.impact_kinetic_energy_j.toFixed(0)} J</span></div>
                </div>
            `;
        }

        if (this.siegeResult && this.currentTrebuchetId === id) {
            const s = this.siegeResult;
            siegeInfo = `
                <div class="popup-section">
                    <h4>攻城效能</h4>
                    <div class="detail-row"><span class="label">弹坑深度</span><span class="value">${s.crater_depth_m.toFixed(3)} m</span></div>
                    <div class="detail-row"><span class="label">弹坑直径</span><span class="value">${s.crater_diameter_m.toFixed(2)} m</span></div>
                    <div class="detail-row"><span class="label">损伤率</span><span class="value">${(s.damage_ratio * 100).toFixed(1)} %</span></div>
                    <div class="detail-row"><span class="label">效能评分</span><span class="value">${s.effectiveness_score.toFixed(1)}/100</span></div>
                    <div class="detail-row"><span class="label">破坏等级</span><span class="value">${s.structural_damage || '--'}</span></div>
                </div>
            `;
        }

        document.getElementById('popupBody').innerHTML = `
            <div class="popup-section">
                <h4>基本信息</h4>
                <div class="detail-row"><span class="label">类型</span><span class="value">${t.type_ || t.type}</span></div>
                <div class="detail-row"><span class="label">配重</span><span class="value">${t.counterweight_kg} kg</span></div>
                <div class="detail-row"><span class="label">弹丸质量</span><span class="value">${t.projectile_kg} kg</span></div>
                <div class="detail-row"><span class="label">臂长</span><span class="value">${t.arm_length_m} m</span></div>
                <div class="detail-row"><span class="label">最大发射角</span><span class="value">${t.max_angle_deg}°</span></div>
            </div>
            ${ballisticsInfo}
            ${siegeInfo}
        `;
        document.getElementById('infoPopup').classList.add('active');
    }
}

class SiegeSimulation {
    constructor() {
        this.scene = new SceneManager('canvasContainer');
        this.particleSystem = new TrajectoryParticles(this.scene.scene);
        this.panel = new BallisticPanelController(this.scene, this.particleSystem);
        this.animate();
    }

    animate() {
        requestAnimationFrame(() => this.animate());
        this.scene.render(this.particleSystem);
    }
}

window.addEventListener('DOMContentLoaded', () => {
    window.sim = new SiegeSimulation();
});
