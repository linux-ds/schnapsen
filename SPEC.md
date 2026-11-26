# GNOME Schnapsen – Projektspezifikation

**Status:** Konzept / Grundlage für die erste Implementierung  
**Zielversion:** 0.1 (spielbarer Prototyp)  
**Plattform:** GNOME auf Linux, Verteilung als Flatpak  
**Sprache dieser Spezifikation:** Deutsch

## 1. Projektziel

Entwicklung eines nativen, schlanken Schnapsenspiels für GNOME. Das Spiel soll sich wie ein physischer Kartentisch in Draufsicht anfühlen, ohne eine klassische Game-Engine oder eine physikalische Simulation einzusetzen. Im Vordergrund stehen korrekte Spielregeln, natürliche Bedienung und ein nachvollziehbarer Spielzustand. Grafische Perfektion ist **kein** Ziel der ersten Version.

Die funktionale Entwicklung bis zur Beta-Abnahme hat Vorrang. Historische Grafik, Audio und zusätzliche Animationen folgen in einer eigenen Phase. Die Reihenfolge der Aufträge und die offenen Prüfungen stehen in [PLAN.md](PLAN.md).

### Leitprinzipien

1. **Physisch nachvollziehbar, nicht physikalisch simuliert.** Karten, Talon und Stichstapel verhalten sich erkennbar wie reale Gegenstände; aufwändige Flugbahnen sind unnötig.
2. **Regellogik ist die Quelle der Wahrheit.** Die Oberfläche darf keine Spielregeln implementieren oder den Spielzustand unabhängig verändern.
3. **Grafik ist austauschbar.** Einfache Platzhalterkarten und spätere historische Kartenbilder verwenden dieselbe Logik, Geometrie und Interaktion.
4. **Native GNOME-Anwendung.** Ressourcenschonend, offline nutzbar und per Flatpak verteilbar.
5. **Zugänglich und kontrollierbar.** Maus, Tastatur und Touch sind gleichwertig; Animationen und Spielhilfen sind einstellbar.

## 2. Produktumfang

### Im ersten spielbaren Umfang

- Klassisches Schnapsen mit **20 Karten** und zwei Spielern (Mensch gegen Computer).
- Wahl zwischen **weichem** und **scharfem** Schnapsen; exakte Unterschiede müssen vor Fertigstellung als nachvollziehbare Regelprofile festgelegt und getestet werden.
- Doppeldeutsche Kartenfarben: **Herz, Schellen, Grün/Laub, Eichel**.
- Werte: **Ass, Zehn, König, Ober, Unter**.
- Karten geben, Talon/Trumpf, gültige Spielzüge, Stichentscheidung, Ziehen, Ansagen (20/40), Trumpftausch, Zudrehen, Punktewertung, Spielende und nächste Runde.
- Regelkonformer Computergegner mit drei Spielstärken und einer gemeinsamen öffentlichen Beobachtung.
- Tischdarstellung mit eigener Hand, kleiner verdeckter Gegnerhand, Talon mit Trumpfkarte, Tischmitte und sichtbaren gewonnenen Stichstapeln.
- Auswahl einzelner Karten, gegebenenfalls zweier Karten zur Vorbereitung einer Ansage.
- Ausspielen per Tastatur und Touchgeste sowie eindeutige Mausbedienung.
- Einsicht in Stichstapel entsprechend den gewählten Regeln; optionale Spielhilfen.
- Visuelle Rückmeldung für Auswahl und Stapel. Zusätzliche Animationen folgen in der grafischen Verfeinerungsphase.
- Lokale Einstellungen und automatische Fortsetzung der zuletzt gespeicherten Partie. Das Spiel ist offline nutzbar.

### Nicht Bestandteil der ersten Version

- Online-Multiplayer, Konten, Server und Datenbankdienst.
- 3D-Tisch, Physiksimulation, komplexe Partikeleffekte oder Game-Engine.
- Fotorealistische Kartenbewegungen.
- Neuronale Netze oder aufwendige Monte-Carlo-KI.
- Internationale Regelvarianten wie deutsches 66.
- Ausgearbeitetes historisches Kartenbild als Voraussetzung für einen spielbaren Prototyp.

## 3. Technik und Architektur

### Technologiestack

- **Rust** als Programmiersprache.
- **GTK4** für Eingaben, Darstellung und Fensterintegration.
- **libadwaita** für GNOME-konforme Anwendungselemente, Einstellungen und Dialoge.
- **Flatpak** als Distributionsformat.
- Keine klassische Game-Engine.

### Logische Module

```text
src/
  core/       Karten, Zustand, Aktionen, Regeln, Wertung
  ai/         Strategie des Computergegners
  ui/         Kartentisch, Zeichnen, Eingaben, Animationen, Overlays
  app/        Anwendung, Einstellungen, Sitzungsverwaltung
  main.rs
```

Dies ist eine **logische** Struktur, keine Verpflichtung zu einer bestimmten Anzahl von Crates. Möglichst einfach beginnen.

### Architekturgrenzen

