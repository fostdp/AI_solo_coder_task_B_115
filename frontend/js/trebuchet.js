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

        const woodMaterial = new THREE.MeshStandardMaterial({
            color: 0x8B4513,
            roughness: 0.8,
            metalness: 0.1,
        });

        const darkWoodMaterial = new THREE.MeshStandardMaterial({
            color: 0x5D3A1A,
            roughness: 0.9,
            metalness: 0.1,
        });

        const metalMaterial = new THREE.MeshStandardMaterial({
            color: 0x4a4a4a,
            roughness: 0.4,
            metalness: 0.8,
        });

        const stoneMaterial = new THREE.MeshStandardMaterial({
            color: 0x808080,
            roughness: 0.9,
            metalness: 0.1,
        });

        const baseWidth = 4 * scale;
        const baseDepth = 3 * scale;
        const baseHeight = 0.3 * scale;

        const baseGeo = new THREE.BoxGeometry(baseWidth, baseHeight, baseDepth);
        const base = new THREE.Mesh(baseGeo, darkWoodMaterial);
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
            const wheel = new THREE.Mesh(wheelGeo, darkWoodMaterial);
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
            const pole = new THREE.Mesh(poleGeo, woodMaterial);
            pole.position.set(pos[0] * scale, pos[1] * scale, pos[2] * scale);
            pole.castShadow = true;
            this.group.add(pole);
        });

        const topBeamGeo = new THREE.BoxGeometry(frameWidth * scale, 0.15 * scale, 0.2 * scale);
        const topBeam = new THREE.Mesh(topBeamGeo, darkWoodMaterial);
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
        const arm = new THREE.Mesh(armGeo, woodMaterial);
        arm.position.z = armLength * 0.3;
        arm.castShadow = true;
        this.armGroup.add(arm);

        const slingLength = 1.5 * scale;
        const slingGeo = new THREE.CylinderGeometry(0.02 * scale, 0.02 * scale, slingLength, 6);
        const sling1 = new THREE.Mesh(slingGeo, darkWoodMaterial);
        sling1.position.set(0.1 * scale, -slingLength / 2, armLength * 0.85);
        sling1.rotation.x = Math.PI / 2;
        this.armGroup.add(sling1);

        const sling2 = new THREE.Mesh(slingGeo, darkWoodMaterial);
        sling2.position.set(-0.1 * scale, -slingLength / 2, armLength * 0.85);
        sling2.rotation.x = Math.PI / 2;
        this.armGroup.add(sling2);

        const projDiameter = Math.cbrt(this.projectileKg / 2600 / (Math.PI * 4 / 3)) * 2 * scale;
        const projGeo = new THREE.SphereGeometry(projDiameter / 2, 16, 16);
        this.projectile = new THREE.Mesh(projGeo, stoneMaterial);
        this.projectile.position.set(0, -slingLength, armLength * 0.85);
        this.projectile.castShadow = true;
        this.armGroup.add(this.projectile);

        if (this.type === "配重式") {
            const cwVolume = this.counterweightKg / 7000;
            const cwSize = Math.cbrt(cwVolume) * scale;
            const cwGeo = new THREE.BoxGeometry(cwSize, cwSize, cwSize);
            const counterweight = new THREE.Mesh(cwGeo, stoneMaterial);
            counterweight.position.set(0, -cwSize / 2, -armLength * 0.2);
            counterweight.castShadow = true;
            this.armGroup.add(counterweight);

            const cwFrameGeo = new THREE.BoxGeometry(cwSize * 1.2, cwSize * 0.3, cwSize * 1.2);
            const cwFrame = new THREE.Mesh(cwFrameGeo, woodMaterial);
            cwFrame.position.set(0, -cwSize * 0.15, -armLength * 0.2);
            this.armGroup.add(cwFrame);
        } else {
            const ropeCount = 5;
            const ropeLength = 2 * scale;
            for (let i = 0; i < ropeCount; i++) {
                const ropeGeo = new THREE.CylinderGeometry(0.015 * scale, 0.015 * scale, ropeLength, 6);
                const rope = new THREE.Mesh(ropeGeo, darkWoodMaterial);
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
        const winch = new THREE.Mesh(winchGeo, woodMaterial);
        winch.position.set(0, baseHeight + 0.5 * scale, -baseDepth / 2 + 0.3 * scale);
        winch.castShadow = true;
        this.group.add(winch);

        const cableGeo = new THREE.CylinderGeometry(0.02 * scale, 0.02 * scale, frameHeight * 0.9, 6);
        const cable = new THREE.Mesh(cableGeo, metalMaterial);
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
