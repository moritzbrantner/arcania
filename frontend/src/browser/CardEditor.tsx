import { useState } from "react";
import { Plus, Trash2 } from "lucide-react";
import type { CatalogCard, CardKind } from "../types";
import type { CustomCard } from "./engine";
import { NumberField } from "./RuleEditor";

function newCard(): CustomCard {
  return { id: `custom-${crypto.randomUUID()}`, name: "Runic Guardian", rarity: "basic", cost: 2, text: "A guardian for your arena.", kind: { type: "unit", attack: 2, armor: 4, maxAp: 2 } };
}

export function CardEditor({ cards, catalog, onChange }: { cards: CustomCard[]; catalog: CatalogCard[]; onChange: (cards: CustomCard[]) => void | Promise<boolean> }) {
  const [draft, setDraft] = useState<CustomCard>(newCard);
  const [source, setSource] = useState("");
  const [advanced, setAdvanced] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const editing = cards.some((card) => card.id === draft.id);
  function changeKind(kind: CardKind) { setDraft({ ...draft, kind }); }
  const kind = draft.kind;
  async function applyCard() {
    try {
      const card = advanced === null ? draft : { ...draft, kind: JSON.parse(advanced) };
      // The parent validates the entire preset with Rust before accepting edits.
      const accepted = await onChange(editing ? cards.map((existing) => existing.id === card.id ? card : existing) : [...cards, card]);
      if (accepted !== false) { setDraft(card); setError(null); }
    } catch (error) { setError(String(error)); }
  }
  return <section className="workshop-card-editor">
    <div><h2>Card editor</h2><p>Create a Unit or Spell, or duplicate any existing card. Each custom card is added to both opening hands.</p></div>
    <div className="workshop-card-columns">
      <div className="workshop-card-form">
        <label className="workshop-field"><span>Start from a card</span><select value={source} onChange={(event) => {
          setSource(event.target.value);
          const card = catalog.find((entry) => entry.id === event.target.value);
          if (!card) { setDraft(newCard()); setAdvanced(null); }
          if (card) { setDraft({ id: `custom-${crypto.randomUUID()}`, name: `${card.name} Variant`, rarity: card.rarity, cost: card.cost, text: card.text, kind: card.kind }); setAdvanced(null); }
        }}><option value="">New card</option>{catalog.map((card) => <option key={card.id} value={card.id}>{card.name}</option>)}</select></label>
        <label className="workshop-field"><span>Card name</span><input required maxLength={80} value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} /></label>
        <div className="workshop-fields">
          <NumberField label="Mana cost" value={draft.cost} min={0} max={30} onChange={(cost) => setDraft({ ...draft, cost })} />
          <label className="workshop-field"><span>Card type</span><select value={kind.type} onChange={(event) => {
            if (event.target.value === "unit") { changeKind({ type: "unit", attack: 2, armor: 4, maxAp: 2 }); }
            if (event.target.value === "spell") { changeKind({ type: "spell", range: 2, priority: 2, effect: { type: "damage", amount: 2 } }); }
            setAdvanced(null);
          }}><option value="unit">Unit</option><option value="spell">Spell</option>{kind.type !== "unit" && kind.type !== "spell" ? <option value={kind.type}>{kind.type}</option> : null}</select></label>
        </div>
        {kind.type === "unit" ? <div className="workshop-fields">
          <NumberField label="Unit attack" value={kind.attack} min={0} max={100} onChange={(attack) => changeKind({ ...kind, attack })} />
          <NumberField label="Unit armor" value={kind.armor} max={100} onChange={(armor) => changeKind({ ...kind, armor })} />
          <NumberField label="Unit AP" value={kind.maxAp} max={100} onChange={(maxAp) => changeKind({ ...kind, maxAp })} />
        </div> : null}
        {kind.type === "spell" ? <>
          <div className="workshop-fields">
            <NumberField label="Spell range" value={kind.range} min={0} max={6} onChange={(range) => changeKind({ ...kind, range })} />
            <NumberField label="Spell priority" value={kind.priority} min={0} max={10} onChange={(priority) => changeKind({ ...kind, priority })} />
          </div>
          <label className="workshop-field"><span>Spell effect</span><select value={kind.effect.type} onChange={(event) => {
            const type = event.target.value;
            if (type === "damage" || type === "heal" || type === "draw") { changeKind({ ...kind, effect: { type, amount: 2 } }); }
          }}><option value="damage">Damage</option><option value="heal">Heal</option><option value="draw">Draw cards</option>{!["damage", "heal", "draw"].includes(kind.effect.type) ? <option value={kind.effect.type}>{kind.effect.type}</option> : null}</select></label>
          {"amount" in kind.effect ? <NumberField label="Effect amount" value={kind.effect.amount} max={100} onChange={(amount) => changeKind({ ...kind, effect: { ...kind.effect, amount } as typeof kind.effect })} /> : null}
        </> : null}
        <label className="workshop-field"><span>Card description</span><textarea maxLength={500} value={draft.text} onChange={(event) => setDraft({ ...draft, text: event.target.value })} /></label>
        <details onToggle={(event) => { if (event.currentTarget.open) { setAdvanced(JSON.stringify(draft.kind, null, 2)); } else { setAdvanced(null); } }}><summary>Advanced effect editor</summary><p>Edit the existing Unit, Spell, Item or Building effect data. Effects are validated when added.</p><textarea aria-label="Card effect JSON" rows={12} value={advanced ?? ""} onChange={(event) => setAdvanced(event.target.value)} /></details>
        {error ? <p role="alert">{error}</p> : null}
        <button className="primary-button" type="button" disabled={!editing && cards.length >= 8} onClick={applyCard}><Plus size={18} />{editing ? "Update card" : "Add card"}</button>
      </div>
      <aside className="workshop-card-preview" aria-label="Card preview">
        <div className="workshop-preview-art"><img src={`${import.meta.env.BASE_URL}workshop/rune-field.svg`} alt="" /><span>{draft.cost}</span><strong>{kind.type === "unit" ? "✦" : "✧"}</strong></div>
        <p className="eyebrow">{draft.rarity} · {kind.type}</p><h3>{draft.name || "Untitled card"}</h3><p>{draft.text}</p>
        {kind.type === "unit" ? <dl><div><dt>Attack</dt><dd>{kind.attack}</dd></div><div><dt>Armor</dt><dd>{kind.armor}</dd></div><div><dt>AP</dt><dd>{kind.maxAp}</dd></div></dl> : null}
      </aside>
    </div>
    <h3>Your custom cards · {cards.length}/8</h3>
    {cards.length === 0 ? <p>No custom cards yet. Your first creation will appear here.</p> : <ul className="workshop-card-list">{cards.map((card) => <li key={card.id}><button type="button" onClick={() => { setDraft(card); setAdvanced(null); }}>{card.name}<small>{card.cost} Mana · {card.kind.type}</small></button><button className="icon-button" aria-label={`Remove ${card.name}`} type="button" onClick={() => onChange(cards.filter((entry) => entry.id !== card.id))}><Trash2 size={18} /></button></li>)}</ul>}
    {editing ? <button className="secondary-link" type="button" onClick={() => { setDraft(newCard()); setSource(""); setAdvanced(null); }}>Create another card</button> : null}
  </section>;
}