- `core` darf **nicht** von GTK, Grafikressourcen oder Animationen abhängen.
- Änderungen am Spiel erfolgen ausschließlich über geprüfte Aktionen und Zustandsübergänge.
- `ai` erhält nur die Informationen, die ein menschlicher Spieler in derselben Situation legal kennen dürfte; kein Zugriff auf verdeckte Gegnerkarten im Entscheidungsmodell.
- `ui` rendert den aktuellen Zustand und visualisiert Ereignisse, ohne selbst über Stichgewinner oder Punkte zu entscheiden.
- Kartengrafiken sind über ein austauschbares Theme-/Renderer-Konzept angebunden.
- Zufällige Kartenverteilung soll für Tests über einen steuerbaren Zufallszahlengenerator reproduzierbar sein.

### Spielvariante und Partiekonfiguration

`core::rules` unterscheidet die Spielvariante (`GameVariant`) vom Regelprofil (`RuleProfile`). Aktuell ist österreichisches Zweierschnapsen in den Profilen Weich und Scharf verfügbar. Die Variante liefert Kartenbestand, Kartenwerte, Rangfolge, Teilnehmerzahl, Handgröße, Austeilen, Nachziehreihenfolge, Stichregeln sowie die Werte für den letzten Stich und den Partiestart. Die Profile bestimmen die profilabhängige Handwertung einschließlich der Wienerisch-Wertung. Die vorhandene Engine bleibt auf zwei Spieler spezialisiert. Die Aufnahme einer Variante mit anderer Spielerzahl erfordert einen eigenen Spielzustand und eine passende Tischansicht.

Die Profile bündeln die bestehenden Hausregeln für Ansagen und deren Wirksamkeit, Talonaktionen, Handabschluss, Zudrehwertung, Sticheinsicht und erlaubte Spielhilfen. Die manuelle Ausmeldung ist eine mögliche spätere Alternative zum automatischen Abschluss. Sie ist derzeit nicht auswählbar.

`MatchConfig` bindet Spielvariante, Regelprofil, Schneiderwertung und Computer-Spielstärke an eine Partie. Jede weitere Hand derselben Partie übernimmt diese Konfiguration. Das Menü **Partie** zeigt das unterstützte Spiel und seine Profile. Die dort gewählte Konfiguration greift erst mit **Neue Partie**. Lernhilfen sind davon unabhängige lokale Einstellungen innerhalb der vom aktiven Profil erlaubten Grenzen.

### Schnittstellen für Darstellung und Strategie

- Die Kartenzeichnung erhält Kartenidentität, Vorder- oder Rückseite und Darstellungszustand. Kartenfront und Kartenrücken werden unabhängig gewählt. Ein Wechsel wirkt sofort und erhält Kartenauswahl, Geometrie, Hand und Partiekonfiguration.
- Audio und Animationen können auf geprüfte Aktionen und ihre sichtbaren Ergebnisse reagieren. Der fachliche Zustand bleibt in der Engine.
- Die Computerstrategie entscheidet anhand der öffentlichen `Observation` zwischen von der Engine gelieferten legalen Karten, Ansagen, Trumpftausch und Zudrehen. Die Engine überprüft die gewählte Aktion.
- Persönliche Einstellungen gehören zur Anwendung und Darstellung. Spielvariante, Regelprofil und Schneiderwertung werden als eigene Konfiguration an die Partie übergeben.

### Kartendesigns und persönliche Einstellungen

`ui::design` registriert Kartenfronten und Kartenrücken getrennt. Die programmatisch gezeichneten Fronten Doppeldeutsch und Französisch arbeiten auf demselben Kartenmodell. Französisch stellt Schellen als Karo, Grün als Pik, Eichel als Kreuz sowie Ober und Unter als Dame und Bube dar. Herz bleibt Herz. Der gemeinsame Cairo-Renderer zeichnet Karten am Tisch, Trumpfsymbole, die Stichübersicht und Vorschauen der Kartenrücken.

Das native Einstellungsfenster speichert Designs und Spielhilfen über GSettings. Die Spielhilfen unterliegen weiterhin dem aktiven Regelprofil. Unbekannte Design-IDs fallen auf Doppeldeutsch beziehungsweise den bisherigen Kartenrücken zurück. Der Schlüssel für Audio ist vorbereitet. Eine Audiooption erscheint erst mit der Wiedergabe.

Für einen weiteren Rücken kann beispielsweise ein Modul mit einer Cairo-Zeichenfunktion ergänzt und ein Eintrag mit unveränderlicher ID, Anzeigename und Renderer in `BACKS` registriert werden. Eine weitere Front wird analog in `FRONTS` registriert und liefert zusätzlich Bezeichnungen und Farbsymbole. Ein späterer Renderer kann etwa vorbereitete SVG-Ressourcen in den übergebenen Cairo-Kontext zeichnen. Änderungen am Kartenmodell oder an den Spielregeln sind dafür nicht erforderlich.

### Internationalisierung

Die Oberfläche folgt der System-Locale. Deutsch verwendet österreichische Schnapsen-Terminologie. Englisch ist zugleich die Quellsprache und der Rückfall für Sprachen ohne Katalog. Eine vom System gesetzte gettext-Sprachpriorität (`LANGUAGE`) wird berücksichtigt.

