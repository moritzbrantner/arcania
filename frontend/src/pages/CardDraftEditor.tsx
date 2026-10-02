import type { Rarity } from "../types";
import type { CardDefinition } from "../types/cardWorkshop";

const RARITIES: { value: Rarity; label: string }[] = [
  { value: "basic", label: "Basic" }, { value: "advanced", label: "Advanced" }, { value: "rare", label: "Rare" },
];

// Ranges the Draft API can represent (core u8/i32 fields). Card validity stays with rune-lanes-core.
const U8 = { min: 0, max: 255 };
const I32 = { min: -2_147_483_648, max: 2_147_483_647 };

type NumericField = { label: string; value: number; range: { min: number; max: number } };

function numericFields(definition: CardDefinition): NumericField[] {
  const fields: NumericField[] = [{ label: "Mana cost", value: definition.cost, range: U8 }];
  if (definition.kind.type === "unit") {
    fields.push(
      { label: "Unit attack", value: definition.kind.attack, range: I32 },
      { label: "Unit armor", value: definition.kind.armor, range: I32 },
      { label: "Unit AP", value: definition.kind.maxAp, range: U8 },
    );
  }
  return fields;
}

/** Problems that prevent sending the edited definition at all; the core reports everything else after saving. */
export function draftInputProblems(definition: CardDefinition): string[] {
  return numericFields(definition)
    .filter(({ value, range }) => !Number.isInteger(value) || value < range.min || value > range.max)
    .map(({ label, range }) => `${label} must be a whole number from ${range.min} to ${range.max}.`);
}

export function CardDraftEditor({ definition, disabled, onChange }: {
  definition: CardDefinition; disabled: boolean; onChange: (definition: CardDefinition) => void;
}) {
  const kind = definition.kind;
  return <div className="card-draft-editor">
    <label className="workshop-field"><span>Card name</span><input disabled={disabled} value={definition.name} onChange={(event) => onChange({ ...definition, name: event.target.value })} /></label>
    <div className="workshop-fields">
      <label className="workshop-field"><span>Rarity</span><select disabled={disabled} value={definition.rarity} onChange={(event) => onChange({ ...definition, rarity: event.target.value as Rarity })}>{RARITIES.map((rarity) => <option key={rarity.value} value={rarity.value}>{rarity.label}</option>)}</select></label>
      <IntegerField label="Mana cost" disabled={disabled} value={definition.cost} range={U8} onChange={(cost) => onChange({ ...definition, cost })} />
    </div>
    {kind.type === "unit" ? <div className="workshop-fields">
      <IntegerField label="Unit attack" disabled={disabled} value={kind.attack} range={I32} onChange={(attack) => onChange({ ...definition, kind: { ...kind, attack } })} />
      <IntegerField label="Unit armor" disabled={disabled} value={kind.armor} range={I32} onChange={(armor) => onChange({ ...definition, kind: { ...kind, armor } })} />
      <IntegerField label="Unit AP" disabled={disabled} value={kind.maxAp} range={U8} onChange={(maxAp) => onChange({ ...definition, kind: { ...kind, maxAp } })} />
    </div> : null}
  </div>;
}

function IntegerField({ label, value, range, disabled, onChange }: {
  label: string; value: number; range: { min: number; max: number }; disabled: boolean; onChange: (value: number) => void;
}) {
  return <label className="workshop-field"><span>{label}</span><input type="number" step={1} min={range.min} max={range.max} disabled={disabled} value={Number.isNaN(value) ? "" : value} onChange={(event) => onChange(event.target.valueAsNumber)} /></label>;
}
