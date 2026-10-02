import { useEffect, useRef, useState } from "react";
import { ApiRequestError, createCardDraft, loadCardDraft, loadCardDrafts, loadCardRevisionTransfer, loadCatalog, updateCardDraft } from "../api";
import type { AccountProps } from "../appTypes";
import { TopNav } from "../components/common";
import { rarityLabel } from "../deckHelpers";
import type { AuthUser, CardKind, CatalogCard } from "../types";
import type { CardDefinition, CardDraft, CardDefinitionValidationError } from "../types/cardWorkshop";
import { CardDraftEditor, draftInputProblems } from "./CardDraftEditor";

type DraftBrowserState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; drafts: CardDraft[]; officialCards: CatalogCard[] };

type SaveFeedback =
  | { draftId: number; status: "saved" | "error"; message: string }
  | { draftId: number; status: "conflict"; message: string };

const KIND_LABELS: Record<CardKind["type"], string> = {
  unit: "Unit", spell: "Spell", item: "Item", building: "Building", manaSource: "Mana source",
};

export function CardDraftsPage({ currentUser, onNavigate, onSignOut }: AccountProps & { currentUser: AuthUser }) {
  const [state, setState] = useState<DraftBrowserState>({ status: "loading" });
  const [reload, setReload] = useState(0);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [officialId, setOfficialId] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Unsaved edits per Draft survive selection changes, failed saves and conflict recovery.
  const [edits, setEdits] = useState<Record<number, CardDefinition>>({});
  const [saving, setSaving] = useState(false);
  const [feedback, setFeedback] = useState<SaveFeedback | null>(null);
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);

  useEffect(() => {
    let cancelled = false;
    setState({ status: "loading" });
    Promise.all([loadCardDrafts(), loadCatalog()])
      .then(([response, catalog]) => {
        if (cancelled) return;
        setState({ status: "ready", drafts: response.drafts, officialCards: catalog.cards });
        setSelectedId(response.drafts[0]?.id ?? null);
        setOfficialId(catalog.cards[0]?.templateId ?? "");
      })
      .catch((error: unknown) => {
        if (!cancelled) setState({ status: "error", message: error instanceof Error ? error.message : "Could not load Card Drafts." });
      });
    return () => { cancelled = true; };
  }, [reload]);

  async function duplicate() {
    if (state.status !== "ready" || !officialId || busy) return;
    setBusy(true);
    setError(null);
    try {
      const source = await loadCardRevisionTransfer({ cardId: officialId, revision: 1 });
      if (!mounted.current) return;
      const draft = await createCardDraft({ ...source.revision.definition, id: `custom-${crypto.randomUUID()}` });
      if (!mounted.current) return;
      setState((current) => current.status === "ready" ? { ...current, drafts: [...current.drafts, draft] } : current);
      setSelectedId(draft.id);
    } catch (error) {
      if (mounted.current) setError(error instanceof Error ? error.message : "Could not duplicate the Card.");
    } finally {
      if (mounted.current) setBusy(false);
    }
  }

  function replaceDraft(draft: CardDraft) {
    setState((current) => current.status === "ready" ? { ...current, drafts: current.drafts.map((existing) => existing.id === draft.id ? draft : existing) } : current);
  }

  function discardEdits(draftId: number) {
    setEdits(({ [draftId]: _discarded, ...rest }) => rest);
    setFeedback(null);
  }

  async function save(draft: CardDraft) {
    const definition = edits[draft.id];
    if (!definition || saving) return;
    const problems = draftInputProblems(definition);
    if (problems.length > 0) {
      setFeedback({ draftId: draft.id, status: "error", message: problems.join(" ") });
      return;
    }
    setSaving(true);
    setFeedback(null);
    try {
      const saved = await updateCardDraft(draft.id, draft.version, definition);
      if (!mounted.current) return;
      replaceDraft(saved);
      setEdits(({ [draft.id]: _saved, ...rest }) => rest);
      setFeedback({ draftId: draft.id, status: "saved", message: `Draft saved (version ${saved.version}).` });
    } catch (error) {
      if (!mounted.current) return;
      const message = error instanceof Error ? error.message : "Could not save the Draft.";
      setFeedback({ draftId: draft.id, status: error instanceof ApiRequestError && error.status === 409 ? "conflict" : "error", message });
    } finally {
      if (mounted.current) setSaving(false);
    }
  }

  async function loadLatest(draftId: number) {
    if (saving) return;
    setSaving(true);
    try {
      const latest = await loadCardDraft(draftId);
      if (!mounted.current) return;
      replaceDraft(latest);
      setFeedback({ draftId, status: "saved", message: `Loaded saved version ${latest.version}. Your unsaved edits are kept; saving replaces that version.` });
    } catch (error) {
      if (mounted.current) setFeedback({ draftId, status: "conflict", message: error instanceof Error ? error.message : "Could not load the latest Draft." });
    } finally {
      if (mounted.current) setSaving(false);
    }
  }

  const selected = state.status === "ready" ? state.drafts.find((draft) => draft.id === selectedId) : undefined;
  const editing = selected ? edits[selected.id] ?? selected.definition : undefined;
  const dirty = selected ? selected.id in edits : false;
  const selectedFeedback = selected && feedback?.draftId === selected.id ? feedback : null;
  return (
    <main className="app-shell card-drafts-shell">
      <TopNav currentUser={currentUser} onNavigate={onNavigate} onSignOut={onSignOut} activePath="/workshop/cards" />
      <header className="card-drafts-heading"><p className="eyebrow">Card Workshop</p><h1>Card Drafts</h1><p>Your saved Cards belong to your Account.</p></header>
      {state.status === "loading" ? <p role="status">Loading Card Drafts…</p> : null}
      {state.status === "error" ? <div role="alert"><p>{state.message}</p><button className="secondary-link" onClick={() => setReload((value) => value + 1)}>Retry loading</button></div> : null}
      {state.status === "ready" ? <div className="card-drafts-layout">
        <section aria-label="Account Drafts" className="card-drafts-library">
          <h2>Your Drafts</h2>
          {state.drafts.length === 0 ? <p>No Card Drafts yet. Duplicate an official Card to get started.</p> : <ul className="card-drafts-list">{state.drafts.map((draft) => <li key={draft.id}><button className="secondary-link" aria-label={`Select ${draft.definition.name || "Unnamed Draft"}`} aria-pressed={selectedId === draft.id} onClick={() => setSelectedId(draft.id)}>{draft.definition.name || "Unnamed Draft"}<span>{draft.definition.cost} Mana · {KIND_LABELS[draft.definition.kind.type]}{draft.id in edits ? " · Unsaved changes" : ""}</span></button></li>)}</ul>}
          <h2>Duplicate an official Card</h2>
          <label className="workshop-field"><span>Official Card</span><select disabled={busy} value={officialId} onChange={(event) => setOfficialId(event.target.value)}>{state.officialCards.map((card) => <option key={card.templateId} value={card.templateId}>{card.name}</option>)}</select></label>
          <button className="primary-button" disabled={busy || !officialId} onClick={() => void duplicate()}>{busy ? "Duplicating…" : "Duplicate into Draft"}</button>
          {error ? <p role="alert">{error}</p> : null}
        </section>
        <section aria-label="Draft details" className="card-drafts-details">
          {selected && editing ? <>
            <h2>{selected.definition.name || "Unnamed Draft"}</h2>
            <dl className="card-drafts-stats"><div><dt>Cost</dt><dd>{selected.definition.cost} Mana</dd></div><div><dt>Rarity</dt><dd>{rarityLabel(selected.definition.rarity)}</dd></div><div><dt>Kind</dt><dd>{KIND_LABELS[selected.definition.kind.type]}</dd></div></dl>
            <p>{selected.definition.text}</p>
            {selected.validationErrors.length === 0 ? <p>Draft validation passed.</p> : <div role="alert"><p>This Draft has validation errors:</p><ul>{selected.validationErrors.map((error, index) => <li key={index}>{diagnosticMessage(error)}</li>)}</ul></div>}
            <h3>Edit Draft</h3>
            <CardDraftEditor definition={editing} disabled={saving} onChange={(definition) => { setEdits((current) => ({ ...current, [selected.id]: definition })); }} />
            <div className="card-draft-actions">
              <button className="primary-button" disabled={!dirty || saving} onClick={() => void save(selected)}>{saving ? "Saving…" : "Save Draft"}</button>
              {dirty ? <button className="secondary-link" disabled={saving} onClick={() => discardEdits(selected.id)}>Discard unsaved changes</button> : null}
            </div>
            {selectedFeedback?.status === "saved" ? <p role="status">{selectedFeedback.message}</p> : null}
            {selectedFeedback?.status === "error" ? <p role="alert">Draft was not saved: {selectedFeedback.message} Your unsaved edits are kept.</p> : null}
            {selectedFeedback?.status === "conflict" ? <div role="alert"><p>This Draft changed elsewhere: {selectedFeedback.message} Your unsaved edits are kept.</p><button className="secondary-link" disabled={saving} onClick={() => void loadLatest(selected.id)}>Load latest saved version</button></div> : null}
          </> : <p>Select a Draft to see its saved details.</p>}
        </section>
      </div> : null}
    </main>
  );
}

function diagnosticMessage(error: CardDefinitionValidationError): string {
  if (error.code === "invalidId") return `Card ID contains unsupported characters: ${error.value}.`;
  if (error.code === "blankName") return "Card name is required.";
  const field = error.field.replace(/^(kind|taxonomy)\./, "").replaceAll(".", " ").replace(/([a-z])([A-Z])/g, "$1 $2");
  switch (error.code) {
    case "invalidTaxonomyTag": return `${field}: invalid tag ${error.value}.`;
    case "duplicateTaxonomyTag": return `${field}: duplicate tag ${error.value}.`;
    case "negativeValue": return `${field} cannot be negative (saved value: ${error.value}).`;
    case "nonPositiveValue": return `${field} must be positive (saved value: ${error.value}).`;
    case "zeroValue": return `${field} cannot be zero.`;
    case "emptyStatChange": return `${field} has no effective stat change.`;
  }
}
