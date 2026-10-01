import { Canvas } from "@react-three/fiber";
import { Component, Suspense, useEffect, useState, type ReactNode } from "react";
import { canCreateWebGLContext } from "./boardRenderer";
import { effectiveBoardVisualMode } from "./boardVisualMode";
import {
  BOARD_HERO_APPEARANCE_MODEL_ASSETS,
  BOARD_HERO_APPEARANCE_RECIPES,
  type BoardModelAssetEntry,
} from "./board3dModelManifest";
import type { Object3D } from "three";
import { loadGltfScene, normalizeModel } from "./Board3D/ModelLoader";
import { ProceduralMiniature } from "./Board3D/ProceduralMiniature";
import { baseHeroAppearanceId, type HeroAppearanceId } from "./heroAppearances";
import type { BoardVisualMode, HeroType } from "./types";

const HERO_STAGE_CENTER_Y = 0.65;

type HeroPreview3DProps = {
  heroType: HeroType;
  label: string;
  appearanceId?: HeroAppearanceId | null;
  visualMode?: BoardVisualMode;
};

export function HeroPreview3D({ heroType, label, appearanceId, visualMode }: HeroPreview3DProps) {
  const portrait = (
    <div className="hero-preview-3d">
      <img src={`${import.meta.env.BASE_URL}hero-art/${heroType}.svg`} alt={`${label} portrait`} />
    </div>
  );
  if ((visualMode ?? effectiveBoardVisualMode(null)) === "2d") return portrait;
  return <HeroStage heroType={heroType} label={label} appearanceId={appearanceId} portrait={portrait} />;
}

function HeroStage({ heroType, label, appearanceId, portrait }: HeroPreview3DProps & { portrait: ReactNode }) {
  const [canRender] = useState(canCreateWebGLContext);
  if (!canRender) return portrait;
  const requested = appearanceId ? BOARD_HERO_APPEARANCE_RECIPES[appearanceId] : undefined;
  const resolvedAppearance = requested?.heroAppearance?.heroType === heroType && appearanceId
    ? appearanceId : baseHeroAppearanceId(heroType);
  const recipe = BOARD_HERO_APPEARANCE_RECIPES[resolvedAppearance];
  const modelAsset = BOARD_HERO_APPEARANCE_MODEL_ASSETS[resolvedAppearance];

  return (
    <HeroPreviewErrorBoundary fallback={portrait}>
      <div className="hero-preview-3d" aria-label={`${label} 3D preview`}>
        <Canvas
          fallback={portrait}
          frameloop="demand"
          camera={{ position: [0, 1.35, 3.35], fov: 34, near: 0.1, far: 20 }}
          dpr={[1, 1.6]}
          gl={{ antialias: true, alpha: true }}
          onCreated={({ camera }) => camera.lookAt(0, HERO_STAGE_CENTER_Y, 0)}
        >
          <ambientLight intensity={0.9} />
          <directionalLight position={[2, 4, 3]} intensity={1.8} />
          <pointLight position={[-2, 2.2, 2.2]} intensity={0.8} />
          <Suspense fallback={null}>
            <group rotation={[0, -0.38, 0]}>
              <HeroModel asset={modelAsset}>
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
              </HeroModel>
            </group>
          </Suspense>
        </Canvas>
      </div>
    </HeroPreviewErrorBoundary>
  );
}

class HeroPreviewErrorBoundary extends Component<
  { children: ReactNode; fallback: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}

function HeroModel({ asset, children }: { asset: BoardModelAssetEntry | undefined; children: ReactNode }) {
  const [loaded, setLoaded] = useState<{ asset: BoardModelAssetEntry; scene: Object3D } | null>(null);
  useEffect(() => {
    if (!asset) return;
    let cancelled = false;
    const path = `${import.meta.env.BASE_URL}${asset.path.replace(/^\//, "")}`;
    loadGltfScene(path).then((scene) => {
      if (cancelled) return;
      const clone = scene.clone(true);
      normalizeModel(clone, asset.scale);
      clone.position.y += HERO_STAGE_CENTER_Y + (asset.yOffset ?? 0);
      clone.rotation.y += asset.rotationY ?? 0;
      setLoaded({ asset, scene: clone });
    }).catch(() => {
      // Keep the procedural miniature when the configured asset cannot load.
    });
    return () => { cancelled = true; };
  }, [asset]);
  if (loaded && loaded.asset === asset) return <primitive object={loaded.scene} />;
  return children;
}