`src/i18n.rs` bindet GNU gettext im Anwendungsprozess. Die Oberfläche übersetzt Spielereignisse, Abschlussgründe, Designnamen, Kartenbezeichnungen und Cairo-Beschriftungen bei der Ausgabe. Varianten, Regelprofile und Karten bleiben fachliche Typen. Die technischen Design-IDs und GSettings-Werte ändern sich mit der Sprache nicht. Desktop-Datei und AppStream entstehen über Meson aus englischen Vorlagen und den Katalogen.

Neue sichtbare Texte werden mit `tr!` markiert. Dynamische Texte verwenden vollständige Sätze mit benannten Platzhaltern, deren Reihenfolge der Katalog ändern kann. `trn!` übergibt den Zähler an gettext und stellt ihn als `{count}` bereit. Statische Einträge in Designbeschreibungen werden mit `msgid!` markiert und bei der Verwendung übersetzt.

| Deutsch | Englisch | Bedeutung |
|---|---|---|
| Schnapsen | Schnapsen | Spielname |
| Bummerl, Schneiderbummerl | Bummerl, Schneider | Verlust einer Partie, bei der Schneiderwertung zwei Bummerl |
| Schneider, Schwarz | Schneider, Schwarz | Zwei beziehungsweise drei Spielpunkte bei der entsprechenden Handwertung |
| Talon, Zudrehen | stock, close the stock | Nachziehstapel und seine Sperre |
| Augen, Spielpunkte | card points, game points | Punkte aus Karten und Ansagen beziehungsweise Wertung einer Hand |
| Stich, Ansage | trick, marriage | Ausgespieltes Kartenpaar beziehungsweise Ansage eines gleichfarbigen König-Ober-Paars |
| Weich, Scharf | Soft Schnapsen, Sharp Schnapsen | Regelprofile |
| Ober, Unter | Ober, Unter | Doppeldeutsche Kartenwerte. Im französischen Thema lauten sie Queen und Jack. |

#### Übersetzungen pflegen

1. Neue Quelldateien in `po/POTFILES` eintragen. Im GNOME SDK die Meson-Ziele `schnapsen-pot` und `schnapsen-update-po` ausführen, etwa mit `meson compile -C BUILD schnapsen-update-po`.
2. `po/de.po` aktualisieren und benannte Platzhalter erhalten. Der englische Katalog enthält die Quelltexte. Mit `msginit --no-translator --locale=en -i po/schnapsen.pot -o BUILD/en-base.po` und `msgen --lang=en BUILD/en-base.po -o po/en.po` neu erzeugen. Er stellt sicher, dass Englisch auch in einer Sprachpräferenzliste vor Deutsch gewählt werden kann.
3. Beide Kataloge mit `msgfmt --check --statistics -o /dev/null po/de.po` beziehungsweise `po/en.po` prüfen. Für weitere Sprachen einen Katalog aus der POT-Datei anlegen und den Sprachcode in `po/LINGUAS` aufnehmen.
4. Cargo baut für Entwicklung und Tests eigene MO-Dateien. Meson installiert die Kataloge im Locale-Verzeichnis. Das Flatpak enthält beide Kataloge direkt.

## 4. Datenmodell und Spielmechanik

### Karten

Jede Karte besteht aus Farbe und Wert; insgesamt 4 × 5 = 20 eindeutige Karten. Augenwerte:

| Wert | Augen |
|---|---:|
| Ass | 11 |
| Zehn | 10 |
| König | 4 |
| Ober | 3 |
| Unter | 2 |

Die Kartenidentität ist unabhängig vom gewählten Kartenbild. Die Karte kennt weder Bildschirmposition noch Animation.

### Spielzustand

Der Zustand enthält mindestens:

- Karten auf beiden Händen (mit getrennten Sichtbarkeitsrechten), Talon und offene Trumpfkarte.
- Trumpffarbe, aktuellen Ausspieler und aktuelle Tischkarten.
- Gewonnene Stiche **als geordnete Kartenpaare** pro Spieler, nicht nur als Gesamtpunkte.
- Bekannte Ansagen, erhaltene Augen, Spielpunkte und Rundenzustand.
- Status des Talons (offen, zugedreht, leer) sowie die für die Stichregeln relevanten Informationen.
- Gewähltes Regelprofil und dessen Optionen.
- Ereignisse der letzten Aktion für UI-Animationen.

### Automatischer Spielstand

`app::save` verwaltet einen automatischen Speicherplatz in `$XDG_DATA_HOME/schnapsen/save.json`. Ohne gesetztes `XDG_DATA_HOME` verwendet GLib das XDG-Standardverzeichnis unter `~/.local/share`. Im Flatpak liegt die Datei im app-eigenen Datenverzeichnis unter `~/.var/app/io.github.schnapsen.Schnapsen/data/schnapsen/`. Persönliche Einstellungen und Fensterzustand verbleiben in GSettings.

