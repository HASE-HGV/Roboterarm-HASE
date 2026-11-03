% IMAGE-BUILDER-CONFIG(5) | Raspberry Pi OS Image Builder | Konfigurationsreferenz

# NAME

`config.yaml` — Build-Konfiguration für den Raspberry-Pi-OS-Image-Builder

# SYNOPSIS

```sh
docker compose -f image-builder/compose.yaml run --rm builder \
  build --input /input/raspios.img.xz \
  --config /project/config.yaml \
  --output /output/raspios-custom.img [--test]
```

Die Konfiguration ist eine YAML-Datei. Relative Quelldateien werden relativ
zum Verzeichnis der Konfigurationsdatei aufgelöst. Sie müssen im in den
Container eingebundenen Projektverzeichnis `/project` liegen.

# BESCHREIBUNG

Die Datei legt Architektur, Paketbehandlung, zusätzliche Dateien und Programme,
Netplan, systemd-Units und Hooks für einen Image-Build fest. Nicht angegebene
Abschnitte werden mit den unten genannten Standardwerten behandelt.

Die Quelldatei kann `.img` oder `.img.xz` sein. Eine `.xz`-Datei wird automatisch
entpackt. Der Builder verändert die Eingabedatei nicht und schreibt ein
unkomprimiertes Image an den mit `--output` angegebenen Ort.

# KONFIGURATION

## `image`

```yaml
image:
  max_size_gb: 16
  reserve_gb: 2
```

- `max_size_gb` (Zahl, Standard `16`): harte Obergrenze in dezimalen GB für
  das fertige Image. Werte über 16 werden abgelehnt. Während des Builds wird
  das Arbeitsimage bis zu dieser Grenze vergrößert, damit APT und zusätzliche
  Container-Images Platz haben. Danach wird das Root-Dateisystem auf seinen
  tatsächlichen Inhalt plus Reserve verkleinert.
- `reserve_gb` (Zahl, Standard `2`): freier Platz, der nach dem Build im
  Root-Dateisystem verbleibt. Wenn Inhalt plus Reserve die Obergrenze
  überschreiten, schlägt der Build mit einer Größenmeldung fehl.

Raspberry Pi OS vergrößert das Root-Dateisystem beim ersten Start üblicherweise
bis zur Kapazität der geflashten SD-Karte. Der Builder lässt diese
Erststart-Konfiguration erhalten. So bleibt die Image-Datei kompakt, während
das laufende System anschließend den verfügbaren Kartenspeicher nutzen kann.

## `architecture`

```yaml
architecture: auto
```

Zielarchitektur des Raspberry-Pi-OS-Images. Zulässige Werte sind `auto`,
`arm64` und `armhf`. `auto` liest die Architektur aus der dpkg-Datenbank des
Images. Das mitgelieferte `rustctl`-Binary wird derzeit nur für `arm64`
bereitgestellt. Für `armhf` muss `rustctl.enabled` auf `false` gesetzt werden.

## `hostname`

```yaml
hostname: robo
```

Optionaler Zielhostname; die mitgelieferten Starterkonfigurationen verwenden
`robo`. Der Builder schreibt den Namen nach `/etc/hostname` und aktualisiert
den Hosteintrag `127.0.1.1` in `/etc/hosts`. Zulässig ist ein DNS-Hostname mit
1 bis 63 Buchstaben, Ziffern oder Bindestrichen, ohne Bindestrich am Anfang
oder Ende. Wird `hostname` weggelassen, bleibt der Hostname aus dem Basisimage
unverändert.

## `packages`

```yaml
packages:
  update: true
  prune: false
  keep:
    - systemd
    - bash
    - sudo
  extra:
    - curl
```

- `update` (bool, Standard `true`): `apt-get update` und `apt-get full-upgrade`
  im Zielsystem ausführen.
- `keep` (Liste von Paketnamen, Standard leer): gewünschte Paketwurzeln. Fehlende
  Pakete werden installiert.
