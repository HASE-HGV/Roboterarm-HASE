// Bearbeite hier die Texte und Menüeinträge. Du kannst neue Seiten hinzufügen oder bestehende ändern.
window.SITE_CONTENT = {
  // Setze hier deinen GitHub-Repo-Link
  repoUrl: 'https://github.com/catrick-cpu/HASE-Roboterarm',
  siteTitle: 'HASE Roboterarm',
  tagline: 'Präzision. Forschung. Robotik.',
  pages: [
    {
      id: 'home',
      title: 'Home',
      content: `
        <section class="hero-block">
          <h2>HASE Roboterarm</h2>
          <p>Präzise Steuerung und modulare Erweiterbarkeit für Forschung und Prototyping.</p>
          <p><a class="cta" href="${window.location.origin}">Mehr erfahren</a></p>
        </section>
        <section class="intro">
          <h3>Unsere Mission</h3>
          <p>Wir entwickeln einen offenen, modularen Roboterarm mit Fokus auf einfacher Integration in Forschung und Lehre.</p>
        </section>
      `,
    },
    {
      id: 'about',
      title: 'Über uns',
      content: `
        <h2>Über das Projekt</h2>
        <p>Das HASE-Projekt verbindet mechanisches Design, Motorsteuerung und zuverlässige Software, um Bildungs- und Forschungsprojekte zu unterstützen.</p>
        <ul>
          <li>Open Source Komponenten</li>
          <li>Modulare Motor-Controller</li>
          <li>Web-basiertes Interface</li>
        </ul>
      `,
    },
    {
      id: 'services',
      title: 'Features',
      content: `
        <h2>Features</h2>
        <div class="cards">
          <div class="card"><h4>Präzise Steuerung</h4><p>Feinsteuerung der Achsen mit Kalibrierungsroutinen.</p></div>
          <div class="card"><h4>Modular</h4><p>Erweiterbar mit Sensoren, Greifern und Kameras.</p></div>
          <div class="card"><h4>Web-Interface</h4><p>Steuerung und Monitoring per Browser.</p></div>
        </div>
      `,
    },
    {
      id: 'projects',
      title: 'Projekte',
      content: `
        <h2>Repos & Demos</h2>
        <p>Das zentrale Repository und Hilfsprojekte findest du auf GitHub.</p>
        <p><a href="${window.SITE_CONTENT ? window.SITE_CONTENT.repoUrl : '#'}" target="_blank">Zum GitHub Repository</a></p>
      `,
    },
    {
      id: 'contact',
      title: 'Kontakt',
      content: `
        <h2>Kontakt</h2>
        <p>Fragen, Kooperationen oder Supportanfragen per GitHub Issues oder Email.</p>
        <p>Email: <a href="mailto:info@example.com">info@example.com</a></p>
      `,
    },
  ],
};