Die JSON-Datei verwendet ein Serde-Schema mit numerischem Versionsfeld `version` (aktuell `3`). `snapshot.session` enthält den vollständigen fachlichen Partiezustand mit festgeschriebener Konfiguration, geordneten Karten, Zugfolge, Wertung und Ergebnissen. `snapshot.selection` enthält bis zu zwei ausgewählte Handkarten. Karten, Varianten und Regelprofile verwenden sprachunabhängige IDs. Augen und vorgemerkte Ansagen ergeben sich aus gespeicherten Stichen und Ansageereignissen. Die Engine leitet offene Trumpfkarte, Geber und ausstehende Aktionen aus den gespeicherten Daten ab. Beim Fortsetzen der aktuellen Hand wird kein Zufall benötigt. Erst die nächste Verteilung verwendet wieder den Zufallsgenerator.

Die Anwendung speichert nach einer Zustandsänderung und beim regulären Schließen. Gleiche Snapshots lösen keinen Dateiaustausch aus. Der Austausch schreibt und synchronisiert eine private temporäre Datei im selben Verzeichnis und ersetzt danach den bisherigen Spielstand atomar. Ein abgebrochener Schreibvorgang vor der Ersetzung lässt die bereits geschriebene Version bestehen. Eine bestätigte neue Partie ersetzt den bisherigen Spielstand.

Beim Start prüft die Anwendung Version, Kartenbestand, Zugfolge, Ansagen und Wertungsdaten. Sie stellt einen gültigen Spielstand automatisch wieder her. Computerzug oder Stichauflösung werden aus der gespeicherten Situation neu eingeplant. Hand- und Partieergebnisse erscheinen erneut mit der schon gespeicherten Wertung. Die Oberfläche erstellt vorübergehende Dialoge, Timer und visuelle Effekte bei Bedarf neu. `Strg+Q` kann die Anwendung auch aus einem sichtbaren Ergebnis heraus mit der vorhandenen Wertung beenden.

Bei einer beschädigten, inkompatiblen oder nicht lesbaren Datei bleibt der Speicherplatz gesperrt. Erst die bestätigte Archivierung verschiebt die alte Datei in ein eigenes `unreadable-…`-Verzeichnis und erlaubt die neue Partie. Bei Schreibfehlern zeigt das Spiel einen Hinweis. Beim Schließen kann der Spieler es erneut versuchen oder das Verwerfen ungespeicherter Änderungen bestätigen.

Seit Version 2 speichert das Format die feste Schwierigkeit in der Partiekonfiguration, die zugehörige `strategy_id` und das öffentliche Spielgedächtnis. Der Lader prüft die Zuordnung von Schwierigkeit und Strategie-ID. Er übernimmt Version 1 mit Einfach und rekonstruiert bekannte Stiche und Ansagen aus der Hand. Früher beobachtete Tausche und der letzte offene Trumpfzug aus Version 1 sind nur insoweit verfügbar, wie sie aus den erhaltenen Daten hervorgehen. Bei der ersten regulären Speicherung legt die Anwendung eine unveränderte Sicherung in einem privaten `legacy-v…`-Verzeichnis ab und ersetzt danach den aktiven Speicherplatz atomar. Künftige Migrationen können am Versionskopf ansetzen. Die Strategien brauchen keinen gespeicherten Zufallszustand.

Version 3 speichert zusätzlich die interne Historie der aktuellen Partie. Version 1 und 2 werden mit einer als partiell markierten Historie geladen. Aufgezeichnet werden ab dann die tatsächlich ausgeführten Aktionen. Die nächste Partie beginnt mit vollständiger neuer Historie, die kumulierten Bummerl bleiben erhalten.

### Aktionen (fachliche Schnittstelle)

Beispiele: `PlayCard`, `AnnounceMarriage`, `ExchangeTrump`, `CloseTalon`, `StartNextDeal`, `NewMatch`. Nur im aktuellen Zustand gültige Aktionen werden akzeptiert. Ungültige Aktionen geben einen verständlichen Fehler zurück, ohne den Zustand zu ändern.

Eine 20/40-Ansage bedeutet **nicht**, dass zwei Karten ausgespielt werden: Die Auswahl von König und Ober unterstützt die Bedienung; tatsächlich wird nur eine Karte gespielt.

### Regelqualität

Die exakten Abläufe und Sonderfälle müssen anhand einer ausdrücklich gewählten, nachvollziehbaren Regelfassung spezifiziert werden, insbesondere:

- Karten- und Stichzwang vor/nach dem Ende beziehungsweise Zudrehen des Talons.
- Bedingungen für Ansagen und Trumpftausch.
- Zeitpunkt der Augenwertung und des automatischen Abschlusses bei 66 Augen.
- Spielpunkte und Sonderwertung bei zugedrehtem Talon.
- Letzter Stich, Talonende und sonstige Rundenausgänge.
- Unterschiede von *weich* und *scharf*, insbesondere das Einsehen alter Stiche.

**Keine Hausregel stillschweigend als allgemeingültig implementieren.** Offene Varianten werden dokumentiert, entschieden und mit gezielten Tests abgesichert.

### Verbindliche Variantenregeln