- `extra` (Liste von Paketnamen, Standard leer): zusätzliche gewünschte Pakete.
- `prune` (bool, Standard `false`): nicht mehr benötigte Pakete entfernen.

Bei `prune: true` markiert der Builder alle installierten Pakete als automatisch
installiert und `keep`, `extra` sowie automatisch hinzugefügte Netplan-Pakete
als manuell installiert. Danach führt er `apt-get autoremove --purge` aus.
APT behält essentielle und als `NeverAutoRemove` geschützte Pakete; übrige
Pakete können entfernt werden, wenn sie nicht zu den gewünschten Paketwurzeln
gehören. Soll etwa `sudo` erhalten bleiben, muss es in `keep` oder `extra`
stehen. Das Bereinigen ist absichtlich nicht standardmäßig aktiviert.

## `rustctl`

```yaml
rustctl:
  enabled: true
  install_path: /rustctl/rustctl
```

- `enabled` (bool, Standard `true`): das im Builder-Image kompilierte
  arm64-Programm installieren.
- `install_path` (Pfad im Zielimage, Standard `/rustctl/rustctl`): Zielpfad des
  Programms.

Das Binary wird mit ausführbaren Rechten angelegt. Einen automatischen
systemd-Dienst richtet dieser Abschnitt nicht ein; dazu `systemd.units`
verwenden.

## `files` und `tools`

Beide Abschnitte sind Listen gleich aufgebauter Einträge:

```yaml
files:
  - source: ./files/hostname
    destination: /etc/hostname
    mode: "0644"

tools:
  - source: ./tools/arm-init
    destination: /usr/local/bin/arm-init
    mode: "0755"
```

- `source` (erforderlich): Datei oder Verzeichnis relativ zur Konfiguration.
- `destination` (erforderlich): Zielpfad im Root-Dateisystem des Images.
- `mode` (optional): Dateimodus in oktaler Schreibweise. Bei Verzeichnissen
  wird der Modus auf den kopierten Zielpfad angewendet.

`files` bezeichnet üblicherweise Konfigurationsdateien; `tools` zusätzliche
ausführbare Programme oder Skripte. Beide werden kopiert, nicht kompiliert.

## `netplan`

```yaml
netplan:
  enabled: true
  renderer: NetworkManager
  configs:
    - source: ./netplan/10-robot.yaml
      destination: /etc/netplan/10-robot.yaml
      mode: "0600"
```

- `enabled` (bool, Standard `false`): Netplan-Unterstützung aktivieren.
- `renderer` (Standard `networkd`): `NetworkManager` oder `networkd`.
- `configs` (Liste): Netplan-YAML-Dateien, die ins Image kopiert werden.

Bei Aktivierung installiert der Builder `netplan.io` und das ausgewählte
Backend-Paket. Bei `NetworkManager` aktiviert er zusätzlich
`NetworkManager.service`, damit Netplan die YAML-Konfiguration beim Start an
den laufenden NetworkManager übergeben kann. Der Builder deaktiviert
`systemd-networkd` nicht automatisch, da Netplan verschiedene Geräte an
unterschiedliche Renderer übergeben kann.

Jede Quelldatei muss einen `network`-Abschnitt enthalten. Ihr effektiver
Renderer muss zu `netplan.renderer` passen; fehlt `renderer` in der
Netplan-Datei, gilt Netplans Standard `networkd`. Für NetworkManager muss daher
die Netplan-Datei `renderer: NetworkManager` angeben. Netplan-Einstellungen wie
Ethernet, WLAN und deren Verbindungen werden in den Quelldateien definiert.

Die enthaltene Netplan-Beispieldatei reserviert Ethernet-Adresse
`192.168.50.2/24` für ein direktes Mac-zu-Pi-Kabel; WLAN bleibt auf DHCP.
Stelle den USB-Ethernet-Dienst des Mac auf `192.168.50.1` mit der Netzmaske
`255.255.255.0` ein. Für diese direkte Verbindung bleiben Router und DNS leer.
Die WLAN-Verbindung des Pi stellt weiterhin den normalen Netzwerk- und
Internetzugang bereit.

