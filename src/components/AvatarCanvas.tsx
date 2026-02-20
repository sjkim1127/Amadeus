import React, { useRef, useEffect, useState } from "react";
import { Canvas, useFrame, useThree } from "@react-three/fiber";
import { OrbitControls } from "@react-three/drei";
import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { VRMLoaderPlugin, VRM } from "@pixiv/three-vrm";

// Avatar state types
export type AvatarState = "idle" | "thinking" | "speaking";
export type AvatarEmotion = "neutral" | "happy" | "angry" | "surprised" | "sad";

interface VrmModelProps {
    avatarState: AvatarState;
    emotion: AvatarEmotion;
}

// Smooth interpolation helper
function lerp(current: number, target: number, speed: number): number {
    return current + (target - current) * speed;
}

const VrmModel: React.FC<VrmModelProps> = ({ avatarState, emotion }) => {
    const [vrm, setVrm] = useState<VRM | null>(null);
    const { scene } = useThree();
    const clockRef = useRef(new THREE.Clock());

    // Smooth animation targets
    const animState = useRef({
        mouthOpen: 0,
        headTiltX: 0,
        headTiltY: 0,
        headTiltZ: 0,
        eyeSquint: 0,
        browUp: 0,
        happy: 0,
        angry: 0,
        surprised: 0,
        sad: 0,
    });

    useEffect(() => {
        const loader = new GLTFLoader();
        loader.register((parser) => new VRMLoaderPlugin(parser));

        loader.load(
            "/model/vrm/KurisuMakise.vrm",
            (gltf) => {
                const vrmData = gltf.userData.vrm as VRM;
                if (vrmData) {
                    vrmData.scene.rotation.y = Math.PI;

                    // Set natural rest pose (override T-pose)
                    const h = vrmData.humanoid;
                    if (h) {
                        // Arms down naturally
                        const lUpperArm = h.getNormalizedBoneNode("leftUpperArm");
                        const rUpperArm = h.getNormalizedBoneNode("rightUpperArm");
                        const lLowerArm = h.getNormalizedBoneNode("leftLowerArm");
                        const rLowerArm = h.getNormalizedBoneNode("rightLowerArm");
                        const lHand = h.getNormalizedBoneNode("leftHand");
                        const rHand = h.getNormalizedBoneNode("rightHand");

                        if (lUpperArm) {
                            lUpperArm.rotation.z = 1.2;  // Arm down
                            lUpperArm.rotation.x = 0.1;  // Slightly forward
                        }
                        if (rUpperArm) {
                            rUpperArm.rotation.z = -1.2;
                            rUpperArm.rotation.x = 0.1;
                        }
                        if (lLowerArm) {
                            lLowerArm.rotation.z = 0.15; // Slight bend
                            lLowerArm.rotation.y = 0.0;
                        }
                        if (rLowerArm) {
                            rLowerArm.rotation.z = -0.15;
                            rLowerArm.rotation.y = 0.0;
                        }
                        if (lHand) lHand.rotation.z = 0.1;
                        if (rHand) rHand.rotation.z = -0.1;
                    }

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

    useFrame(() => {
        if (!vrm) return;

        const delta = clockRef.current.getDelta();
        const t = clockRef.current.getElapsedTime();
        const s = animState.current;
        const lerpSpeed = 8 * delta; // Smooth transitions

        // ===== Emotion Targets =====
        const emotionTargets = {
            happy: emotion === "happy" ? 0.7 : 0,
            angry: emotion === "angry" ? 0.6 : 0,
            surprised: emotion === "surprised" ? 0.8 : 0,
            sad: emotion === "sad" ? 0.5 : 0,
        };
        s.happy = lerp(s.happy, emotionTargets.happy, lerpSpeed);
        s.angry = lerp(s.angry, emotionTargets.angry, lerpSpeed);
        s.surprised = lerp(s.surprised, emotionTargets.surprised, lerpSpeed);
        s.sad = lerp(s.sad, emotionTargets.sad, lerpSpeed);

        // ===== State-driven Animation =====
        const head = vrm.humanoid?.getNormalizedBoneNode("head");
        const chest = vrm.humanoid?.getNormalizedBoneNode("chest");
        const spine = vrm.humanoid?.getNormalizedBoneNode("spine");

        switch (avatarState) {
            case "speaking": {
                // Lip sync: rapid mouth movement simulating speech
                const vowelCycle = Math.abs(Math.sin(t * 12.0));
                const consonantPause = Math.sin(t * 3.0) > 0.3 ? 1.0 : 0.2;
                s.mouthOpen = lerp(s.mouthOpen, vowelCycle * consonantPause * 0.6, lerpSpeed * 2);

                // Slight head movement while talking
                s.headTiltY = lerp(s.headTiltY, Math.sin(t * 1.5) * 0.06, lerpSpeed);
                s.headTiltX = lerp(s.headTiltX, Math.sin(t * 0.8) * 0.03, lerpSpeed);

                // Upper body micro-movement
                if (spine) {
                    spine.rotation.y = Math.sin(t * 1.0) * 0.015;
                }
                break;
            }

            case "thinking": {
                // Close mouth
                s.mouthOpen = lerp(s.mouthOpen, 0, lerpSpeed);

                // Tilt head to the side (tsundere thinking pose)
                s.headTiltZ = lerp(s.headTiltZ, Math.sin(t * 0.3) * 0.08 + 0.05, lerpSpeed);
                s.headTiltX = lerp(s.headTiltX, -0.04, lerpSpeed); // Look down slightly
                s.headTiltY = lerp(s.headTiltY, 0.02, lerpSpeed);

                // Slight body sway
                if (spine) {
                    spine.rotation.z = Math.sin(t * 0.5) * 0.01;
                }
                break;
            }

            case "idle":
            default: {
                // Close mouth
                s.mouthOpen = lerp(s.mouthOpen, 0, lerpSpeed);

                // Gentle head sway
                s.headTiltY = lerp(s.headTiltY, Math.sin(t * 0.5) * 0.02, lerpSpeed);
                s.headTiltX = lerp(s.headTiltX, Math.sin(t * 0.3) * 0.01, lerpSpeed);
                s.headTiltZ = lerp(s.headTiltZ, 0, lerpSpeed);

                // Reset spine
                if (spine) {
                    spine.rotation.y = lerp(spine.rotation.y, 0, lerpSpeed);
                    spine.rotation.z = lerp(spine.rotation.z, 0, lerpSpeed);
                }
                break;
            }
        }

        // ===== Apply Head Rotation =====
        if (head) {
            head.rotation.x = s.headTiltX;
            head.rotation.y = s.headTiltY;
            head.rotation.z = s.headTiltZ;
        }

        // ===== Breathing (always active) =====
        const breathe = Math.sin(t * 2.0) * 0.008;
        if (chest) {
            chest.scale.set(1.0 + breathe, 1.0 + breathe, 1.0 + breathe * 1.5);
        }

        // ===== Blink (always active, faster when surprised) =====
        const blinkInterval = emotion === "surprised" ? 6.0 : 4.0;
        const blinkCycle = t % blinkInterval;
        let blinkWeight = 0;
        if (blinkCycle < 0.15) {
            blinkWeight = Math.sin((blinkCycle / 0.15) * Math.PI);
        }

        // ===== Apply Expressions =====
        const em = vrm.expressionManager;
        if (em) {
            // Mouth
            em.setValue("aa", s.mouthOpen * 0.8);
            em.setValue("oh", s.mouthOpen * 0.3 * Math.sin(t * 6.0 + 1.0));

            // Blink (reduced when surprised)
            em.setValue("blink", emotion === "surprised" ? blinkWeight * 0.3 : blinkWeight);

            // Emotions (smooth blending)
            em.setValue("happy", s.happy);
            em.setValue("angry", s.angry);
            em.setValue("surprised", s.surprised);
            em.setValue("relaxed", s.sad); // Use 'relaxed' as sad fallback
        }

        vrm.update(delta);
    });

    return null;
};

// ===== Main Component =====

interface AvatarCanvasProps {
    avatarState?: AvatarState;
    emotion?: AvatarEmotion;
}

export const AvatarCanvas: React.FC<AvatarCanvasProps> = ({
    avatarState = "idle",
    emotion = "neutral",
}) => {
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
                <VrmModel avatarState={avatarState} emotion={emotion} />
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
