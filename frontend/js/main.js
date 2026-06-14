class SiegeSimulation {
    constructor() {
        this.scene = null;
        this.camera = null;
        this.renderer = null;
        this.controls = null;
        this.clock = null;

        this.trebuchets = [];
        this.currentTrebuchetId = 1;
        this.particleSystem = null;

        this.wallTypes = [];
        this.currentWallId = 1;

        this.ballisticsResult = null;
        this.siegeResult = null;

        this.apiBase = "http://localhost:8080";

        this.params = {
            velocity: 50,
            angle: 45,
            windSpeed: 0,
            windDirection: 0,
        };

        this.raycaster = new THREE.Raycaster();
        this.mouse = new THREE.Vector2();

        this.init();
    }

    init() {
        const container = document.getElementById('canvasContainer');

        this.scene = new THREE.Scene();
        this.scene.background = new THREE.Color(0x1a2332);
        this.scene.fog = new THREE.Fog(0x1a2332, 50, 200);

        this.camera = new THREE.PerspectiveCamera(
            60,
            container.clientWidth / container.clientHeight,
            0.1,
            1000
        );
        this.camera.position.set(30, 25, 40);

        this.renderer = new THREE.WebGLRenderer({ antialias: true });
        this.renderer.setSize(container.clientWidth, container.clientHeight);
        this.renderer.shadowMap.enabled = true;
        this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
        container.appendChild(this.renderer.domElement);

        if (THREE.OrbitControls) {
            this.controls = new THREE.OrbitControls(this.camera, this.renderer.domElement);
            this.controls.enableDamping = true;
            this.controls.dampingFactor = 0.05;
            this.controls.maxPolarAngle = Math.PI / 2 - 0.1;
        }

        this.clock = new THREE.Clock();

        this.setupLighting();
        this.createGround();
        this.createWall();

        this.particleSystem = new TrajectoryParticles(this.scene);

        this.setupEventListeners();
        this.loadData();

        this.animate();
    }

    setupLighting() {
        const ambientLight = new THREE.AmbientLight(0x404050, 0.5);
        this.scene.add(ambientLight);

        const sunLight = new THREE.DirectionalLight(0xfff5e6, 1);
        sunLight.position.set(50, 80, 30);
        sunLight.castShadow = true;
        sunLight.shadow.mapSize.width = 2048;
        sunLight.shadow.mapSize.height = 2048;
        sunLight.shadow.camera.near = 0.5;
        sunLight.shadow.camera.far = 200;
        sunLight.shadow.camera.left = -80;
        sunLight.shadow.camera.right = 80;
        sunLight.shadow.camera.top = 80;
        sunLight.shadow.camera.bottom = -80;
        this.scene.add(sunLight);

        const fillLight = new THREE.DirectionalLight(0x8899aa, 0.3);
        fillLight.position.set(-30, 20, -20);
        this.scene.add(fillLight);

        const hemiLight = new THREE.HemisphereLight(0x87ceeb, 0x3d5c3d, 0.3);
        this.scene.add(hemiLight);
    }

    createGround() {
        const groundGeo = new THREE.PlaneGeometry(200, 200, 50, 50);
        const groundMat = new THREE.MeshStandardMaterial({
            color: 0x4a5568,
            roughness: 0.9,
            metalness: 0.1,
        });

        const positions = groundGeo.attributes.position;
        for (let i = 0; i < positions.count; i++) {
            const x = positions.getX(i);
            const y = positions.getY(i);
            const z = Math.sin(x * 0.05) * Math.cos(y * 0.05) * 0.5;
            positions.setZ(i, z);
        }
        groundGeo.computeVertexNormals();

        const ground = new THREE.Mesh(groundGeo, groundMat);
        ground.rotation.x = -Math.PI / 2;
        ground.receiveShadow = true;
        this.scene.add(ground);

        const gridHelper = new THREE.GridHelper(200, 50, 0x2d3748, 0x252d3d);
        gridHelper.position.y = 0.01;
        this.scene.add(gridHelper);
    }

    createWall() {
        const wallGroup = new THREE.Group();
        wallGroup.name = "wall";

        const wallHeight = 10;
        const wallWidth = 30;
        const wallThickness = 3;

        const wallGeo = new THREE.BoxGeometry(wallWidth, wallHeight, wallThickness);
        const wallMat = new THREE.MeshStandardMaterial({
            color: 0x8b7355,
            roughness: 0.9,
            metalness: 0.1,
        });

        const wall = new THREE.Mesh(wallGeo, wallMat);
        wall.position.y = wallHeight / 2;
        wall.position.z = -50;
        wall.castShadow = true;
        wall.receiveShadow = true;
        wallGroup.add(wall);

        const towerGeo = new THREE.BoxGeometry(5, wallHeight + 3, 5);
        const tower1 = new THREE.Mesh(towerGeo, wallMat);
        tower1.position.y = (wallHeight + 3) / 2;
        tower1.position.x = -wallWidth / 2 - 1;
        tower1.position.z = -50;
        tower1.castShadow = true;
        wallGroup.add(tower1);

        const tower2 = tower1.clone();
        tower2.position.x = wallWidth / 2 + 1;
        wallGroup.add(tower2);

        const battlementsCount = 10;
        const battlementGeo = new THREE.BoxGeometry(1.5, 1.5, wallThickness + 0.5);
        for (let i = 0; i < battlementsCount; i++) {
            const battlement = new THREE.Mesh(battlementGeo, wallMat);
            battlement.position.y = wallHeight + 0.75;
            battlement.position.x = -wallWidth / 2 + 1.5 + i * (wallWidth - 3) / (battlementsCount - 1);
            battlement.position.z = -50;
            battlement.castShadow = true;
            wallGroup.add(battlement);
        }

        this.scene.add(wallGroup);
        this.wall = wallGroup;
    }

    createTrebuchets(trebuchetData) {
        for (const t of this.trebuchets) {
            t.removeFromScene();
        }
        this.trebuchets = [];

        const spacing = 12;
        const startX = -((trebuchetData.length - 1) * spacing) / 2;

        trebuchetData.forEach((data, index) => {
            const trebuchet = new TrebuchetModel(this.scene, {
                ...data,
                position: { x: startX + index * spacing, y: 0, z: 20 },
                rotation: 0,
                scale: 0.8,
            });
            trebuchet.addToScene();
            this.trebuchets.push(trebuchet);
        });

        this.updateTrebuchetData = trebuchetData;
    }

    setupEventListeners() {
        window.addEventListener('resize', () => this.onWindowResize());

        document.getElementById('fireBtn').addEventListener('click', () => this.fire());
        document.getElementById('optimizeBtn').addEventListener('click', () => this.optimize());

        const velocitySlider = document.getElementById('velocitySlider');
        const velocityValue = document.getElementById('velocityValue');
        velocitySlider.addEventListener('input', (e) => {
            this.params.velocity = parseFloat(e.target.value);
            velocityValue.textContent = e.target.value;
        });

        const angleSlider = document.getElementById('angleSlider');
        const angleValue = document.getElementById('angleValue');
        angleSlider.addEventListener('input', (e) => {
            this.params.angle = parseFloat(e.target.value);
            angleValue.textContent = e.target.value;
            this.updateTrebuchetAngle();
        });

        const windSlider = document.getElementById('windSlider');
        const windValue = document.getElementById('windValue');
        windSlider.addEventListener('input', (e) => {
            this.params.windSpeed = parseFloat(e.target.value);
            windValue.textContent = e.target.value;
        });

        document.getElementById('viewTop').addEventListener('click', () => this.setView('top'));
        document.getElementById('viewSide').addEventListener('click', () => this.setView('side'));
        document.getElementById('view3d').addEventListener('click', () => this.setView('3d'));
        document.getElementById('viewReset').addEventListener('click', () => this.setView('reset'));

        document.getElementById('popupClose').addEventListener('click', () => {
            document.getElementById('infoPopup').classList.remove('active');
        });

        this.renderer.domElement.addEventListener('click', (e) => this.onCanvasClick(e));
    }

    async loadData() {
        try {
            const response = await fetch(`${this.apiBase}/api/trebuchets');
            const data = await response.json();
            if (data.success && data.data) {
                this.trebuchetData = data.data;
                this.createTrebuchets(data.data);
                this.renderTrebuchetList(data.data);
                this.updateConnectionStatus(true);
            }
        } catch (e) {
            console.warn('Failed to load trebuchets from API, using mock data');
            this.updateConnectionStatus(false);
            this.loadMockData();
        }

        try {
            const response = await fetch(`${this.apiBase}/api/walls`);
            const data = await response.json();
            if (data.success && data.data) {
                this.wallTypes = data.data;
                this.renderWallList(data.data);
            }
        } catch (e) {
            this.loadMockWalls();
        }
    }

    loadMockData() {
        const mockTrebuchets = [
            { id: 1, name: "回回炮-甲", type_: "配重式", counterweight_kg: 3000, projectile_kg: 90, arm_length_m: 12.0, max_angle_deg: 50.0 },
            { id: 2, name: "回回炮-乙", type_: "配重式", counterweight_kg: 5000, projectile_kg: 150, arm_length_m: 15.0, max_angle_deg: 55.0 },
            { id: 3, name: "襄阳砲-壹", type_: "配重式", counterweight_kg: 4000, projectile_kg: 120, arm_length_m: 13.5, max_angle_deg: 52.0 },
            { id: 4, name: "人力砲-一号", type_: "人力牵引式", counterweight_kg: 0, projectile_kg: 30, arm_length_m: 8.0, max_angle_deg: 45.0 },
            { id: 5, name: "人力砲-二号", type_: "人力牵引式", counterweight_kg: 0, projectile_kg: 25, arm_length_m: 7.5, max_angle_deg: 42.0 },
            { id: 6, name: "旋风砲", type_: "人力牵引式", counterweight_kg: 0, projectile_kg: 20, arm_length_m: 6.0, max_angle_deg: 48.0 },
            { id: 7, name: "虎蹲砲", type_: "配重式", counterweight_kg: 1500, projectile_kg: 50, arm_length_m: 9.0, max_angle_deg: 47.0 },
            { id: 8, name: "无敌砲", type_: "配重式", counterweight_kg: 6000, projectile_kg: 200, arm_length_m: 18.0, max_angle_deg: 58.0 },
            { id: 9, name: "飞云砲", type_: "人力牵引式", counterweight_kg: 0, projectile_kg: 15, arm_length_m: 5.5, max_angle_deg: 40.0 },
            { id: 10, name: "震天雷砲", type_: "配重式", counterweight_kg: 8000, projectile_kg: 300, arm_length_m: 20.0, max_angle_deg: 60.0 },
        ];
        this.trebuchetData = mockTrebuchets;
        this.createTrebuchets(mockTrebuchets);
        this.renderTrebuchetList(mockTrebuchets);
    }

    loadMockWalls() {
        this.wallTypes = [
            { id: 1, name: "夯土墙", material: "rammed_earth", thickness_m: 3.0, density_kgm3: 1800, compressive_strength_pa: 2000000, tensile_strength_pa: 200000 },
            { id: 2, name: "包砖墙", material: "brick_veneer", thickness_m: 2.5, density_kgm3: 2000, compressive_strength_pa: 10000000, tensile_strength_pa: 800000 },
            { id: 3, name: "石砌墙", material: "stone_masonry", thickness_m: 4.0, density_kgm3: 2400, compressive_strength_pa: 25000000, tensile_strength_pa: 2000000 },
            { id: 4, name: "双层夯土墙", material: "double_rammed_earth", thickness_m: 6.0, density_kgm3: 1700, compressive_strength_pa: 1800000, tensile_strength_pa: 180000 },
            { id: 5, name: "糯米灰浆墙", material: "sticky_rice_lime", thickness_m: 3.5, density_kgm3: 2100, compressive_strength_pa: 15000000, tensile_strength_pa: 1200000 },
        ];
        this.renderWallList(this.wallTypes);
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
            item.addEventListener('click', () => {
                this.selectTrebuchet(t.id);
            });
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
            item.addEventListener('click', () => {
                this.selectWall(w.id);
            });
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
            document.getElementById('velocitySlider').max = t.max_angle_deg * 1.5;
            this.updateTrebuchetAngle();
        }
    }

    selectWall(id) {
        this.currentWallId = id;

        document.querySelectorAll('.wall-item').forEach((el, idx) => {
            el.classList.toggle('active', this.wallTypes[idx].id === id);
        });
    }

    updateTrebuchetAngle() {
        const trebuchet = this.trebuchets.find(t => t.id === this.currentTrebuchetId);
        if (trebuchet) {
            trebuchet.setArmAngle(this.params.angle);
        }
    }

    async fire() {
        this.particleSystem.clearAll();

        const trebuchet = this.trebuchets.find(t => t.id === this.currentTrebuchetId);
        if (!trebuchet) return;

        const tData = this.trebuchetData.find(t => t.id === this.currentTrebuchetId);
        if (!tData) return;

        trebuchet.animateFire(500);

        setTimeout(async () => {
            try {
                const response = await fetch(
                    `${this.apiBase}/api/calc/ballistics?velocity=${this.params.velocity}` +
                    `&angle=${this.params.angle}&mass=${tData.projectile_kg}` +
                    `&wind_speed=${this.params.windSpeed}&wind_direction=0`
                );
                const data = await response.json();
                if (data.success && data.data) {
                    this.ballisticsResult = data.data;
                    this.showTrajectory(data.data, tData);
                    this.updateHUD(data.data);
                    await this.calculateSiege(data.data, tData);
                }
            } catch (e) {
                const trajectory = this.calculateLocalTrajectory(tData);
                this.showTrajectory(trajectory, tData);
                this.updateHUD(trajectory);
                this.calculateLocalSiege(trajectory, tData);
            }
        }, 300);
    }

    calculateLocalTrajectory(tData) {
        const g = 9.81;
        const angleRad = this.params.angle * Math.PI / 180;
        const v0 = this.params.velocity;
        const points = [];
        const dt = 0.05;
        let t = 0;

        while (true) {
            const x = v0 * Math.cos(angleRad) * t;
            const y = 5 + v0 * Math.sin(angleRad) * t - 0.5 * g * t * t;

            if (y < 0) break;
            if (t > 30) break;

            const velocity = Math.sqrt(
                Math.pow(v0 * Math.cos(angleRad), 2) +
                Math.pow(v0 * Math.sin(angleRad) - g * t, 2)
            );

            points.push({ x: x, y: y, z: 0, velocity, time_s: t });
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

    calculateLocalSiege(ballistics, tData) {
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

    classifyDamage(ratio) {
        if (ratio >= 0.9) return "完全摧毁";
        if (ratio >= 0.7) return "严重破坏";
        if (ratio >= 0.5) return "中等破坏";
        if (ratio >= 0.3) return "轻度破坏";
        if (ratio >= 0.1) return "表面损伤";
        return "无明显损伤";
    }

    showTrajectory(result, tData) {
        const trebuchet = this.trebuchets.find(t => t.id === this.currentTrebuchetId);
        if (!trebuchet || !result.trajectory) return;

        const startPos = trebuchet.group.position;

        const adjustedPoints = result.trajectory.map(p => ({
            x: startPos.x + p.x * Math.sin(trebuchet.rotation),
            y: startPos.y + p.y + 2,
            z: startPos.z - p.x * Math.cos(trebuchet.rotation),
        }));

        this.particleSystem.createTrajectoryLine(adjustedPoints, {
            color: 0xffd700,
            speed: 0.8,
            onComplete: () => {
                console.log('Impact!');
            }
        });

        this.particleSystem.createTrajectory(adjustedPoints, {
            color: 0xffa500,
            particleSize: 0.4,
            speed: 0.8,
        });
    }

    async calculateSiege(ballistics, tData) {
        const wall = this.wallTypes.find(w => w.id === this.currentWallId);
        if (!wall) return;

        try {
            const response = await fetch(
                `${this.apiBase}/api/calc/siege?impact_energy=${ballistics.impact_kinetic_energy_j}` +
                `&projectile_mass=${tData.projectile_kg}` +
                `&projectile_diameter=0.4` +
                `&impact_angle=${ballistics.impact_angle_deg}` +
                `&wall_thickness=${wall.thickness_m}` +
                `&wall_density=${wall.density_kgm3}` +
                `&wall_compressive_strength=${wall.compressive_strength_pa}` +
                `&wall_tensile_strength=${wall.tensile_strength_pa}`
            );
            const data = await response.json();
            if (data.success && data.data) {
                this.siegeResult = data.data;
                this.updateSiegeHUD(data.data);
            }
        } catch (e) {
            this.calculateLocalSiege(ballistics, tData);
        }
    }

    async optimize() {
        const tData = this.trebuchetData.find(t => t.id === this.currentTrebuchetId);
        const wall = this.wallTypes.find(w => w.id === this.currentWallId);
        if (!tData || !wall) return;

        try {
            const response = await fetch(
                `${this.apiBase}/api/calc/optimize?projectile_mass=${tData.projectile_kg}` +
                `&wall_thickness=${wall.thickness_m}` +
                `&wall_density=${wall.density_kgm3}` +
                `&wall_compressive_strength=${wall.compressive_strength_pa}` +
                `&min_velocity=20&max_velocity=80` +
                `&min_angle=30&max_angle=60`
            );
            const data = await response.json();
            if (data.success && data.data) {
                document.getElementById('angleSlider').value = data.data.optimal_angle_deg.toFixed(1);
                document.getElementById('angleValue').textContent = data.data.optimal_angle_deg.toFixed(1);
                document.getElementById('velocitySlider').value = data.data.optimal_velocity_mps.toFixed(1);
                document.getElementById('velocityValue').textContent = data.data.optimal_velocity_mps.toFixed(1);
                this.params.angle = data.data.optimal_angle_deg;
                this.params.velocity = data.data.optimal_velocity_mps;
                this.updateTrebuchetAngle();
            }
        } catch (e) {
            console.log('Optimize not available offline');
        }
    }

    updateHUD(result) {
        document.getElementById('hudRange').textContent = result.range_m.toFixed(1) + ' m';
        document.getElementById('hudHeight').textContent = result.max_height_m.toFixed(1) + ' m';
        document.getElementById('hudTime').textContent = result.flight_time_s.toFixed(2) + ' s';
        document.getElementById('hudImpactVel').textContent = result.impact_velocity_mps.toFixed(1) + ' m/s';
        document.getElementById('hudEnergy').textContent =
            (result.impact_kinetic_energy_j / 1000).toFixed(1) + ' kJ';
    }

    updateSiegeHUD(result) {
        document.getElementById('hudCrater').textContent = result.crater_depth_m.toFixed(2) + ' m';
        document.getElementById('hudDamage').textContent = (result.damage_ratio * 100).toFixed(1) + ' %';
        document.getElementById('hudScore').textContent = result.effectiveness_score.toFixed(1);
        document.getElementById('damageFill').style.width = (result.damage_ratio * 100) + '%';
        document.getElementById('damageLabel').textContent = result.structural_damage || '--';
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

    setView(view) {
        if (!this.controls) return;

        switch (view) {
            case 'top':
                this.camera.position.set(0, 80, 0.1);
                break;
            case 'side':
                this.camera.position.set(60, 10, 0);
                break;
            case '3d':
                this.camera.position.set(30, 25, 40);
                break;
            case 'reset':
                this.camera.position.set(30, 25, 40);
                this.controls.target.set(0, 5, 0);
                break;
        }

        this.controls.update();
    }

    onCanvasClick(event) {
        const rect = this.renderer.domElement.getBoundingClientRect();
        this.mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
        this.mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;

        this.raycaster.setFromCamera(this.mouse, this.camera);

        const meshes = [];
        this.trebuchets.forEach(t => {
            t.group.traverse(child => {
                if (child.isMesh) {
                    meshes.push(child);
                }
            });
        });

        const intersects = this.raycaster.intersectObjects(meshes);

        if (intersects.length > 0) {
            let obj = intersects[0].object;
            while (obj.parent && !obj.userData.type) {
                obj = obj.parent;
            }
            if (obj.userData && obj.userData.type === 'trebuchet') {
                this.showTrebuchetInfo(obj.userData.id);
            }
        }
    }

    showTrebuchetInfo(id) {
        const t = this.trebuchetData.find(t => t.id === id);
        if (!t) return;

        document.getElementById('popupTitle').textContent = t.name;

        let ballisticsInfo = '';
        let siegeInfo = '';

        if (this.ballisticsResult && this.currentTrebuchetId === id) {
            ballisticsInfo = `
                <div class="popup-section">
                    <h4>弹道参数</h4>
                    <div class="detail-row"><span class="label">射程</span><span class="value">${this.ballisticsResult.range_m.toFixed(2)} m</span></div>
                    <div class="detail-row"><span class="label">最大高度</span><span class="value">${this.ballisticsResult.max_height_m.toFixed(2)} m</span></div>
                    <div class="detail-row"><span class="label">飞行时间</span><span class="value">${this.ballisticsResult.flight_time_s.toFixed(2)} s</span></div>
                    <div class="detail-row"><span class="label">着速</span><span class="value">${this.ballisticsResult.impact_velocity_mps.toFixed(2)} m/s</span></div>
                    <div class="detail-row"><span class="label">冲击动能</span><span class="value">${this.ballisticsResult.impact_kinetic_energy_j.toFixed(0)} J</span></div>
                </div>
            `;
        }

        if (this.siegeResult && this.currentTrebuchetId === id) {
            siegeInfo = `
                <div class="popup-section">
                    <h4>攻城效能</h4>
                    <div class="detail-row"><span class="label">弹坑深度</span><span class="value">${this.siegeResult.crater_depth_m.toFixed(3)} m</span></div>
                    <div class="detail-row"><span class="label">弹坑直径</span><span class="value">${this.siegeResult.crater_diameter_m.toFixed(2)} m</span></div>
                    <div class="detail-row"><span class="label">损伤率</span><span class="value">${(this.siegeResult.damage_ratio * 100).toFixed(1)} %</span></div>
                    <div class="detail-row"><span class="label">效能评分</span><span class="value">${this.siegeResult.effectiveness_score.toFixed(1)}/100</span></div>
                    <div class="detail-row"><span class="label">破坏等级</span><span class="value">${this.siegeResult.structural_damage || '--'}</span></div>
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

    onWindowResize() {
        const container = document.getElementById('canvasContainer');
        this.camera.aspect = container.clientWidth / container.clientHeight;
        this.camera.updateProjectionMatrix();
        this.renderer.setSize(container.clientWidth, container.clientHeight);
    }

    animate() {
        requestAnimationFrame(() => this.animate());

        const delta = this.clock.getDelta();

        if (this.controls) {
            this.controls.update();
        }

        this.particleSystem.update(delta);

        this.renderer.render(this.scene, this.camera);
    }
}

window.addEventListener('DOMContentLoaded', () => {
    window.sim = new SiegeSimulation();
});