| Regel | Weich | Scharf |
|---|---|---|
| Einsicht auch nach Handende | Eigene gewonnene Stiche und genau der erste gewonnene Stich des Gegners | Abgelegte Stiche bleiben verdeckt |

Für Trumpftausch und Zudrehen gilt in beiden Profilen dieselbe projektbezogene Hausregel. Der erste Stich der Hand muss vollständig aufgelöst sein. Der Talon muss offen sein und mindestens vier Karten einschließlich der offenen Trumpfkarte enthalten. Die Aktion ist nur mit aktuellem Ausspielrecht und vor Beginn des nächsten Stichs möglich. Ein eigener Stichgewinn ist keine zusätzliche Voraussetzung. Für den Tausch muss der Spieler weiterhin den Trumpf-Unter halten. Diese Hausregel kann von veröffentlichten Regelwerken abweichen.

Für die weiche gegnerische Einsicht gelten bewusst weder ein Zeitfenster nach dem Stich noch eine Bindung an das Ausspielrecht. Die Oberfläche öffnet die Übersicht weiterhin erst in einem stabilen Spielzustand.

Die Hand endet nach der vollständigen Auflösung eines Stichs oder unmittelbar nach einer wirksamen Ansage automatisch, sobald eine Seite mindestens 66 wirksame Augen hat. Die Ansagekarte wird regelkonform ausgespielt. Erreicht die Ansage 66 Augen, folgt keine gegnerische Antwort. Vorgemerkte Ansagen im weichen Profil zählen erst ab dem ersten eigenen gewonnenen Stich. Bei erreichtem Ziel endet die Hand vor dem Nachziehen oder einem weiteren Zug. Das ist eine bewusst gewählte Hausregel für Mensch und Computer.

Die gewöhnliche Wertung bringt drei Spielpunkte ohne gegnerischen Stich, zwei bei mindestens einem Stich und weniger als 33 gegnerischen Augen, sonst einen. Erreicht bei offenem oder leerem Talon niemand 66 Augen, gewinnt der letzte Stich einen Spielpunkt.

Bei Talonsperre hat Wienerisch-Wertung Vorrang. Der Zudreher gewinnt bei 66 wirksamen Augen nach der Staffel von drei, zwei oder einem Spielpunkt anhand des eingefrorenen gegnerischen Stands. Erreicht der Gegner zuerst 66 oder bleiben dem Zudreher die nötigen Augen bis zum Ende verwehrt, gewinnt der Gegner drei Spielpunkte, falls er beim Zudrehen stichlos war, sonst zwei.

Die frühere manuelle Ausmeldung samt Strafwertung ist historisch im Regel-Audit dokumentiert. Sie gehört nicht zum automatischen Hausregelprofil.

## 5. Spielfeld und Interaktion

### Grundlayout: Draufsicht

- **Oben:** Gegnerische Hand, nur Kartenrückseiten, kleiner als die eigene Hand. Anzahl ist jederzeit sichtbar.
- **Mitte:** Bereich für die aktuell ausgespielten Karten.
- **Seitlich:** Talon mit erkennbarer Kartenanzahl und sichtbar liegender Trumpfkarte.
- **Unten:** Eigene Hand, größer und gut lesbar.
- **Rechts unten:** Eigener gewonnener Stichstapel; gegnerischer Stapel in dessen Nähe auf der Gegenseite.

Die Standardgröße des Hauptfensters beträgt 680 × 560 logische Pixel. Beim Schließen speichert GSettings die letzte normale Größe und den Maximierungszustand. GTK stellt die normale Größe auch nach dem Aufheben der Maximierung bereit. Bei ungültigen Maßen greifen die Standardwerte. Beim Start begrenzt die größte verfügbare Monitorfläche die gespeicherten Maße, soweit es die Mindestgröße des Tisches zulässt. Die Fensterposition bestimmt der Compositor. Die Bereiche passen sich an Fenstergröße und Seitenverhältnis an. Eine geringe Fensterhöhe begrenzt die Kartengröße. Meldungen, kontextabhängige Aktionen und modale Bestätigungen liegen über der Zeichenfläche und ändern ihre Geometrie nicht. Kartengrößen sind **absichtlich nicht überall gleich**. Priorität hat die Lesbarkeit der eigenen Karten und des aktuellen Stichs.

### Physische Anmutung

- Karten in gewonnenen Stapeln entsprechen den tatsächlich gewonnenen Karten, mit kleinen Versätzen, begrenzter Rotation und Schatten.
- Jeder Stich ergänzt zwei Karten; die Darstellung darf bei engen Platzverhältnissen stärker überlappen.
- Der Talon wird visuell kleiner, wenn Karten gezogen werden.
- Die Gegnerhand schrumpft sichtbar, wenn keine Karten mehr nachgezogen werden.
- Die Position der Objekte kommuniziert ihren Zweck; keine unnötigen HUD-Elemente.

### Bedienung

