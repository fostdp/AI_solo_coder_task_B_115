const MATERIAL_PRESETS = {
    wood:      { color: 0x8B4513, roughness: 0.8, metalness: 0.1 },
    darkWood:  { color: 0x5D3A1A, roughness: 0.9, metalness: 0.1 },
    metal:     { color: 0x4a4a4a, roughness: 0.4, metalness: 0.8 },
    stone:     { color: 0x808080, roughness: 0.9, metalness: 0.1 },
    ground:    { color: 0x4a5568, roughness: 0.9, metalness: 0.1 },
    wall:      { color: 0x8b7355, roughness: 0.9, metalness: 0.1 },
};

const STONE_DENSITY_KGM3 = 2600;
const COUNTERWEIGHT_DENSITY_KGM3 = 7000;

class TrebuchetModel {
    constructor(scene, options = {}) {
        this.scene = scene;
        this.id = options.id || 1;
        this.name = options.name || "投石机";
        this.type = options.type || "配重式";
        this.counterweightKg = options.counterweight_kg || 3000;
        this.projectileKg = options.projectile_kg || 90;
        this.armLength = options.arm_length_m || 12;
        this.maxAngle = options.max_angle_deg || 50;
        this.position = options.position || { x: 0, y: 0, z: 0 };
        this.rotation = options.rotation || 0;
        this.scale = options.scale || 1;

        this.group = new THREE.Group();
        this.armGroup = new THREE.Group();
        this.projectile = null;
        this.currentAngle = 0;

        this.createModel();
    }

