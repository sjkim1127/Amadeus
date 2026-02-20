import React, { useRef, useEffect, useState } from "react";
import { Canvas, useFrame, useThree } from "@react-three/fiber";
import { OrbitControls } from "@react-three/drei";
import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { VRMLoaderPlugin, VRM } from "@pixiv/three-vrm";

const VrmModel: React.FC = () => {
    const [vrm, setVrm] = useState<VRM | null>(null);
    const { scene } = useThree();
    const clockRef = useRef(new THREE.Clock());

    useEffect(() => {
        const loader = new GLTFLoader();
        loader.register((parser) => new VRMLoaderPlugin(parser));

        loader.load(
            "/model/vrm/KurisuMakise.vrm",
            (gltf) => {
                const vrmData = gltf.userData.vrm as VRM;
                if (vrmData) {
                    // Rotate model to face camera
                    vrmData.scene.rotation.y = Math.PI;
                    scene.add(vrmData.scene);
                    setVrm(vrmData);
                    console.log("[VRM] Model loaded:", vrmData);
                }
            },
            (progress) => {
                console.log(
                    "[VRM] Loading:",
                    Math.round((progress.loaded / progress.total) * 100),
                    "%"
                );
            },
            (error) => {
                console.error("[VRM] Failed to load:", error);
            }
        );

        return () => {
            if (vrm) {
                scene.remove(vrm.scene);
                vrm.scene.traverse((obj) => {
                    if ((obj as THREE.Mesh).geometry) {
                        (obj as THREE.Mesh).geometry.dispose();
                    }
                });
            }
        };
    }, []);

    // Idle breathing animation
    useFrame(() => {
        if (!vrm) return;

        const delta = clockRef.current.getDelta();
        const t = clockRef.current.getElapsedTime();

        // Subtle breathing via chest bone
        const breathe = Math.sin(t * 2.0) * 0.008;
        const chest = vrm.humanoid?.getNormalizedBoneNode("chest");
        if (chest) {
            chest.scale.set(1.0 + breathe, 1.0 + breathe, 1.0 + breathe * 1.5);
        }

        // Subtle head sway
        const head = vrm.humanoid?.getNormalizedBoneNode("head");
        if (head) {
            head.rotation.y = Math.sin(t * 0.5) * 0.02;
            head.rotation.x = Math.sin(t * 0.3) * 0.01;
        }

        // Blink every ~4 seconds
        const blinkCycle = t % 4.0;
        if (blinkCycle < 0.15) {
            const blinkWeight = Math.sin((blinkCycle / 0.15) * Math.PI);
            vrm.expressionManager?.setValue("blink", blinkWeight);
        } else {
            vrm.expressionManager?.setValue("blink", 0);
        }

        vrm.update(delta);
    });

    return null;
};

export const AvatarCanvas: React.FC = () => {
    return (
        <div className="avatar-section">
            <Canvas
                camera={{
                    position: [0, 1.2, 2.0],
                    fov: 30,
                    near: 0.1,
                    far: 100,
                }}
                gl={{ alpha: true, antialias: true }}
                style={{ background: "transparent" }}
            >
                <ambientLight intensity={0.6} />
                <directionalLight position={[4, 10, 4]} intensity={1.2} />
                <VrmModel />
                <OrbitControls
                    target={[0, 1.0, 0]}
                    enablePan={false}
                    minDistance={1.5}
                    maxDistance={4}
                    minPolarAngle={Math.PI / 4}
                    maxPolarAngle={Math.PI / 2}
                />
            </Canvas>
        </div>
    );
};