- Tippen/Klicken auf eine eigene Karte wählt sie aus; sie rückt sichtbar nach vorne.
- Erneutes Tippen/Klicken hebt die Auswahl auf.
- Es können höchstens zwei Karten zur Vorbereitung einer gültigen Ansage markiert werden; eine Ansage benötigt eine eindeutige Bestätigung samt Wahl der tatsächlich auszuspielenden Karte.
- **Leertaste** spielt die eindeutig ausgewählte Karte, sofern die Aktion gültig ist.
- Eine Touch-Wischbewegung in Richtung Tischmitte kann eine Karte ausspielen.
- Ein Doppelklick spielt die angeklickte Handkarte. Eine vorbereitete gültige Paaransage hat Vorrang. Rechtsklick und zentrale Aktionsschaltflächen bleiben verfügbar.
- Die Tischmitte reserviert Platz für Aktionsschaltflächen. Ihre Anzeige folgt den zulässigen Aktionen. Modale Bestätigungen sperren Eingaben am Tisch und geben den Fokus nach dem Schließen zurück. Escape bricht die Bestätigung ab.
- `Esc` schließt Overlays und hebt gegebenenfalls eine Auswahl auf.
- Interaktive Tischobjekte wie der Talon dürfen kontextbezogene Aktionen anbieten, z. B. Zudrehen oder Trumpftausch; kritische Aktionen benötigen eine erkennbare Bestätigung, wo sinnvoll.

### Stichübersicht

- Ein Klick/Tippen auf einen gewonnenen Stapel oder ein Tastenkürzel öffnet ein Overlay über dem Tisch.
- Die Übersicht kann Karten nach Stichfolge darstellen; eine zusätzliche Sortierung nach Farbe ist optional.
- Ob und welche eigenen oder gegnerischen Stiche eingesehen werden dürfen, hängt vom gewählten Regelprofil ab.
- Im weichen Profil liegen die beiden Karten des ersten vom Computer gewonnenen Stichs dauerhaft offen neben seinem Stapel. Der Stapel enthält nur die übrigen gewonnenen Karten. Im scharfen Profil bleiben alle Karten des Stapels verdeckt. Das ist eine projektbezogene Hausregel. Die bisherige Stichübersicht bleibt verfügbar.
- Über dem eigenen Stapel stehen die wirksamen Augen einschließlich gutgeschriebener Ansagen als Zahl. Über dem gegnerischen Stapel kann dieselbe kompakte Anzeige eingeblendet werden. Beide Zahlen bleiben oberhalb der maximalen Stapelhöhe. Die unabhängige Lernhilfe „Gegnerische Augen anzeigen“ ist standardmäßig aus. In der linken Statusspalte stehen Restpunkte und Bummerl der Gegenseite oben und die der lokalen Seite unten. Die Zahl abgeschlossener Hände steht dazwischen.
- Spielhilfen, die mehr Informationen als die klassische Regel erlauben, müssen ausdrücklich als solche gekennzeichnet und optional sein.
- Das Overlay ist auch mit Tastatur und Touch vollständig bedienbar.

### Animationen

Kurze Animationen genügen, z. B. 120–350 ms abhängig vom Vorgang. Auswahl bewegt die Karte wenige Pixel; Ausspielen kann durch kurzes Aus- und Einblenden statt einer Flugbahn erfolgen. Ein gewonnener Stich verschwindet aus der Mitte und wächst als realer Kartenstapel an. Animationsdauer bzw. reduzierte Bewegung ist einstellbar. Keine dauerhaft laufende Spielschleife erforderlich.

## 6. Kartengrafik und Lizenzierung

**Erste Phase:** Karten programmatisch als einfache, gut lesbare Rechtecke mit Werten und deutschen Farbsymbolen darstellen. Keine externen Kartenbilder notwendig.

**Später:** Historisches doppeldeutsches Kartenbild als austauschbares Theme ergänzen. Rechte und Quelle des konkreten Kartensatzes **vor** der Übernahme prüfen; keine modernen Herstellerbilder ohne ausreichende Lizenz verwenden. Assets, Schriftarten, Texturen und Sounds mit Quelle und Lizenz dokumentieren.

Einheitliche Karten-Geometrie, Hit-Tests und Interaktionen gelten unabhängig vom Theme. Die grafische Ebene darf anstelle von Bilddateien auch einfache Vektorformen zeichnen.

## 7. Computerstrategien

`core::observation::Observation` ist die für alle Stufen gleiche Informationsgrenze. Sie enthält die eigene Hand, eigene und aus öffentlichen Ereignissen abzählbare gegnerische Augen, Partiestand, öffentliche Karten, Zugfolge, Ansagen, Trumpftausche, Kartenanzahlen und die legalen Aktionen. Sie leitet mögliche gegnerische Karten aus beobachteten Antworten unter Farb-, Stich- und Trumpfzwang ab. Die Engine speichert Tausch und letzten öffentlichen Trumpfzug pro Hand. Stiche und Ansagen liefern die übrigen öffentlichen Fakten. Das Austeilen einer neuen Hand startet ein neues Gedächtnis. Die verdeckte Verteilung und Ziehreihenfolge bleiben in der Engine.