    createModel() {
        const scale = this.scale;

        const woodMat = new THREE.MeshStandardMaterial(MATERIAL_PRESETS.wood);
        const darkWoodMat = new THREE.MeshStandardMaterial(MATERIAL_PRESETS.darkWood);
        const metalMat = new THREE.MeshStandardMaterial(MATERIAL_PRESETS.metal);
        const stoneMat = new THREE.MeshStandardMaterial(MATERIAL_PRESETS.stone);

        const baseWidth = 4 * scale;
        const baseDepth = 3 * scale;
        const baseHeight = 0.3 * scale;

        const baseGeo = new THREE.BoxGeometry(baseWidth, baseHeight, baseDepth);
        const base = new THREE.Mesh(baseGeo, darkWoodMat);
        base.position.y = baseHeight / 2;
        base.castShadow = true;
        base.receiveShadow = true;
        this.group.add(base);

        const wheelRadius = 0.4 * scale;
        const wheelGeo = new THREE.CylinderGeometry(wheelRadius, wheelRadius, 0.15 * scale, 16);
        wheelGeo.rotateZ(Math.PI / 2);

        const wheelPositions = [
            [baseWidth / 2 - 0.2, wheelRadius, baseDepth / 2 - 0.2],
            [-baseWidth / 2 + 0.2, wheelRadius, baseDepth / 2 - 0.2],
            [baseWidth / 2 - 0.2, wheelRadius, -baseDepth / 2 + 0.2],
            [-baseWidth / 2 + 0.2, wheelRadius, -baseDepth / 2 + 0.2],
        ];

        wheelPositions.forEach(pos => {
            const wheel = new THREE.Mesh(wheelGeo, darkWoodMat);
            wheel.position.set(pos[0], pos[1], pos[2]);
            wheel.castShadow = true;
            this.group.add(wheel);
        });

        const frameHeight = 5 * scale;
        const frameWidth = 3 * scale;
        const poleRadius = 0.12 * scale;
        const poleGeo = new THREE.CylinderGeometry(poleRadius, poleRadius, frameHeight, 8);

        const framePositions = [
            [-frameWidth / 2, frameHeight / 2 + baseHeight, -0.5],
            [frameWidth / 2, frameHeight / 2 + baseHeight, -0.5],
            [-frameWidth / 2 + 0.5, frameHeight / 2 + baseHeight, 0.8],
            [frameWidth / 2 - 0.5, frameHeight / 2 + baseHeight, 0.8],
        ];

        framePositions.forEach(pos => {
            const pole = new THREE.Mesh(poleGeo, woodMat);
            pole.position.set(pos[0] * scale, pos[1] * scale, pos[2] * scale);
            pole.castShadow = true;
            this.group.add(pole);
        });

        const topBeamGeo = new THREE.BoxGeometry(frameWidth * scale, 0.15 * scale, 0.2 * scale);
        const topBeam = new THREE.Mesh(topBeamGeo, darkWoodMat);
        topBeam.position.y = (frameHeight + baseHeight) * scale;
        topBeam.position.z = 0.15 * scale;
        topBeam.castShadow = true;
        this.group.add(topBeam);

        const pivotY = (frameHeight + baseHeight) * scale;
        this.armGroup.position.y = pivotY;
        this.armGroup.position.z = 0.15 * scale;
        this.group.add(this.armGroup);

        const armLength = this.armLength * scale;
        const armThickness = 0.2 * scale;
        const armGeo = new THREE.BoxGeometry(armThickness, armThickness, armLength);
        const arm = new THREE.Mesh(armGeo, woodMat);
        arm.position.z = armLength * 0.3;
        arm.castShadow = true;
        this.armGroup.add(arm);

        const slingLength = 1.5 * scale;
        const slingGeo = new THREE.CylinderGeometry(0.02 * scale, 0.02 * scale, slingLength, 6);
        const sling1 = new THREE.Mesh(slingGeo, darkWoodMat);
        sling1.position.set(0.1 * scale, -slingLength / 2, armLength * 0.85);
        sling1.rotation.x = Math.PI / 2;
        this.armGroup.add(sling1);

        const sling2 = new THREE.Mesh(slingGeo, darkWoodMat);
        sling2.position.set(-0.1 * scale, -slingLength / 2, armLength * 0.85);
        sling2.rotation.x = Math.PI / 2;
        this.armGroup.add(sling2);

        const projDiameter = Math.cbrt(this.projectileKg / STONE_DENSITY_KGM3 / (Math.PI * 4 / 3)) * 2 * scale;
        const projGeo = new THREE.SphereGeometry(projDiameter / 2, 16, 16);
        this.projectile = new THREE.Mesh(projGeo, stoneMat);
        this.projectile.position.set(0, -slingLength, armLength * 0.85);
        this.projectile.castShadow = true;
        this.armGroup.add(this.projectile);

        if (this.type === "配重式") {
            const cwVolume = this.counterweightKg / COUNTERWEIGHT_DENSITY_KGM3;
            const cwSize = Math.cbrt(cwVolume) * scale;
            const cwGeo = new THREE.BoxGeometry(cwSize, cwSize, cwSize);
            const counterweight = new THREE.Mesh(cwGeo, stoneMat);
            counterweight.position.set(0, -cwSize / 2, -armLength * 0.2);
            counterweight.castShadow = true;
            this.armGroup.add(counterweight);

            const cwFrameGeo = new THREE.BoxGeometry(cwSize * 1.2, cwSize * 0.3, cwSize * 1.2);
            const cwFrame = new THREE.Mesh(cwFrameGeo, woodMat);
            cwFrame.position.set(0, -cwSize * 0.15, -armLength * 0.2);
            this.armGroup.add(cwFrame);
        } else {
            const ropeCount = 5;
            const ropeLength = 2 * scale;
            for (let i = 0; i < ropeCount; i++) {
                const ropeGeo = new THREE.CylinderGeometry(0.015 * scale, 0.015 * scale, ropeLength, 6);
                const rope = new THREE.Mesh(ropeGeo, darkWoodMat);
                rope.position.set(
                    (i - ropeCount / 2 + 0.5) * 0.15 * scale,
                    ropeLength / 2,
                    -armLength * 0.2
                );
                this.armGroup.add(rope);
            }
        }

        const winchGeo = new THREE.CylinderGeometry(0.3 * scale, 0.3 * scale, 0.4 * scale, 12);
        winchGeo.rotateZ(Math.PI / 2);
        const winch = new THREE.Mesh(winchGeo, woodMat);
        winch.position.set(0, baseHeight + 0.5 * scale, -baseDepth / 2 + 0.3 * scale);
        winch.castShadow = true;
        this.group.add(winch);

        const cableGeo = new THREE.CylinderGeometry(0.02 * scale, 0.02 * scale, frameHeight * 0.9, 6);
        const cable = new THREE.Mesh(cableGeo, metalMat);
        cable.position.set(0, frameHeight * 0.55 + baseHeight, -0.3 * scale);
        cable.rotation.x = -0.3;
        this.group.add(cable);

        this.group.position.set(this.position.x, this.position.y, this.position.z);
        this.group.rotation.y = this.rotation;

        this.group.userData = {
            type: "trebuchet",
            id: this.id,
            name: this.name,
        };

        this.setArmAngle(20);
    }

