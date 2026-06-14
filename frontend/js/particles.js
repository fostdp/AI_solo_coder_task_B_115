class InstancedParticleSystem {
    constructor(scene) {
        this.scene = scene;
        this.maxParticles = 2000;
        this.pool = [];
        this.activeCount = 0;
        this.dummy = new THREE.Object3D();
        this.tmpColor = new THREE.Color();

        this._initPool();
    }

    _initPool() {
        const sizes = [0.08, 0.15, 0.25, 0.4, 0.6];
        const geoms = sizes.map(s => new THREE.SphereGeometry(s, 6, 6));

        for (let i = 0; i < 5; i++) {
            const geom = geoms[i];
            const mat = new THREE.MeshBasicMaterial({
                vertexColors: false,
                transparent: true,
                opacity: 1.0,
            });

            const instanced = new THREE.InstancedMesh(
                geom, mat, this.maxParticles / 5
            );
            instanced.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
            instanced.frustumCulled = false;

            const colors = new Float32Array((this.maxParticles / 5) * 3);
            instanced.instanceColor = new THREE.InstancedBufferAttribute(colors, 3);
            instanced.instanceColor.setUsage(THREE.DynamicDrawUsage);

            this.scene.add(instanced);

            const poolEntry = {
                mesh: instanced,
                material: mat,
                data: new Array(this.maxParticles / 5).fill(null),
                size: sizes[i],
            };

            for (let j = 0; j < poolEntry.data.length; j++) {
                this.dummy.position.set(-9999, -9999, -9999);
                this.dummy.rotation.set(0, 0, 0);
                this.dummy.scale.set(1, 1, 1);
                this.dummy.updateMatrix();
                instanced.setMatrixAt(j, this.dummy.matrix);
                instanced.setColorAt(j, new THREE.Color(0, 0, 0));
            }
            instanced.instanceMatrix.needsUpdate = true;
            instanced.instanceColor.needsUpdate = true;
            instanced.count = 0;

            this.pool.push(poolEntry);
        }
    }

    _findBestPoolIndex(desiredSize) {
        let bestIdx = 0;
        let bestDiff = Infinity;
        for (let i = 0; i < this.pool.length; i++) {
            const diff = Math.abs(this.pool[i].size - desiredSize);
            if (diff < bestDiff) {
                bestDiff = diff;
                bestIdx = i;
            }
        }
        return bestIdx;
    }

    spawn(position, velocity, options = {}) {
        const size = options.size || 0.2;
        const life = options.life || 2.0;
        const decay = options.decay || (1.0 / life);
        const color = options.color || 0xffd700;
        const gravity = options.gravity !== undefined ? options.gravity : -9.81;
        const drag = options.drag !== undefined ? options.drag : 0.0;

        const poolIdx = this._findBestPoolIndex(size);
        const pool = this.pool[poolIdx];

        let slot = -1;
        for (let i = 0; i < pool.data.length; i++) {
            if (pool.data[i] === null || pool.data[i].life <= 0) {
                slot = i;
                break;
            }
        }

        if (slot === -1) {
            return null;
        }

        pool.data[slot] = {
            position: position.clone(),
            velocity: velocity.clone(),
            life: life,
            decay: decay,
            color: new THREE.Color(color),
            gravity: gravity,
            drag: drag,
            active: true,
        };

        this.activeCount++;
        return { poolIdx, slot };
    }

    spawnBurst(center, options = {}) {
        const count = options.count || 30;
        const minSpeed = options.minSpeed || 2;
        const maxSpeed = options.maxSpeed || 8;
        const upward = options.upward || 2;
        const size = options.size || 0.2;
        const life = options.life || 1.5;
        const color = options.color || 0x8B4513;

        const results = [];
        for (let i = 0; i < count; i++) {
            const angle = Math.random() * Math.PI * 2;
            const speed = Math.random() * (maxSpeed - minSpeed) + minSpeed;
            const upwardSpeed = Math.random() * 6 + upward;

            const velocity = new THREE.Vector3(
                Math.cos(angle) * speed,
                upwardSpeed,
                Math.sin(angle) * speed
            );

            results.push(this.spawn(
                center.clone(),
                velocity,
                { size: size * (0.5 + Math.random() * 0.8), life, decay: 1 / life, color }
            ));
        }
        return results;
    }

    createTrajectory(trajectoryPoints, options = {}) {
        const color = options.color || 0xffd700;
        const particleSize = options.particleSize || 0.3;
        const speed = options.speed || 1;
        const onComplete = options.onComplete || null;

        const trajectory = {
            points: trajectoryPoints,
            progress: 0,
            speed: speed,
            active: true,
            color: color,
            particleSize: particleSize,
            trailSlots: [],
            projectileHandle: null,
            onComplete: onComplete,
        };

        this._activeTrajectories = this._activeTrajectories || [];
        this._activeTrajectories.push(trajectory);
        return trajectory;
    }

    createTrajectoryLine(trajectoryPoints, options = {}) {
        const color = options.color || 0xffd700;
        const opacity = options.opacity || 0.6;

        const points = trajectoryPoints.map(p =>
            new THREE.Vector3(p.x, Math.max(p.y, 0), p.z || 0)
        );

        const geometry = new THREE.BufferGeometry().setFromPoints(points);
        const material = new THREE.LineBasicMaterial({
            color: color,
            transparent: true,
            opacity: opacity,
        });
        const line = new THREE.Line(geometry, material);
        this.scene.add(line);

        this._lineMeshes = this._lineMeshes || [];
        this._lineMeshes.push(line);

        return line;
    }

    createImpactRing(position, options = {}) {
        const color = options.color || 0xffff00;
        const maxScale = options.maxScale || 5;

        const ringGeo = new THREE.RingGeometry(0.5, 3, 32);
        const ringMat = new THREE.MeshBasicMaterial({
            color: color,
            transparent: true,
            opacity: 0.8,
            side: THREE.DoubleSide,
            depthWrite: false,
        });
        const ring = new THREE.Mesh(ringGeo, ringMat);
        ring.rotation.x = -Math.PI / 2;
        ring.position.copy(position);
        ring.position.y = 0.01;

        this._rings = this._rings || [];
        this._rings.push({ mesh: ring, scale: 1, maxScale, life: 1 });
        this.scene.add(ring);
    }

    createGroundImpact(position, options = {}) {
        const color = options.color || 0x8B4513;
        const count = options.count || 30;

        this.spawnBurst(position.clone().setY(0.1), {
            count,
            color,
            minSpeed: 2,
            maxSpeed: 8,
            upward: 2,
            size: 0.15,
            life: 1.5,
        });

        this.createImpactRing(position.clone().setY(0.01), { color: 0xffff00, maxScale: 5 });
    }

    update(deltaTime) {
        const dt = deltaTime || 0.016;

        for (const pool of this.pool) {
            let dirty = false;
            let maxActiveSlot = -1;

            for (let i = 0; i < pool.data.length; i++) {
                const p = pool.data[i];

                if (!p || p.life <= 0) {
                    if (i <= maxActiveSlot) {
                        this.dummy.position.set(-9999, -9999, -9999);
                        this.dummy.scale.set(0.01, 0.01, 0.01);
                        this.dummy.updateMatrix();
                        pool.mesh.setMatrixAt(i, this.dummy.matrix);
                        pool.mesh.setColorAt(i, new THREE.Color(0, 0, 0));
                        dirty = true;
                    }
                    continue;
                }

                p.velocity.y += p.gravity * dt;
                p.velocity.multiplyScalar(Math.max(0, 1 - p.drag * dt));
                p.position.addScaledVector(p.velocity, dt);

                if (p.position.y <= 0.05) {
                    p.position.y = 0.05;
                    p.velocity.y *= -0.3;
                    p.velocity.x *= 0.7;
                    p.velocity.z *= 0.7;
                    if (Math.abs(p.velocity.y) < 0.5) {
                        p.velocity.y = 0;
                    }
                }

                p.life -= p.decay * dt;

                const opacity = Math.max(0, Math.min(1, p.life));
                pool.material.opacity = Math.max(pool.material.opacity, opacity);

                this.dummy.position.copy(p.position);
                const scale = 0.5 + opacity * 0.5;
                this.dummy.scale.set(scale, scale, scale);
                this.dummy.updateMatrix();
                pool.mesh.setMatrixAt(i, this.dummy.matrix);

                const r = p.color.r * opacity;
                const g = p.color.g * opacity;
                const b = p.color.b * opacity;
                this.tmpColor.setRGB(r, g, b);
                pool.mesh.setColorAt(i, this.tmpColor);

                maxActiveSlot = i;
                dirty = true;
            }

            if (dirty) {
                pool.mesh.count = Math.max(0, maxActiveSlot + 1);
                pool.mesh.instanceMatrix.needsUpdate = true;
                pool.mesh.instanceColor.needsUpdate = true;
            }
        }

        if (this._activeTrajectories) {
            for (let tIdx = this._activeTrajectories.length - 1; tIdx >= 0; tIdx--) {
                const traj = this._activeTrajectories[tIdx];
                if (!traj.active) continue;

                traj.progress += dt * traj.speed * 2;
                const totalPoints = traj.points.length;
                const pointIdx = Math.floor(traj.progress * (totalPoints - 1));

                if (pointIdx >= totalPoints - 1) {
                    traj.active = false;
                    const last = traj.points[totalPoints - 1];
                    this.createGroundImpact(
                        new THREE.Vector3(last.x, 0.1, last.z || 0),
                        { color: 0x8B4513, count: 25 }
                    );
                    if (traj.onComplete) traj.onComplete();
                    this._activeTrajectories.splice(tIdx, 1);
                    continue;
                }

                const pt = traj.points[pointIdx];

                if (traj.projectileHandle) {
                    const pool = this.pool[traj.projectileHandle.poolIdx];
                    const slot = traj.projectileHandle.slot;
                    if (pool.data[slot]) {
                        pool.data[slot].position.set(pt.x, pt.y, pt.z || 0);
                        pool.data[slot].life = 0.1;
                    }
                } else {
                    const projVel = new THREE.Vector3(0, 0, 0);
                    const pos = new THREE.Vector3(pt.x, pt.y, pt.z || 0);
                    traj.projectileHandle = this.spawn(pos, projVel, {
                        size: traj.particleSize * 1.5,
                        color: 0xffa500,
                        life: 999,
                        decay: 0,
                        gravity: 0,
                    });

                    for (let trailIdx = 0; trailIdx < 15; trailIdx++) {
                        const tPos = new THREE.Vector3(pt.x, pt.y, pt.z || 0);
                        const tVel = new THREE.Vector3(0, 0, 0);
                        const size = traj.particleSize * (1 - trailIdx / 15) * 0.8;
                        const cIdx = Math.floor(trailIdx / 3);
                        const c = [0xffa500, 0xff8c00, 0xff6600, 0xff4500, 0xff0000][cIdx] || 0xffa500;
                        const h = this.spawn(tPos, tVel, {
                            size,
                            color: c,
                            life: 999,
                            decay: 0,
                            gravity: 0,
                        });
                        if (h) traj.trailSlots.push(h);
                    }
                }

                for (let i = 0; i < traj.trailSlots.length; i++) {
                    const h = traj.trailSlots[i];
                    if (!h) continue;
                    const pool = this.pool[h.poolIdx];
                    const slot = h.slot;
                    const targetIdx = Math.max(0, pointIdx - i * 3);
                    const targetPoint = traj.points[targetIdx];
                    if (pool.data[slot] && targetPoint) {
                        pool.data[slot].position.set(
                            targetPoint.x, targetPoint.y, targetPoint.z || 0
                        );
                        pool.data[slot].life = Math.max(0.1, 1 - i / traj.trailSlots.length);
                    }
                }
            }
        }

        if (this._rings) {
            for (let i = this._rings.length - 1; i >= 0; i--) {
                const r = this._rings[i];
                r.scale += dt * 8;
                r.life -= dt * 2;
                r.mesh.scale.setScalar(r.scale);
                r.mesh.material.opacity = r.life * 0.8;
                if (r.life <= 0 || r.scale >= r.maxScale * 1.5) {
                    this.scene.remove(r.mesh);
                    r.mesh.geometry.dispose();
                    r.mesh.material.dispose();
                    this._rings.splice(i, 1);
                }
            }
        }
    }

    clearAll() {
        for (const pool of this.pool) {
            for (let i = 0; i < pool.data.length; i++) {
                pool.data[i] = null;
                this.dummy.position.set(-9999, -9999, -9999);
                this.dummy.scale.set(0.01, 0.01, 0.01);
                this.dummy.updateMatrix();
                pool.mesh.setMatrixAt(i, this.dummy.matrix);
                pool.mesh.setColorAt(i, new THREE.Color(0, 0, 0));
            }
            pool.mesh.count = 0;
            pool.mesh.instanceMatrix.needsUpdate = true;
            pool.mesh.instanceColor.needsUpdate = true;
        }

        if (this._activeTrajectories) {
            this._activeTrajectories = [];
        }

        if (this._rings) {
            for (const r of this._rings) {
                this.scene.remove(r.mesh);
                r.mesh.geometry.dispose();
                r.mesh.material.dispose();
            }
            this._rings = [];
        }

        if (this._lineMeshes) {
            for (const line of this._lineMeshes) {
                this.scene.remove(line);
                line.geometry.dispose();
                line.material.dispose();
            }
            this._lineMeshes = [];
        }

        this.activeCount = 0;
    }
}

class TrajectoryParticles extends InstancedParticleSystem {
    constructor(scene) {
        super(scene);
        this._activeTrajectories = [];
        this._lineMeshes = [];
        this._rings = [];
    }
}