`ai` registriert je Schwierigkeit eine Strategie mit stabiler ID. Ein neues Modul mit `Strategy::decide` kann über die Registrierung ergänzt werden. Die Oberfläche steuert Zeitpunkt und Darstellung, `ai::apply` führt die regulären Engine-Aktionen aus.

| Stufe | Strategie-ID | Verfahren |
|---|---|---|
| Einfach | `first_legal` | Tauscht und sagt wie bisher an. Ergreift einen unmittelbar gewinnenden Antwortstich für 66 Augen. Sonst bleibt die bisherige Reihenfolge legaler Karten maßgeblich. |
| Mittel | `tactical` | Bewertet Stichwert, Zugrecht, 66-Augen-Chancen, Kartenverbrauch, Trumpf und Ansagepaare. Spart hohe Trümpfe, wenn eine günstigere Antwort genügt. |
| Schwer | `foresight` | Bewertet Stichfolgen auf bis zu zwölf mit der Beobachtung vereinbaren hypothetischen Gegnerhänden. Nutzt ein begrenztes Minimax mit Alpha-Beta-Pruning und berücksichtigt Hand- und Partiewertung. |

Schwer prüft bei legalem Zudrehen die gesperrte Fortsetzung bis zum Handende oder zum Suchbudget. Das Zudrehen kommt nur infrage, wenn die untersuchten Szenarien einen Gewinn prognostizieren und der erwartete Wert den besten offenen Zug mit Sicherheitsabstand übertrifft. Die Stichentscheidung und der Bedienzwang in der Suche stammen aus den Engine-Regeln. Die Stufen Einfach und Mittel lassen den Talon offen.

Die Suche hat feste Grenzen für Stichzahl und Knoten und ist reproduzierbar. Bei offenem Talon mit mehr als zwei Karten bewertet Schwer den nächsten Stich. Beim letzten Ziehen und leerem Talon werden bis zu drei Stiche betrachtet. Für Abschlussprognosen bei zugedrehtem Talon stehen je Aktion und Szenario höchstens 256 Suchknoten zur Verfügung. Spätere Ansagen werden heuristisch bewertet. Die Ergebnisse bleiben Näherungen für die verfügbaren Beobachtungen.

Im Menü **Partie** wird die Spielstärke für die nächste Partie gewählt und sprachunabhängig in GSettings gespeichert. **Nächste Hand** behält die aktive Strategie. **Neue Partie** übernimmt die Vorgabe. Die aktive Spielstärke steht im Fenstertitel.

Der GTK-freie Release-Vergleich läuft mit `cargo run --release --bin compare_strategies -- 64 1`. Er verwendet beide Profile, gleiche Austeilfolgen mit vertauschten Strategieplätzen, vollständige Partien und Kontrollpaarungen. Das Werkzeug meldet Siege, Punkte, Augen, Schneider, Zudrehen, ungültige Aktionen und Entscheidungszeiten als JSONL.

## 8. Einstellungen und Barrierefreiheit

Mindestens vorsehen:

- Regelprofil: weich / scharf.
- Spielhilfen: ein-/ausschaltbar, möglichst einzeln.
- Animationen: normal / reduziert.
- Eingaben: vollständig per Maus und Tastatur, Touch-Unterstützung zusätzlich.
- Gut erkennbare Kartenwerte; Fokus- und Auswahlzustände sind nicht nur durch Farbe unterscheidbar.
- Keine unnötigen Berechtigungen oder Netzwerkanforderungen im Flatpak.

## 9. Test- und Qualitätsanforderungen

- Unit-Tests für Kartendeck, Augen, Kartenverteilung und eindeutige Kartenidentitäten.
- Zustandsbasierte Tests für legale/illegale Aktionen und sämtliche Regel-Sonderfälle.
- Reproduzierbare Szenarien für Ansagen, Zudrehen, Trumpftausch, Talonende, 66 Augen und Spielpunktvergabe.
- Tests, dass KI nur legale Züge erzeugt und keine verdeckten Informationen auswertet.
- UI-Smoketests für Auswahl, Aktion, Overlay und verschiedene Fenstergrößen.
- Prüfung von Tastatur, Maus und Touch; reduzierte Bewegung und lesbare kleine Karten.
- Flatpak-Build und Start unter GNOME prüfen.

### Historie, Testfälle und Replay

`core::history` erfasst Handbeginn mit Vorhand, offenem Trumpf und interner Austeilung, das Ausspielen, Ansagen, Trumpftausch, Zudrehen, Stichauflösung, Augenänderung und Aktivierung von Ansageaugen. Es speichert Ziehreihenfolge und Talonende, Hand- und Partieabschluss, Spielpunkte und Bummerl. Geber ist die Gegenseite der Vorhand. Stichkarten folgen aus den ausgespielten Karten. Private Einträge halten verdeckte Austeilungen und Züge sowie die tatsächlichen Shuffle-Entscheidungen. Die getrennte öffentliche Projektion enthält den offen gezogenen letzten Trumpf. Strategien erhalten weiterhin ausschließlich `core::observation::Observation`. Die Historie bleibt auf die aktuelle Partie begrenzt, beim Laden gilt eine Schranke von 1.024 Ereignissen.