Für ein WPA2-Personal-Netz verwendet Netplan `key-management: psk` und das
WLAN-Passwort unter `auth.password`. Zugangsdaten stehen damit im Quell-YAML
im Klartext; der Builder kopiert die Beispiel-WLAN-Datei mit Modus `0600` ins
Image. Passe SSID, Passwort und `regulatory-domain` an deinen Standort an.

## `systemd`

```yaml
systemd:
  units:
    - source: ./systemd/rustctl.service
      destination: /etc/systemd/system/rustctl.service
      enabled: true
    - name: bluetooth.service
      state: disabled
```

Jeder Eintrag muss entweder eine Unit-Datei über `source` und `destination`
installieren oder eine vorhandene Unit über `name` referenzieren.

- `enabled: true|false` wird verwendet, wenn `state` fehlt.
- `state`: `enabled`, `disabled` oder `masked`; hat Vorrang vor `enabled`.
- `enabled` und `disabled` werden mit `systemctl --root` im Image gesetzt.
- `masked` legt im Image einen Symlink der Unit auf `/dev/null` an.

## `hooks`

```yaml
hooks:
  mode: container
  after_packages:
    - ./hooks/configure-system.sh
  before_finalize: []
```

Hooks sind Bash-Skripte. `mode` muss derzeit `container` sein. Die Skripte
werden im Builder-Container ausgeführt, nicht im Ziel-chroot. Sie erhalten:

- `ROOTFS`: Mountpunkt des Ziel-Root-Dateisystems während des Hooks.
- `BUILD_STAGE`: `after_packages` oder `before_finalize`.

Die Hook-Listen sind optional und standardmäßig leer. `ROOTFS` ist nach Ende
des Hooks nicht mehr verfügbar.

## `test`

```yaml
test:
  shell: false
```

`shell: true` öffnet vor dem Finalisieren eine interaktive chroot-Bash im
Zielsystem. Alternativ kann `--test` auf der CLI verwendet werden. Mit `exit`
wird der Build fortgesetzt. Für ARM-Images auf x86 ist QEMU/binfmt-Unterstützung
erforderlich.

## `guacamole`

```yaml
guacamole:
  enabled: false
  username: root
  password: root
  ssh_host: host.docker.internal
  ssh_port: 22
  web_port: 2000
  bind_address: 127.0.0.1
  target_bind_address: 127.0.0.1
  version: "1.6.0"
```

- `enabled` (bool, Standard `false`): Docker und Guacamole im Zielimage
  installieren, beim Systemstart starten und zusätzlich ein lokales
  Compose-Testbundle neben dem Image erzeugen. Das Image muss ARM64 sein.
- `username`, `password` (Standard jeweils `root`): Login für die
  Guacamole-Weboberfläche.
- `ssh_host` (Standard `host.docker.internal`) und `ssh_port` (Standard `22`):
  SSH-Ziel, das der Guacamole-Container erreicht. `host.docker.internal` wird
  im Pi-Image auf das Docker-Host-Gateway gesetzt und verbindet Guacamole mit
  dem SSH-Server des Pi.
- `web_port` (Standard `2000`): Port der Guacamole-Weboberfläche.
- `bind_address` (Standard `127.0.0.1`): IPv4-Adresse, an die Docker den
  Web-Port im optionalen lokalen Compose-Testbundle bindet.
- `target_bind_address` (Standard `127.0.0.1`): IPv4-Adresse, an die Docker im
  Pi-Image den Web-Port bindet. Für direkten Zugriff aus dem LAN kann sie auf
  `0.0.0.0` gesetzt werden; verwende dann unbedingt eigene Guacamole-Login-Daten.
- Die mitgelieferte `basic-config.yaml` setzt `target_bind_address: 0.0.0.0`,
  damit die Oberfläche direkt unter `http://<ip-adresse-des-pi>:2000/` erreichbar
  ist. Sie verwendet zunächst `root`/`root`; ändere diese Zugangsdaten vor dem
  Betrieb in einem gemeinsam genutzten oder nicht vertrauenswürdigen Netz.
