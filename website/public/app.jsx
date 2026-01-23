const { useState, useEffect } = React;

function App() {
  const content = window.SITE_CONTENT || { pages: [], repoUrl: '#' };
  const [active, setActive] = useState(content.pages && content.pages[0] ? content.pages[0].id : null);

  useEffect(() => {
    if (!active && content.pages && content.pages[0]) setActive(content.pages[0].id);
  }, [active, content.pages]);

  // no theme handling — keep layout simple and readable

  const page = (content.pages || []).find((p) => p.id === active) || null;

  function handleNav(id) {
    const el = document.getElementById(id);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
      setActive(id);
    }
  }

  // theme toggle removed

  return (
    <div className="page">
      <header className="topnav">
        <div className="nav-inner">
          <div className="logo">{content.siteTitle || 'Company'}</div>
          <nav className="navlinks">
            {(content.pages || []).map((p) => (
              <a key={p.id} className={p.id === active ? 'active' : ''} onClick={() => handleNav(p.id)}>{p.title}</a>
            ))}
            <a className="repo-link" href={content.repoUrl} target="_blank" rel="noreferrer">GitHub</a>
          </nav>
        </div>
      </header>

      <main className="container corporate">
        {(content.pages || []).map((p) => (
          <section key={p.id} id={p.id} className={"panel glass section-" + p.id} dangerouslySetInnerHTML={{ __html: p.content }} />
        ))}
      </main>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(React.createElement(App));
