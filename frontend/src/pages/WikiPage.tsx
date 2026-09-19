import { BookOpen, ChevronRight, House, LibraryBig, List } from "lucide-react";
import type { AccountProps } from "../appTypes";
import { TopNav } from "../components/common";
import { WikiScenePanel } from "../wiki/WikiScenePanel";
import { WIKI_TOPICS, wikiTopicById, type WikiTopic } from "../wiki/wikiContent";
import { wikiSceneById } from "../wiki/wikiScenes";

type WikiPageProps = {
  topicSlug: string | null;
} & AccountProps;

export function WikiPage({ topicSlug, currentUser, onNavigate, onSignOut }: WikiPageProps) {
  const topic = topicSlug ? wikiTopicById(topicSlug) : null;

  if (topicSlug && !topic) {
    return (
      <main className="app-shell wiki-shell">
        <TopNav currentUser={currentUser} onNavigate={onNavigate} onSignOut={onSignOut} activePath="/wiki" />
        <section className="shell-message wiki-not-found" aria-label="Rules topic not found">
          <p className="eyebrow">Rules Wiki</p>
          <h1>Rules topic not found</h1>
          <div className="actions shell-actions">
            <button className="primary-button" type="button" onClick={() => onNavigate("/wiki")}>
              <BookOpen size={18} />
              Open Rules
            </button>
            <button className="secondary-link" type="button" onClick={() => onNavigate("/")}>
              <House size={18} />
              Dashboard
            </button>
          </div>
        </section>
      </main>
    );
  }

  return (
    <main className="app-shell wiki-shell">
      <TopNav currentUser={currentUser} onNavigate={onNavigate} onSignOut={onSignOut} activePath="/wiki" />
      {topic ? <WikiTopicPage topic={topic} onNavigate={onNavigate} /> : <WikiOverview onNavigate={onNavigate} />}
    </main>
  );
}

function WikiOverview({ onNavigate }: { onNavigate: (to: string) => void }) {
  return (
    <section className="wiki-layout" aria-label="Rules wiki">
      <header className="wiki-hero">
        <div>
          <p className="eyebrow">Arcania</p>
          <h1>Rules Wiki</h1>
          <p>
            Core match rules for defeating the enemy Hero on the hex board, from turn flow and
            Mana to Combat, Buildings, and Deck recipe limits.
          </p>
        </div>
        <div className="wiki-actions">
          <button className="primary-button" type="button" onClick={() => onNavigate("/tutorial")}>
            <BookOpen size={18} />
            Tutorial
          </button>
          <button className="secondary-link" type="button" onClick={() => onNavigate("/catalog/")}>
            <LibraryBig size={18} />
            Card Catalog
          </button>
        </div>
      </header>

      <section className="wiki-topic-grid" aria-label="Rules topics">
        {WIKI_TOPICS.map((topic) => (
          <button
            className="wiki-topic-card"
            type="button"
            key={topic.id}
            aria-label={`Open ${topic.title}`}
            onClick={() => onNavigate(`/wiki/${topic.id}`)}
          >
            <span>
              <strong>{topic.title}</strong>
              <span>{topic.summary}</span>
            </span>
            <ChevronRight size={18} aria-hidden="true" />
          </button>
        ))}
      </section>
    </section>
  );
}

function WikiTopicPage({ topic, onNavigate }: { topic: WikiTopic; onNavigate: (to: string) => void }) {
  const scene = wikiSceneById(topic.sceneId);

  return (
    <section className="wiki-layout wiki-detail-layout" aria-label={`${topic.title} rules`}>
      <WikiTopicNav activeTopicId={topic.id} onNavigate={onNavigate} />
      <article className="wiki-article">
        <header className="wiki-article-header">
          <p className="eyebrow">Rules Wiki</p>
          <h1>{topic.title}</h1>
          <p>{topic.summary}</p>
        </header>

        {scene ? (
          <WikiScenePanel scene={scene} />
        ) : import.meta.env.DEV ? (
          <p className="wiki-scene-missing" role="status">
            Missing wiki scene for {topic.sceneId}.
          </p>
        ) : null}

        <section className="wiki-article-section">
          <h2>Key Rules</h2>
          <ul>
            {topic.keyRules.map((rule) => (
              <li key={rule}>{rule}</li>
            ))}
          </ul>
        </section>

        <section className="wiki-article-section">
          <h2>Example</h2>
          <p>{topic.example}</p>
        </section>

        <section className="wiki-article-section">
          <h2>Common Mistakes</h2>
          <ul>
            {topic.commonMistakes.map((mistake) => (
              <li key={mistake}>{mistake}</li>
            ))}
          </ul>
        </section>

        <section className="wiki-article-section">
          <h2>Related Topics</h2>
          <div className="wiki-related-links">
            {topic.relatedTopicIds.map((topicId) => {
              const relatedTopic = wikiTopicById(topicId);
              return relatedTopic ? (
                <button
                  className="secondary-link"
                  type="button"
                  key={topicId}
                  onClick={() => onNavigate(`/wiki/${topicId}`)}
                >
                  <ChevronRight size={17} />
                  {relatedTopic.title}
                </button>
              ) : null;
            })}
          </div>
        </section>
      </article>
    </section>
  );
}

function WikiTopicNav({
  activeTopicId,
  onNavigate,
}: {
  activeTopicId: WikiTopic["id"];
  onNavigate: (to: string) => void;
}) {
  return (
    <nav className="wiki-topic-nav" aria-label="Rules topics">
      <button className="secondary-link wiki-overview-link" type="button" onClick={() => onNavigate("/wiki")}>
        <List size={17} />
        All Rules
      </button>
      <div>
        {WIKI_TOPICS.map((topic) => (
          <button
            className={topic.id === activeTopicId ? "active" : ""}
            type="button"
            key={topic.id}
            disabled={topic.id === activeTopicId}
            onClick={() => onNavigate(`/wiki/${topic.id}`)}
          >
            {topic.title}
          </button>
        ))}
      </div>
    </nav>
  );
}