`testing::validation` prüft bei Bedarf nach jeder Aktion die Engine-Invarianten und zählt Augen, Wertungsgrenzen und die Partieabrechnung zusätzlich unabhängig nach. `testing::case` spielt die aufgezeichneten Aktionen über die Engine erneut aus. Es prüft Zulässigkeit, erwartete Ereignisse und Zustände. Abweichungen werden mit der frühesten erkannten Aktion gemeldet. Eine vollständige gespeicherte Partiehistorie kann mit den Echt-Austeilentscheidungen ohne Seed nachgespielt werden.

Das eigenständige Testfallformat ist JSON Version 1 mit `created_with`, einem seedbasierten Start oder einem exakten Engine-Snapshot, geordneten Aktionen und optionalen Kontrollständen, erwarteter Historie und Endzustand. Wertung und Ergebnis sind Teil der erwarteten Zustände. Ein importierter Spielstand bleibt unverändert. Partielle Historien eignen sich als Ausgangssnapshot für nachfolgende Aktionen. Ein Seed garantiert einen identischen Verlauf bei identischer Engine, Shuffle-Implementierung und Strategie. Ein Testfall mit expliziten Aktionen oder eine vollständige Historie macht Änderungen über Programmversionen hinweg sichtbar. Kompatible Wiedergabe über beliebige künftige Regel- oder Formatänderungen ist nicht zugesichert.

### Entwicklerwerkzeug

Die folgenden Befehle verwenden die GTK-freie Konfiguration. Das frühere Vergleichsprogramm bleibt als Einstieg für die Vergleichsmatrix von Auftrag 25 verfügbar und ruft dieselbe Simulation auf.

```sh
cargo build --release --no-default-features --bin schnapsen_lab
lab=target/release/schnapsen_lab
"$lab" --help
"$lab" simulate --seed 11 --count 20 --variant austrian_schnapsen --profile sharp \
  --schneider single --local medium --opponent hard --forehand local --mirror \
  --validate --output report.json --failure failure
"$lab" matrix --seed 1 --count 224 --output matrix.json
"$lab" record --seed 11 --profile soft --local medium --opponent hard \
  --validate --output recorded.json
"$lab" replay recorded.json
"$lab" validate-case recorded.json
"$lab" validate-save save.json
"$lab" from-save save.json --seed 12 --output regression.json
```

`simulate` nutzt pro Seed eine Partie und mit `--mirror` zusätzlich dieselbe Austeilfolge mit getauschten Strategien. `matrix` aktiviert Validierung und Spiegelung und umfasst alle Profile der gewählten Variante, beide Schneiderregeln und alle Strategiepaare. `record` zeichnet eine Partie auf, wobei `--mirror` hier nur die getauschte Anordnung auswählt. `validate-save` prüft Spielstand und, bei vollständiger Historie, deren Wiederholung. `from-save` erzeugt einen exakten Ausgangszustand mit Seed für spätere Austeilungen. `--failure` legt den Ausgabepfad für Fehlerdiagnose und Replay fest. Bestehende Ausgabedateien werden geschützt.

`testing::statistics` gruppiert Berichte nach Variante, Profil, Schneiderregel und Strategiepaar. Gewinnquoten beziehen sich auf die abgeschlossenen Partien beziehungsweise Hände der Gruppe. Durchschnittliche Augen verwenden sämtliche Hände der Gruppe. Strategielaufzeiten messen Beobachtung und Entscheidung. Die Gesamtdauer umfasst auch Engine, Validierung und Statistik. JSON enthält dieselben Kennzahlen wie der menschenlesbare Bericht.

Ein späteres Regelprofil wird als technische ID in `RuleProfile` und in den unterstützten Profilen der Variante geführt. Die Matrix nimmt es über `GameVariant::profiles()` auf. Ergänzende Referenzfälle können beispielsweise mit `record --profile <id>` erstellt und mit festen erwarteten Kontrollständen geprüft werden. Varianten mit anderen Wertungsgrenzen ergänzen die zugehörigen unabhängigen Prüfwerte in `testing::validation`.

## 10. Abnahmekriterien für den ersten spielbaren Prototyp

Ein Prototyp ist erst dann spielbar, wenn eine vollständige Partie gegen den Computer ohne manuelle Zustandskorrekturen durchlaufen werden kann, alle Züge gültig sind, Punktezählung und Rundenausgang stimmen und der Tisch bei normaler Desktop-Fenstergröße bedienbar ist. Einfache Kartenbilder sind ausdrücklich ausreichend. Noch nicht verifizierte Regelvarianten dürfen nicht als vollständig implementiert gekennzeichnet werden.

## 11. Offene Entscheidungen

1. Exakte Mausgeste für das Ausspielen und Bestätigung einer 20/40-Ansage.
2. Minimale unterstützte Fenstergröße und Verhalten bei sehr schmalen Fenstern.
3. Konkrete App-ID, Anzeigename und Lizenz des Quellcodes.

Diese Punkte blockieren nicht den Beginn der Regel-Engine, müssen aber vor dem entsprechenden Implementierungsschritt entschieden werden.
