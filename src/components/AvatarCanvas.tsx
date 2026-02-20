import React, { useRef, useEffect, useState } from "react";
import { Canvas, useFrame, useThree } from "@react-three/fiber";
import { OrbitControls } from "@react-three/drei";
import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { VRMLoaderPlugin, VRM } from "@pixiv/three-vrm";
import { loadMixamoAnimation } from "../utils/loadMixamoAnimation";

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

    // Mixamo Animation System
    const mixerRef = useRef<THREE.AnimationMixer | null>(null);
    const actionsRef = useRef<Record<string, THREE.AnimationAction>>({});
    const currentActionRef = useRef<THREE.AnimationAction | null>(null);

    // Emotion target states
    const animState = useRef({
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
            async (gltf) => {
                const vrmData = gltf.userData.vrm as VRM;
                if (!vrmData) return;

                vrmData.scene.rotation.y = Math.PI;

                // Set natural rest pose fallback (override T-pose immediately)
                const h = vrmData.humanoid;
                if (h) {
                    const lUpperArm = h.getNormalizedBoneNode("leftUpperArm");
                    const rUpperArm = h.getNormalizedBoneNode("rightUpperArm");
                    const lLowerArm = h.getNormalizedBoneNode("leftLowerArm");
                    const rLowerArm = h.getNormalizedBoneNode("rightLowerArm");
                    const lHand = h.getNormalizedBoneNode("leftHand");
                    const rHand = h.getNormalizedBoneNode("rightHand");

                    if (lUpperArm) { lUpperArm.rotation.z = 1.2; lUpperArm.rotation.x = 0.1; }
                    if (rUpperArm) { rUpperArm.rotation.z = -1.2; rUpperArm.rotation.x = 0.1; }
                    if (lLowerArm) { lLowerArm.rotation.z = 0.15; }
                    if (rLowerArm) { rLowerArm.rotation.z = -0.15; }
                    if (lHand) lHand.rotation.z = 0.1;
                    if (rHand) rHand.rotation.z = -0.1;
                }

                scene.add(vrmData.scene);
                setVrm(vrmData);
                console.log("[VRM] Model loaded");

                // Initialize AnimationMixer
                const mixer = new THREE.AnimationMixer(vrmData.scene);
                mixerRef.current = mixer;

                // Load Mixamo FBX animations
                const anims = [
                    { name: 'idle', url: '/model/animations/idle.fbx' },
                    { name: 'thinking', url: '/model/animations/thinking.fbx' },
                    { name: 'speaking', url: '/model/animations/speaking.fbx' },
                ];

                for (const anim of anims) {
                    try {
                        const clip = await loadMixamoAnimation(anim.url, vrmData);
                        if (clip) {
                            console.log(`[VRM] Loaded ${anim.name} with ${clip.tracks.length} tracks`);
                            const action = mixer.clipAction(clip);
                            action.play();
                            action.weight = 0; // Start faded out
                            actionsRef.current[anim.name] = action;
                        } else {
                            console.warn(`[VRM] Failed to parse clip for ${anim.name}`);
                        }
                    } catch (e) {
                        console.error(`[VRM] Failed to load animation ${anim.name}:`, e);
                    }
                }

                // Start idle animation
                if (actionsRef.current['idle']) {
                    actionsRef.current['idle'].weight = 1;
                    currentActionRef.current = actionsRef.current['idle'];
                }
            },
            () => { },
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
            if (mixerRef.current) {
                mixerRef.current.stopAllAction();
            }
        };
    }, []);

    // Handle State Changes (Crossfade Animations)
    useEffect(() => {
        const newActionName = avatarState; // 'idle', 'thinking', 'speaking'
        const newAction = actionsRef.current[newActionName] || actionsRef.current['idle'];

        if (newAction && currentActionRef.current !== newAction) {
            const previousAction = currentActionRef.current;
            currentActionRef.current = newAction;

            // Smooth crossfade over 0.5 seconds
            if (previousAction) {
                newAction.reset().play();
                newAction.setEffectiveTimeScale(1);
                newAction.setEffectiveWeight(1);
                newAction.crossFadeFrom(previousAction, 0.5, true);
            } else {
                newAction.play();
                newAction.weight = 1;
            }
        }
    }, [avatarState]);

    // Handle Frame updates (Mixer & Emotion Blending)
    useFrame(() => {
        if (!vrm) return;

        const delta = clockRef.current.getDelta();
        const t = clockRef.current.getElapsedTime();
        const s = animState.current;
        const lerpSpeed = 8 * delta;

        // Update AnimationMixer!
        // This drives all body bone rotations/positions based on Mixamo clips
        if (mixerRef.current) {
            mixerRef.current.update(delta);
        }

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

        // ===== Lip Sync (When speaking) =====
        let mouthOpen = 0;
        if (avatarState === "speaking") {
            const vowelCycle = Math.abs(Math.sin(t * 12.0));
            const consonantPause = Math.sin(t * 3.0) > 0.3 ? 1.0 : 0.2;
            mouthOpen = vowelCycle * consonantPause * 0.6;
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
            em.setValue("aa", mouthOpen * 0.8);
            em.setValue("oh", mouthOpen * 0.3 * Math.sin(t * 6.0 + 1.0));
            em.setValue("blink", emotion === "surprised" ? blinkWeight * 0.3 : blinkWeight);
            em.setValue("happy", s.happy);
            em.setValue("angry", s.angry);
            em.setValue("surprised", s.surprised);
            em.setValue("relaxed", s.sad);
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
