import { Canvas } from "@react-three/fiber";
import { Suspense, useEffect, useState } from "react";
import type { Object3D } from "three";
import {
  BOARD_HERO_APPEARANCE_MODEL_ASSETS,
  BOARD_HERO_APPEARANCE_RECIPES,
  type BoardModelAssetEntry,
} from "./board3dModelManifest";
import { loadGltfScene, normalizeModel } from "./Board3D/ModelLoader";
import { ProceduralMiniature } from "./Board3D/ProceduralMiniature";
import { baseHeroAppearanceId, type HeroAppearanceId } from "./heroAppearances";
import type { HeroType } from "./types";

type HeroPreview3DProps = {
  heroType: HeroType;
  label: string;
  appearanceId?: HeroAppearanceId | null;
};

export function HeroPreview3D({ heroType, label, appearanceId }: HeroPreview3DProps) {
  const resolvedAppearanceId = appearanceId ?? baseHeroAppearanceId(heroType);
  const recipe =
    BOARD_HERO_APPEARANCE_RECIPES[resolvedAppearanceId] ??
    BOARD_HERO_APPEARANCE_RECIPES[baseHeroAppearanceId(heroType)];
  const modelAsset = BOARD_HERO_APPEARANCE_MODEL_ASSETS[resolvedAppearanceId];

  return (
    <div className="hero-preview-3d" aria-label={`${label} 3D preview`}>
      <Canvas
        camera={{ position: [0, 1.3, 3.25], fov: 32, near: 0.1, far: 20 }}
        dpr={[1, 1.6]}
        gl={{ antialias: true, alpha: true }}
        onCreated={({ camera }) => camera.lookAt(0, 0.65, 0)}
      >
        <ambientLight intensity={0.86} />
        <directionalLight position={[2.4, 4.5, 3]} intensity={2.15} />
        <directionalLight position={[-2.2, 2.4, -2]} intensity={0.55} />
        <pointLight position={[-1.8, 1.7, 2.4]} intensity={0.9} />
        <Suspense fallback={null}>
          <group rotation={[0, -0.38, 0]}>
            <HeroModelOrMiniature
              heroType={heroType}
              label={label}
              modelAsset={modelAsset}
              recipe={recipe}
            />
          </group>
        </Suspense>
      </Canvas>
    </div>
  );
}

function HeroModelOrMiniature({
  heroType,
  label,
  modelAsset,
  recipe,
}: {
  heroType: HeroType;
  label: string;
  modelAsset?: BoardModelAssetEntry;
  recipe: (typeof BOARD_HERO_APPEARANCE_RECIPES)[string];
}) {
  const [model, setModel] = useState<Object3D | null>(null);

  useEffect(() => {
    let cancelled = false;
    setModel(null);

    if (!modelAsset) {
      return () => {
        cancelled = true;
      };
    }

    const path = `${import.meta.env.BASE_URL}${modelAsset.path.replace(/^\//, "")}`;
    void loadGltfScene(path)
      .then((scene) => {
        if (cancelled) {
          return;
        }
        const clone = scene.clone(true);
        normalizeModel(clone, modelAsset.scale);
        setModel(clone);
      })
      .catch(() => {
        if (!cancelled) {
          setModel(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [modelAsset]);

  if (model && modelAsset) {
    return (
      <primitive
        object={model}
        position={[0, modelAsset.yOffset ?? 0.68, 0]}
        rotation={[0, modelAsset.rotationY ?? 0, 0]}
      />
    );
  }

  return (
    <ProceduralMiniature
      position={[0, 0.02, 0]}
      side="player"
      pieceType="hero"
      visualIdentity={{
        status: "resolved",
        heroType,
        name: label,
        portraitPath: `/hero-art/${heroType}.svg`,
        portraitAlt: `${label} portrait`,
        accentClass: heroType,
        fallbackLabel: label.slice(0, 3).toUpperCase(),
      }}
      recipe={recipe}
      selected={false}
      legalTarget={false}
      coord={{ q: 0, r: 0 }}
      readOnly
      disabled
      onClick={() => undefined}
      onContextMenu={() => undefined}
    />
  );
}