- `version` (Standard `1.6.0`): gemeinsamer Tag der offiziellen
  `guacamole/guacamole`- und `guacamole/guacd`-Images.

Bei `enabled: true` lädt der Builder die offiziellen ARM64-Container-Images
während des Builds herunter und legt sie im Image unter
`/opt/guacamole/images/` ab. Das Zielimage enthält Docker, `openssh-server`,
die Guacamole-Konfiguration und einen aktivierten `guacamole.service`.
Beim ersten Boot importiert der Dienst die vorbereiteten Images in Docker und
startet `guacd` sowie die Webanwendung. Der Pi muss dazu keine Images aus dem
Internet laden. Das vergrößert das Image deutlich und Docker braucht zusätzliche
Speicher- und CPU-Ressourcen.

Standardmäßig lauscht Guacamole nur auf dem Pi selbst, damit die voreingestellten
Zugangsdaten nicht im LAN offenliegen. Vom Rechner aus öffnest du einen SSH-Tunnel
mit `ssh -L 2000:127.0.0.1:2000 <pi-benutzer>@<ip-adresse-des-pi>` und rufst
`http://127.0.0.1:2000/` auf. Für direkten Zugriff über
`http://<ip-adresse-des-pi>:2000/` setze `target_bind_address: 0.0.0.0` und
ändere `username`/`password` vor dem Build. Der Builder erzeugt weiterhin
`<image-name>-guacamole/` mit `compose.yaml` und
`user-mapping.xml`, damit sich die Verbindung separat auf einem Docker-Rechner
lokal testen lässt. Dieses optionale Testbundle lädt Container beim Start aus
dem Internet.

`username`/`password` gelten für den Guacamole-Weblogin und werden zugleich mit
`${GUAC_USERNAME}` und `${GUAC_PASSWORD}` als SSH-Benutzername und Passwort
weitergereicht. Dadurch fragt die Verbindung nicht noch einmal nach Zugangsdaten.
Wenn `username: root` gesetzt ist, legt der Builder das konfigurierte Passwort
für das vorhandene Root-Konto fest und aktiviert Passwort-SSH-Login für root,
damit die Standardkonfiguration funktioniert. Bei einem anderen Benutzernamen
muss dieser Linux-Account bereits existieren und dasselbe Passwort akzeptieren.
Diese Credential-Tokens werden von [Apache Guacamole](https://guacamole.apache.org/doc/gug/configuring-guacamole.html#parameter-tokens)
unterstützt. Die Weblogin-Passwortdatei wird im XML mit MD5-Encoding abgelegt.
Apache beschreibt die Datei-Authentifizierung als einfache Lösung für kleine
Setups und rät davon ab, `user-mapping.xml` für öffentlich erreichbare
Installationen zu verwenden. Ändere `root`/`root` vor dem Flashen, wenn der Pi
aus dem LAN erreichbar ist.

# VOLLSTÄNDIGES MINIMALBEISPIEL

```yaml
architecture: auto
hostname: robo

packages:
  update: true
  prune: false
  keep: [systemd, coreutils, bash, sudo, openssh-server]
  extra: []

rustctl:
  enabled: true
  install_path: /rustctl/rustctl

files: []
tools: []
netplan:
  enabled: false
systemd:
  units:
    - source: ./systemd/rustctl.service
      destination: /etc/systemd/system/rustctl.service
      enabled: true
hooks:
  mode: container
  after_packages: []
  before_finalize: []
test:
  shell: false
```

# DATEIEN

- `basic-config.yaml` — aktualisieren und `rustctl` installieren, bestehende
  Paketmenge erhalten.
- `minimal-config.yaml` — Beispiel mit aktivierter Paketbereinigung.
- `examples/config.yaml` — Beispiel für Dateien, Netplan, systemd und Hooks.

# SIEHE AUCH

`../README.md#77-building-a-raspberry-pi-os-image`, `basic-config.yaml`,
`minimal-config.yaml`.
