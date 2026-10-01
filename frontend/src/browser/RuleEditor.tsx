import { HERO_OPTIONS } from "../heroes";
import type { WorkshopSetup } from "./engine";

export function NumberField({ label, value, min = 1, max, onChange }: {
  label: string; value: number; min?: number; max: number; onChange: (value: number) => void;
}) {
  return <label className="workshop-field"><span>{label}</span><input type="number" required min={min} max={max} value={Number.isNaN(value) ? "" : value} onChange={(event) => onChange(event.target.valueAsNumber)} /></label>;
}

export function RuleEditor({ setup, onChange }: { setup: WorkshopSetup; onChange: (setup: WorkshopSetup) => void }) {
  const turn = setup.ruleset.turn;
  function changeTurn(patch: Partial<typeof turn>) {
    onChange({ ...setup, ruleset: { ...setup.ruleset, turn: { ...turn, ...patch } } });
  }
  return <div className="workshop-rules">
    <h2>Match rules</h2>
    <p>Changes apply to your next browser Solo match. Both sides use the same rules.</p>
    <div className="workshop-fields">
      <NumberField label="Base Hero Mana" value={turn.baseHeroMana} max={30} onChange={(baseHeroMana) => changeTurn({ baseHeroMana })} />
      <NumberField label="Opening hand size" value={turn.openingHandSize} max={12} onChange={(openingHandSize) => changeTurn({ openingHandSize })} />
      <NumberField label="Carried Item limit" value={turn.maxCarriedItems} max={6} onChange={(maxCarriedItems) => changeTurn({ maxCarriedItems })} />
      <NumberField label="Unit attack range" value={turn.defaultAttackRange} max={6} onChange={(defaultAttackRange) => changeTurn({ defaultAttackRange })} />
    </div>
    <h3>Hero attributes</h3>
    {HERO_OPTIONS.map((hero) => {
      const rule = setup.ruleset.heroes[hero.id];
      function update(patch: Partial<typeof rule>) {
        onChange({ ...setup, ruleset: { ...setup.ruleset, heroes: { ...setup.ruleset.heroes, [hero.id]: { ...rule, ...patch } } } });
      }
      return <fieldset key={hero.id} className="workshop-hero-rule"><legend>{hero.name}</legend><div className="workshop-fields">
        <NumberField label={`${hero.name} HP`} value={rule.maxHp} max={100} onChange={(maxHp) => update({ maxHp })} />
        <NumberField label={`${hero.name} attack`} value={rule.attack} min={0} max={20} onChange={(attack) => update({ attack })} />
        <NumberField label={`${hero.name} AP`} value={rule.maxAp} max={10} onChange={(maxAp) => update({ maxAp })} />
        <NumberField label={`${hero.name} attack range`} value={rule.attackRange} max={6} onChange={(attackRange) => update({ attackRange })} />
      </div></fieldset>;
    })}
  </div>;
}