    setArmAngle(angleDeg) {
        this.currentAngle = angleDeg;
        this.armGroup.rotation.x = -angleDeg * Math.PI / 180;
    }

    getProjectileWorldPosition() {
        const pos = new THREE.Vector3();
        this.projectile.getWorldPosition(pos);
        return pos;
    }

    getProjectileInitialVelocity(angleDeg, speed) {
        const worldPos = this.getProjectileWorldPosition();
        const angleRad = (angleDeg + this.rotation * 180 / Math.PI) * Math.PI / 180;

        const vx = speed * Math.cos(angleRad * Math.PI / 180) * Math.sin(this.rotation);
        const vy = speed * Math.sin(angleDeg * Math.PI / 180);
        const vz = speed * Math.cos(angleDeg * Math.PI / 180) * Math.cos(this.rotation);

        return {
            position: { x: worldPos.x, y: worldPos.y, z: worldPos.z },
            velocity: { x: vx, y: vy, z: vz },
        };
    }

    animateFire(duration = 1000) {
        const startAngle = this.currentAngle;
        const endAngle = this.maxAngle;
        const startTime = performance.now();

        const animate = (currentTime) => {
            const elapsed = currentTime - startTime;
            const progress = Math.min(elapsed / duration, 1);

            const eased = 1 - Math.pow(1 - progress, 3);
            const angle = startAngle + (endAngle - startAngle) * eased;

            this.setArmAngle(angle);

            if (progress < 1) {
                requestAnimationFrame(animate);
            }
        };

        requestAnimationFrame(animate);
    }

    reset() {
        this.setArmAngle(20);
    }

    addToScene() {
        this.scene.add(this.group);
    }

    removeFromScene() {
        this.scene.remove(this.group);
    }
}

class SceneManager {
    constructor(containerId) {
        this.containerId = containerId;
        this.trebuchets = [];
        this.wall = null;
        this.trebuchetData = [];

        this.initScene();
    }

    initScene() {
        const container = document.getElementById(this.containerId);

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
        const groundMat = new THREE.MeshStandardMaterial(MATERIAL_PRESETS.ground);

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
        const wallMat = new THREE.MeshStandardMaterial(MATERIAL_PRESETS.wall);

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

        this.trebuchetData = trebuchetData;
    }

    findTrebuchetById(id) {
        return this.trebuchets.find(t => t.id === id);
    }

    findTrebuchetDataById(id) {
        return this.trebuchetData.find(t => t.id === id);
    }

    setTrebuchetArmAngle(id, angle) {
        const t = this.findTrebuchetById(id);
        if (t) t.setArmAngle(angle);
    }

    animateTrebuchetFire(id, duration) {
        const t = this.findTrebuchetById(id);
        if (t) t.animateFire(duration);
    }

    getRaycastTrebuchetMeshes() {
        const meshes = [];
        this.trebuchets.forEach(t => {
            t.group.traverse(child => {
                if (child.isMesh) meshes.push(child);
            });
        });
        return meshes;
    }

    resolveTrebuchetFromIntersect(intersectObject) {
        let obj = intersectObject;
        while (obj.parent && !obj.userData.type) {
            obj = obj.parent;
        }
        if (obj.userData && obj.userData.type === 'trebuchet') {
            return obj.userData.id;
        }
        return null;
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

    onWindowResize() {
        const container = document.getElementById(this.containerId);
        this.camera.aspect = container.clientWidth / container.clientHeight;
        this.camera.updateProjectionMatrix();
        this.renderer.setSize(container.clientWidth, container.clientHeight);
    }

    render(particleSystem) {
        const delta = this.clock.getDelta();
        if (this.controls) this.controls.update();
        if (particleSystem) particleSystem.update(delta);
        this.renderer.render(this.scene, this.camera);
        return delta;
    }

    getCanvasDomElement() {
        return this.renderer.domElement;
    }

    getRaycaster() {
        return this.raycaster || (this.raycaster = new THREE.Raycaster());
    }

    getMouseVector() {
        return this.mouseVec || (this.mouseVec = new THREE.Vector2());
    }
}
