# GNOME Schnapsen – Entwicklungsplan

**Status:** Mehrhand-Partien gegen den Computer mit Restpunkten, Bummerl-Wertung und profilabhängiger Stichübersicht. Flatpak-Entwicklung mit GNOME SDK 51.  
**Verbindliche Produktanforderungen:** [`SPEC.md`](SPEC.md)  
**Arbeitsanweisungen für Codex:** [`AGENTS.md`](AGENTS.md).

## Arbeitsweise

- Vor jeder neuen Aufgabe `SPEC.md` und `PLAN.md` lesen.
- Nur die für die Aufgabe notwendigen Dateien ändern; Architektur und Abhängigkeiten klein halten.
- Jede abgeschlossene Aufgabe mit tatsächlichen Änderungen und Prüfergebnissen dokumentieren.
- Keine Aufgabe als erledigt markieren, wenn relevante Tests nicht durchgeführt wurden; nicht ausgeführte Tests explizit nennen.
- Bei offenen oder widersprüchlichen Schnapsregeln keine Annahmen verschweigen: als offene Entscheidung markieren.
- Nach jeder Arbeitssitzung die Abschnitte **Aktueller Stand**, **In Arbeit**, **Nächster Schritt** und **Änderungsprotokoll** aktualisieren.

## Aktueller Stand

- [RULE_AUDIT.md](RULE_AUDIT.md) vergleicht die implementierten Profile mit drei Regelquellen und dokumentiert Abweichungen und Stabilitätsprüfungen.

- `src/i18n.rs` bindet GNU gettext für die Oberfläche an. Das Flatpak enthält beide Kataloge und die aus lokalisierten Vorlagen erzeugten Desktop- und AppStream-Dateien.
- `src/app/mod.rs` verwaltet den Lebenszyklus der Anwendung und öffnet oder aktiviert das Fenster.
- `src/ui/mod.rs` bindet den Kartentisch über eine `GtkDrawingArea` in das Fenster ein. Die Zeichenfläche hat eine Mindestgröße von 560 × 400 logischen Pixeln. Die Titelleiste liegt außerhalb. Der Partiestand steht in einer festen Spalte links auf der Zeichenfläche und nutzt den vorhandenen Abstand zum Talon. Die Zielgröße des Fensters ist 680 × 560. Kontextaktionen liegen in einer ungemessenen GTK-Overlay-Ebene. Hand- und Partieergebnisse erscheinen in einem modalen libadwaita-Dialog über dem Tisch.
- `src/core/game.rs` prüft Ausspielaktionen und löst vollständige Stiche auf. Eine Ansage prüft Ausspielrecht, Paar und Profil und spielt atomar eine Karte aus. Stichaugen und vorgemerkte oder wirksame Ansageaugen werden aus Stichpaaren und Ansageereignissen abgeleitet. Ab 66 wirksamen Augen beendet die Engine die Hand nach Stichauflösung oder Ansage automatisch. Der Abschlussgrund und die Spielpunkte stehen in `Finished`.
- Der Trumpftausch ersetzt den Trumpf-Unter in der Hand durch die offene Trumpfkarte. Der Unter bleibt als letzte Karte des Talons liegen. Tausch und Zudrehen teilen dieselbe Hausregelprüfung. Sie verlangen einen abgeschlossenen ersten Stich, aktuelles Ausspielrecht vor Stichbeginn und mindestens vier Karten im offenen Talon einschließlich der Trumpfkarte. Die Engine prüft beim Tausch zusätzlich den Besitz des Trumpf-Unters.
- Zudrehen sperrt den Stapel und speichert den wirksamen Augenstand und die Stichzahl des Gegners für die Wertung. Ab dann gelten dieselben Bedienpflichten wie bei einem regulär leeren Talon. Die gesperrten Karten bleiben erhalten.
- `src/ai.rs` wählt deterministisch die erste zulässige Ansage oder die erste legale Karte. Die Auswahl erhält keine verdeckten Karten des anderen Spielers.
- `src/core/rules.rs` definiert die unterstützte Spielvariante, die beiden Regelprofile und die feste Partiekonfiguration.
- `src/core/match_play.rs` verwaltet die aktuelle Hand, Restpunkte, Handanzahl, Vorhand und Bummerl. Jede abgeschlossene Hand wird einmal angerechnet. Null Restpunkte beenden die Partie.
- `src/ui/table.rs` hält die Partie sowie Auswahl und Fokus. Die Layout-Sicht erhält nur eigene und öffentliche Kartenwerte sowie Anzahlen für verdeckte Karten. Ein Handwechsel setzt die handbezogenen Bedienelemente zurück.
- `src/ui/layout.rs` platziert Hände, beide Stichstapel, Talon mit Trumpfkarte und bis zu zwei ausgespielte Karten. Die GTK-unabhängige Berechnung hebt ausgewählte Karten um 12 Prozent ihrer Höhe an. Trefferprüfung und Zeichnung verwenden dieselben Positionen und die Kartenkontur. Bei wenig Breite überlappt die Hand stärker.
- `src/ui/input.rs` bindet Linksklick und kurzes Tippen an die Auswahl. Pfeiltasten bewegen den Fokus, Enter wechselt die Auswahl, Escape hebt sie auf. Leertaste und Rechtsklick spielen eine einzeln ausgewählte Karte aus. Ein Doppelklick spielt die angeklickte Karte oder verwendet die gültige vorbereitete Paaransage. Eine Wischgeste nach oben kann die berührte Karte direkt ausspielen.
- Mit ausgewähltem Trumpf-Unter tauschen **T**, ein Klick oder ein Tippen auf die offene Trumpfkarte. Die neue Handkarte erhält den Fokus. Der Computer prüft den Tausch über dieselbe Engine vor seinem Ausspiel.
- Klick oder Tippen auf den verdeckten Stapel und **Z** starten die Bestätigung zum Zudrehen. Die bisher offene Trumpfkarte erscheint danach verdeckt quer auf dem Stapel.
- `src/ui/turn.rs` steuert Computerzug und Stichauflösung mit GLib-Timern. Der Computer beginnt auch als Vorhand. Beim Hand- oder Partiewechsel werden alte Timer entfernt.
- `src/ui/review.rs` zeigt die erlaubten Stichpaare in einem abgedunkelten GTK-Overlay mit anpassbarer Spaltenzahl und vertikalem Scrollen. Die Kartenzeichnung stammt vom Kartentisch. Klick oder Tippen außerhalb, Escape und „Schließen“ beenden die Übersicht mit Fokusrückgabe.
- `src/ui/paint.rs` zeichnet Kartenfronten, einheitliche Rückseiten, Rahmen und Schatten mit Cairo. Ausgewählte Karten erhalten einen goldenen Rand, der sichtbare Tastaturfokus eine gestrichelte Innenlinie.
- Die abschaltbare **Spielhilfe** markiert die legalen Karten. Über den Stichstapeln stehen die wirksamen eigenen Augen als Zahl und optional die gegnerischen Augen über eine separate Lernhilfe. Links stehen die Restpunkte und Bummerl beider Seiten sowie die Zahl abgeschlossener Hände. Kontextaktionen liegen in der Tischmitte. `src/ui/result.rs` zeigt den Handabschluss mit Augen und Spielpunkten. Beim Partieende kommen Bummerl und Reststand hinzu. Ausschließlich **Nächste Hand** oder **Neue Partie** schließen den Ergebnisdialog und setzen das Spiel fort.
- Das Menü **Partie** zeigt österreichisches Zweierschnapsen und konfiguriert Regelprofil und Schneiderwertung für die nächste Partie. Die erste Partie startet weich mit doppelter Schneiderwertung. Bummerl lassen sich über eine getrennte Aktion mit Bestätigung zurücksetzen.
- `src/lib.rs` exportiert `core::cards` mit Kartenidentität, Augenwert und vollständigem 20er-Kartensatz. Das Modul verwendet nur die Rust-Standardbibliothek.
- `Cargo.toml` und `Cargo.lock` definieren die Anwendung und ihre Abhängigkeiten. GTK4 wird über libadwaita eingebunden. Die App-ID `io.github.schnapsen.Schnapsen` ist vorläufig.
- `io.github.schnapsen.Schnapsen.json` ist die Build-Konfiguration für GNOME Builder und Flatpak. `meson.build` startet Cargo und installiert Programm, Desktop-Datei, AppStream-Metadaten und Icon.
- `cargo-sources.json` enthält die Crate-Archive und Prüfsummen aus `Cargo.lock`. Flatpak lädt die Archive vor dem Build. Das Modul `cargo-dependencies` stellt sie auch der separaten Meson-Pipeline in Builder bereit. Cargo baut mit `--locked --offline`. Flatpak entfernt die vorbereiteten Quellen beim Paketabschluss.

## Roadmap ab Auftrag 23

Die funktionale Spielentwicklung hat Vorrang. Die vorhandene Darstellung reicht für die laufende Entwicklung und praktische Spieltests aus.

### Phase 1 – Funktionale Spielentwicklung

| Auftrag | Thema |
|---|---|
| 23 | Internationalisierung |
| 24 | Spielstände speichern und fortsetzen |
| 25 | Computerstrategien und Schwierigkeitsstufen |
| 26 | Erweiterte Spieltests, reproduzierbare Spielsituationen und Simulationen |
| 27 | Weitere regionale Regelprofile |
| 28 | Weitere Spielvarianten |
| 29 | Funktionale Beta-Abnahme |

### Phase 2 – Grafische und akustische Verfeinerung

| Auftrag | Thema |
|---|---|
| 30 | Historische und hochwertige Kartengrafiken |
| 31 | Audio und Kartengeräusche |
| 32 | Animationen und Überblendungen |

### Phase 3 – Veröffentlichung

| Auftrag | Thema |
|---|---|
| 33 | Anwendungssymbol und AppStream-Metadaten |
| 34 | Abschließende Qualitäts- und Barrierefreiheitsprüfung |
| 35 | Veröffentlichung auf Flathub |

## Meilensteine

### M0 – Projektgerüst
- [x] Rust-Projekt initialisieren.
- [x] GTK4/libadwaita-Anwendung mit startbarem Fenster anlegen.
- [x] Flatpak-Manifest und lokales Build-Verfahren anlegen.
- [x] Module `core`, `ui` und `app` trennen.
- [x] Tests/Formatierung/Lint-Befehle dokumentieren.

**Ergebnis:** Native GNOME-Anwendung startet im Flatpak-Build.

### M1 – Regeldefinition und Kartenmodell
- [x] Verbindliche Regelquellen und Unterschiede zwischen weich/scharf dokumentieren.
- [x] Kartentypen, 20er-Deck und Augenwerte implementieren.
- [x] Spielzustand für die Anfangsverteilung definieren.
- [x] Zulässiges Ausspielen bei offenem und leerem Talon definieren.
- [x] Mischen und Verteilen mit reproduzierbar steuerbarer Zufallsquelle implementieren.
- [x] Vollständigkeit, Eindeutigkeit und Augenwerte des Kartenmodells testen.
- [x] Austeilen und dessen Invarianten testen.

**Ergebnis:** Spielzustände sind ohne GUI erzeugbar und zuverlässig testbar.

### M2 – Vollständige Regel-Engine
- [x] Stichfolge und Stichgewinner bei offenem Talon implementieren.
- [x] Nachziehen und letztes Ziehen der offenen Trumpfkarte implementieren.
- [x] 20/40-Ansagen für weich und scharf implementieren.
- [x] Trumpftausch für weich und scharf implementieren.
- [x] Zudrehen einschließlich Wienerisch-Wertung implementieren.
- [x] Übergang zu leerem Talon abbilden.
- [x] Farb- und Stichzwang nach Talonende implementieren.
- [x] Hand automatisch bei 66 wirksamen Augen oder nach dem letzten Stich abschließen.
- [x] Augen gewonnener Stiche erfassen.
- [x] Spielpunkte einer regulären Hand berechnen.
- [x] Mehrhand-Partien mit Restpunkten und kumulierten Bummerln werten.
- [x] Historische Ausmeldevariante samt Strafwertung dokumentieren.
- [ ] Sonderfälle und Regelprofile umfassend testen.

**Ergebnis:** Vollständige Partie über die Core-API simulierbar.

### M3 – Einfacher Computergegner
- [x] Legale Karten in beiden Spielabschnitten aus dem Regelzustand ableiten und deterministisch die erste wählen.
- [x] Die Kartenfolge in automatisierten Testpartien bis zum zehnten Stich prüfen.
- [x] Dieselbe automatische 66-Augen-Wertung für Computer und Mensch verwenden.
- [ ] Einfaches heuristisches Verhalten implementieren.
- [ ] Informationsgrenzen und reproduzierbare KI-Tests absichern.
- [x] Automatische Testpartien gegen den Computer durchführen.

**Ergebnis:** Computer beendet regelkonforme Partien.

### M4 – Spielbarer Kartentisch
- [x] Responsive Draufsicht mit festen Funktionsbereichen erstellen.
- [x] Einfache Kartenfronten und -rückseiten programmatisch zeichnen.
- [x] Hände, Talon, Trumpf, Tischmitte und Stichstapel aus dem Spielzustand anzeigen.
- [x] Kartenauswahl per Maus, Tastatur und Touch umsetzen.
- [x] Ausspielbedienung an geprüfte Spielaktionen anbinden und Regelverstöße verständlich anzeigen.
- [x] Abschaltbare Markierung zulässiger Karten anbieten.

**Ergebnis:** Vollständig bedienbare Partie mit Platzhaltergrafik.

### M5 – Physische Anmutung und Einsicht
- [ ] Dezente Auswahl-, Stich- und Stapelanimationen einbauen.
- [x] Kartenversätze und begrenzte Stapelgröße in der statischen Darstellung umsetzen.
- [x] Talonabnahme und wachsende Stichstapel aus dem Spielzustand darstellen.
- [x] Stichübersicht als Overlay implementieren.
- [x] Zugriffsregeln für Stichübersicht an Regelprofil koppeln.
- [x] Tastaturkürzel zur Kartenauswahl umsetzen.
- [x] Touchgeste für die geprüfte Ausspielaktion bei offenem Talon anbinden.
- [ ] Einstellung für reduzierte Bewegung ergänzen.

**Ergebnis:** Kartentisch vermittelt den Spielverlauf ohne aufwendige Engine.

### M6 – GNOME-Politur und Veröffentlichungsvorbereitung
- [x] Regelprofil und Schneiderwertung für die nächste Partie sowie Spielhilfe auswählen.
- [ ] Fenstergrößen, Fokusnavigation und Eingabeverhalten testen.
- [ ] App-Metadaten, Icon und Asset-Lizenzen dokumentieren.
- [ ] Flatpak-Manifest und Paket prüfen.
- [ ] Bekannte Probleme und Veröffentlichungsumfang dokumentieren.

**Ergebnis:** Technisch überprüfbarer Release-Kandidat.

## In Arbeit

Die Tischdarstellung und die Hausregeln für Stichdarstellung und automatischen Abschluss sind in [SPEC.md](SPEC.md) dokumentiert. Physischer Touch, HiDPI, Klick und Tippen auf beide Stichstapel sowie Scrollen bleiben zur manuellen Abnahme offen.

## Nächster Schritt

Auftrag 24 behandelt das Speichern und Fortsetzen von Spielständen. Die offenen manuellen Eingabeprüfungen bleiben Teil der Abnahme.

## Build in GNOME Builder

1. In GNOME Builder den Projektordner öffnen. In den Build-Einstellungen `io.github.schnapsen.Schnapsen.json` als aktive Konfiguration auswählen.
2. Falls Builder nach Runtimes fragt, `org.gnome.Platform//51`, `org.gnome.Sdk//51` und `org.freedesktop.Sdk.Extension.rust-stable//26.08` aus Flathub bereitstellen. Die erste Vorbereitung benötigt Zugriff auf Flathub und `static.crates.io`.
3. **Bauen** wählen und in der Build-Ausgabe das Modul `cargo-dependencies` beziehungsweise dessen Cache-Treffer sowie den Cargo-Release-Build prüfen.
4. **Ausführen** wählen. Erwartet wird ein Fenster „Schnapsen“ mit gemischter Anfangsverteilung und Reststand 7:7.

Für einen Build vom Terminal lässt sich das in GNOME Builder mitgelieferte `flatpak-builder` verwenden. Die Variable `XDG_DATA_HOME` zeigt dabei auf den regulären Benutzerpfad, damit das als Benutzer-Flatpak installierte SDK gefunden wird. Vom Projektordner aus ausführen:

```sh
flatpak run --command=sh org.gnome.Builder -c '
  export XDG_DATA_HOME="$HOME/.local/share"
  flatpak-builder .flatpak-build io.github.schnapsen.Schnapsen.json &&
    flatpak-builder --run .flatpak-build io.github.schnapsen.Schnapsen.json /app/bin/schnapsen
'
```

Bei einem weiteren Build mit vorhandenem `.flatpak-build` kann ein frisches Ausgabeverzeichnis verwendet oder der vorherige Build mit `--force-clean` ersetzt werden.

Nach Änderungen an `Cargo.lock` die Quellenliste mit [flatpak-cargo-generator](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo) aktualisieren und beide Dateien gemeinsam übernehmen.

```sh
python3 flatpak-cargo-generator.py -o cargo-sources.json Cargo.lock
```

## Bekannte Probleme / Risiken

- Die Profile unterscheiden sich bei Ansagen und beim Stichzugriff. Für Talonaktionen gilt dieselbe Hausregel. Die Sticheinsicht ist implementiert. Ihr Zeitfenster und ihre Rollenbindung weichen je nach Regelquelle ab.
- Der Computer wählt die erste legale Karte. Die Engine beendet seine Hand bei 66 wirksamen Augen automatisch wie beim Menschen.
- Der Computer nutzt die gemeinsame Regelprüfung und Wertung. Automatisches Zudrehen bleibt ohne konservative Entscheidungsregel deaktiviert.
- Bummerl werden bis zum Schließen des Fensters gehalten.
- Der SDK-Build ist der vorgesehene Entwicklungsweg auf GNOME OS. Außerhalb des SDKs fehlen der Codex-Umgebung die GTK4- und libadwaita-Entwicklungsbibliotheken.
- Für die Veröffentlichung fehlen Projektlizenz, Homepage, Entwicklerangaben und Release-Metadaten. Die App-ID ist vorläufig.

## Entscheidungen

| Datum | Entscheidung | Begründung |
|---|---|---|
| 2026-11-22 | Rust + GTK4 + libadwaita + Flatpak | Native, schlanke GNOME-Anwendung ohne Game-Engine |
| 2026-11-22 | Erst einfache Kartengrafik | Fokus auf Mechanik und Bedienbarkeit |
| 2026-11-22 | Nur gegen Computer in v0.1 | Kleiner, testbarer Produktumfang |
| 2026-11-22 | Regel-Engine unabhängig von UI | Testbarkeit und langfristige Wartbarkeit |
| 2026-11-23 | GNOME Platform und SDK 51 mit Rust-Erweiterung 26.08 | Aktueller stabiler GNOME-Zweig und passende Freedesktop-SDK-Erweiterung |

## Änderungsprotokoll

### 2026-11-25 – Auftrag 23

- Funktionale Entwicklung bis zur Beta-Abnahme priorisiert. Grafik, Audio und Animationen folgen in Phase 2, Veröffentlichung in Phase 3. Die bisherigen Meilensteine, offenen Prüfungen und Protokolle bleiben erhalten.
- GNU gettext lokalisiert GTK-Oberfläche, Spielmeldungen, Cairo-Beschriftungen, Kartenbezeichnungen, Desktop und AppStream. Die System-Locale wählt Deutsch oder Englisch. Englisch ist zugleich die Quellsprache und der Rückfall. Terminologie und Katalogpflege sind in [SPEC.md](SPEC.md) beschrieben.
- Formatierung, Cargo-Check, alle 73 Tests, Clippy mit `-D warnings`, Prüfung der jeweils 120 Katalogeinträge und Flatpak-Release-Build einschließlich Tests waren im GNOME SDK 51 erfolgreich. Im gebauten Flatpak wurden beide MO-Dateien und die zweisprachigen Metadaten geprüft. Die Testfälle decken Locale, Sprachpriorität, Rückfall, dynamische Meldungen, Pluralformen, Design-IDs und GSettings über einen Sprachwechsel hinweg ab.
- Ein isolierter GTK-Prüflauf unter Wayland bestätigte bei 560 × 450 logischen Pixeln in `de_AT.UTF-8` und `en_GB.UTF-8` Partiemenü, Rücksetzbestätigung, Einstellungen und vollständig sichtbare Handergebnisse. Manuelle Prüfung mit physischem Touch, HiDPI und assistiver Technik bleibt offen.

### 2026-11-25 – Auftrag 22a

- `ui::window_state` stellt die normale Hauptfenstergröße und die Maximierung aus GSettings wieder her. Beim Schließen wird nur ein geänderter Zustand geschrieben. GTK liefert auch maximiert die letzte normale Größe. Für Vollbild gibt es derzeit keine Anwendungsfunktion.
- Zwei neue Regressionstests prüfen ungültige Maße und kleinere Monitorflächen. Der bestehende GSettings-Prozesstest prüft beide Speicherbereiche gemeinsam. Unter Wayland wurde das echte Hauptfenster auf 820 × 620 logische Pixel vergrößert, normal und maximiert neu geöffnet und nach dem Aufheben der Maximierung wieder mit 820 × 620 angezeigt. Ungültige Werte führten beim Start zu 680 × 560.
- Formatierung, Cargo-Check, alle 69 Tests, Clippy mit `-D warnings` und der Flatpak-Release-Build inklusive Tests waren im GNOME SDK 51 erfolgreich. Der Monitorwechsel ist automatisiert über Modellwerte geprüft.

### 2026-11-25 – Auftrag 22

- `ui::design` registriert Doppeldeutsch und Französisch als Kartenfronten sowie die Rücken Original, Klassisch, Wienerisch, Alpin, Geometrisch, Ornament und Minimal. Kartentisch, Trumpfanzeige, Stichübersicht und Rückenvorschau verwenden denselben Cairo-Renderer. Kartenspezifische Hinweise verwenden die Namen des gewählten Themas.
- `ui::preferences` bündelt Darstellung und beide persönlichen Spielhilfen im nativen Einstellungsdialog. `app::settings` speichert stabile IDs über GSettings. Das Flatpak verwendet dessen Keyfile-Backend im app-eigenen Konfigurationsverzeichnis. Der Audio-Schlüssel ist vorbereitet.
- Formatierung, Offline-Cargo-Check, alle 67 Tests, Clippy für alle Targets mit `-D warnings` und der Flatpak-Release-Build einschließlich Tests waren im GNOME SDK 51 erfolgreich. Die Layout- und Rendererprüfungen decken 560 × 400 bis 3840 × 2160 logische Pixel ab.
- Ein GTK-Test unter Wayland prüfte den Dialog mit zwei Fronten und sieben Rückenvorschauen, den unmittelbaren Designwechsel während eines Stichs samt erhaltener Auswahl und die Wiederherstellung nach einem zweiten Start. Zwei getrennte Flatpak-Prozesse schrieben und lasen eine Design-ID im isolierten Keyfile-Profil. Der Fensterstart über `flatpak-builder --run` erreichte den Wayland-Compositor; in dieser Umgebung wurde dabei kein Fensterpuffer bestätigt.
- Die visuelle Abnahme bei physischen Touch- und HiDPI-Konfigurationen bleibt offen. Erweiterungspunkte für weitere Kartenfronten und -rücken stehen in [SPEC.md](SPEC.md).

### 2026-11-24 – Auftrag 21

- `core::rules` trennt die Identität und Grundregeln des österreichischen Zweierschnapsens von den zentralen Profilen Weich und Scharf. Die bisherigen Hausregeln einschließlich der Wienerisch-Wertung bleiben maßgeblich.
- `MatchConfig` friert Spielvariante, Regelprofil und Schneiderwertung für jede Partie ein. Im Menü **Partie** wird das einzige unterstützte Spiel angezeigt. Regelprofil und Schneiderwertung können für die nächste Partie vorgemerkt werden. Die Lernhilfen bleiben separate lokale Einstellungen.
- Drei Regressionstests prüfen das Standardspiel, die bestehenden Hausregeln beider Profile und die Konfiguration über Hand- und Partiewechsel. Ein isolierter GTK-Test unter Wayland bestätigte, dass „Nächste Hand“ Weich und die doppelte Schneiderwertung beibehält, während „Neue Partie“ die vorgemerkte Auswahl Scharf und einfache Schneiderwertung übernimmt. Formatierung, Cargo-Check, alle 62 Tests einschließlich der Simulation über 768 Hände, Clippy mit `-D warnings`, Flatpak-Release-Build einschließlich Tests und Wayland-Fensterstart waren im GNOME SDK 51 erfolgreich.
- Die künftigen Schnittstellen für Kartenthemen, unabhängige Kartenrücken, Audio, Animationen, Computerstrategie und persönliche Einstellungen sind in [SPEC.md](SPEC.md) beschrieben. Die Engine und Tischansicht sind weiterhin auf zwei Spieler ausgelegt. Die manuelle Ausmeldung bleibt eine mögliche spätere Profilerweiterung.

### 2026-11-24 – Auftrag 20d

- Rechts mittig steht eine feste Trumpfanzeige mit dem Farbsymbol und den Farben der Kartenzeichnung. Sie liest die Trumpffarbe direkt aus der Hand und ist unabhängig vom Talonstatus. Die Trumpffarbe ist auch in der zugänglichen Tischbeschreibung enthalten.
- Layouttests prüfen den Abstand zu Karten und zur Aktionszone von 560 × 400 bis 3840 × 2160 logischen Pixeln einschließlich breiter und schmaler Formate. Ein Cairo-Regressionstest zeichnet alle vier Farbsymbole bei 560 × 400 und 1920 × 1080.
- Formatierung, Cargo-Check, alle 59 Tests und Clippy mit `-D warnings` waren im GNOME SDK 51 erfolgreich.

### 2026-11-24 – Auftrag 20c

- Der Wayland-Hit-Test der alten Stichübersicht zeigte außerhalb des Inhalts die libadwaita-Abdunklung als `GtkWindowHandle`. Die Anwendung hatte dort kein eigenes Eingabeziel. Sie wartete auf die Klicksequenz am Fenster im Capture-Pfad eines nativen Fenstergriffs, der eigene Gesten verarbeitet. Die alten synthetischen Prüfungen konnten diesen Unterschied zur realen Bedienung nicht ausschließen. [libadwaita-Floating-Sheet](https://github.com/GNOME/libadwaita/blob/main/src/adw-floating-sheet.c), [GTK-Fenstergriff](https://github.com/GNOME/gtk/blob/main/gtk/gtkwindowhandle.c)
- Die Stichübersicht liegt nun über einer vollflächigen GTK-Zeichenfläche, die ihre eigenen Außengesten empfängt. Der Inhalt liegt als getrennte Vordergrundebene darüber. Der Tisch bleibt bis zum Schließen unbedienbar. Auswahl und Fokus bleiben erhalten. Ergebnisdialog und Bestätigungen verwenden weiterhin den bisherigen libadwaita-Dialogpfad.
- Der Wayland-Hit-Test der neuen Fassung bestätigte an drei Stellen den eigenen Hintergrund als Ziel. Über die vollständige GDK-Broadway-Ereigniskette wurden Mausbewegung, Druck und Loslassen links, rechts und unterhalb der Übersicht, ein Außentipp, Innenklick und Innenwischgeste, Escape sowie die Schließen-Schaltfläche geprüft. Auswahl und Fokus blieben erhalten, Tischkarten wurden nicht ausgelöst. Eine manuelle Prüfung mit physischer Maus und Touch auf dem betroffenen System steht aus.
- Formatierung, Cargo-Check, alle 58 Tests, Clippy mit `-D warnings`, Flatpak-Release-Build einschließlich Tests und Wayland-Fensterstart waren im GNOME SDK 51 erfolgreich.

### 2026-11-24 – Auftrag 20b

- Die Stichübersicht fängt Klick und Tippen außerhalb des Dialoginhalts ab und schließt ohne Weitergabe an den Tisch. Scrollen, Escape, die Schließen-Schaltfläche und Fokusrückgabe bleiben erhalten.
- Gemeinsamer modaler Ergebnisdialog für Hand und Partie. Er zeigt Gewinn oder Niederlage, Abschlussgrund, Augen und Spielpunkte. Schneider und Schwarz werden passend zur Wertung angezeigt. Beim Partieende kommen Bummerl und der abschließende Reststand hinzu. Nur der zentrale Weiter-Button setzt das Spiel fort.
- Formatierung, Cargo-Check, alle 58 Tests und Clippy mit `-D warnings` waren im GNOME SDK 51 erfolgreich. Flatpak-Release-Build samt Tests und Wayland-Fensterstart waren erfolgreich. Der zusätzliche Regressionstest unterscheidet Schneider und Schwarz bei regulären Gewinnen und erfolgreichem Zudrehen von der Strafwertung bei gescheitertem Zudrehen.
- Synthetische Broadway-Ereignisse prüften Außenklick und Außentipp, ausbleibende Tischereignisse, Fokus und offenbleibenden Dialog bei Inneninteraktionen. Unter Wayland wurden ein Scrollversatz von 80 logischen Pixeln und das Escape-Signal am GTK-Controller geprüft.
- Der isolierte Wayland-Prüflauf prüfte Handgewinn, Handverlust, gelungenes und gescheitertes Zudrehen sowie Partieende. Bei Zeichenflächen von 560 × 400, 680 × 513 und 1080 × 673 logischen Pixeln blieben die Tischgröße und die Wertung beim geöffneten Dialog stabil. Die Übergänge über „Nächste Hand“ und „Neue Partie“ gelangen. Physischer Touch, Scrollrad und HiDPI bleiben manuell zu prüfen.

### 2026-11-24 – Auftrag 20a

- Die obere Partiestandszeile entfällt. Die linke Spalte zeigt oben die gegnerischen und unten die eigenen Restpunkte und Bummerl. In der Mitte steht die Zahl abgeschlossener Hände. Große Ziffern, das kleinere Kürzel „B“ und die Schriftfarbe der Stichaugen gliedern die Anzeige.
- Die Spalte wird in den freien linken Rand gezeichnet und belegt keine zusätzliche GTK-Layoutfläche. Mit jedem Neuzeichnen stammen alle Werte direkt aus dem Partiezustand. Eine Beschreibung der Werte ist für assistive Technik am Spielfeld verfügbar.
- Formatierung, Cargo-Check, alle 57 Tests und Clippy mit `-D warnings` waren im GNOME SDK 51 erfolgreich. Die Layouttests prüfen Abstände zwischen Statusspalte und Kartenbereichen sowie die Lage der drei Gruppen bei verschiedenen Größen. Der Flatpak-Release-Build und der Wayland-Fensterstart waren erfolgreich. Cairo-Renderprüfungen bestätigten alle drei Gruppen bei 560 × 400, 680 × 560, 1280 × 720 und 1920 × 1080 logischen Pixeln. Im Vergleich unter Wayland bei unverändertem 680 × 560 großem Fenster wuchs die Zeichenfläche von 680 × 485 auf 680 × 513.

### 2026-11-24 – Auftrag 19

- Die Deckelung auf 138 logische Pixel Kartenbreite entfernt. Die Breite der eigenen Handkarten ergibt sich aus `min(Zeichenbreite / 6,45, Zeichenhöhe / 4,55)`. Relative Größen, Abstände, Stapelversätze, Auswahlhub, Symbole und Augenanzeigen wachsen mit. Der Platz für native GTK-Aktionen bleibt reserviert.
- Automatisierte Layoutprüfung bei 560 × 400, 680 × 560, 1280 × 720, 1920 × 1080, 2560 × 1440 und 3840 × 2160 logischen Pixeln. Zusätzliche Formate waren 2560 × 400, 560 × 900 und 200 × 130. Geprüft wurden fortgesetztes Wachstum, proportionale 3-fach-Skalierung, Spielbereiche bei offenem und gesperrtem Talon, freie Aktionszone, Stapelabstand und Kartentreffer nach Größenänderung.
- Cairo-Testflächen mit 100, 150 und 200 Prozent Geräteskalierung bestätigten dieselbe logische Kartenposition. Die Anwendung übernimmt die GTK-/Compositor-Skalierung ohne zusätzlichen Faktor. Physischer 4K-Monitor und GNOME-Skalierung auf realen Ausgaben bleiben manuell zu prüfen.
- Formatierung, Cargo-Check, alle 57 Tests und Clippy mit `-D warnings` waren im GNOME SDK 51 erfolgreich. Flatpak-Release-Build und Wayland-Fensterstart waren erfolgreich. Ein isolierter GTK-Test mit synthetischen Broadway-Ereignissen bestätigte Doppelklick, Tippen und Wischen sowie eine erhaltene Auswahl nach Zuteilung einer 3840 × 2160 großen Zeichenfläche.


### 2026-11-24 – Auftrag 18

- Die Engine beendet die Hand ab 66 wirksamen Augen nach vollständiger Stichauflösung oder sofort nach einer wirksamen Ansage. Vorgemerkte Ansagen zählen erst nach dem ersten eigenen Stich. Die Prüfung geht dem Nachziehen und der Letztstichwertung voraus. Die Wienerisch-Wertung bleibt erhalten und die Partie rechnet jede Hand genau einmal ab.
- Ausmeldebedienung und Ausmelde-Wartezustände entfernt. Die frühere Strafwertung ist im Regel-Audit und im vorigen Git-Stand dokumentiert.
- Beide Augenanzeigen stehen als Zahl über der maximalen Stapelhöhe. Die separate Lernhilfe für gegnerische Augen bleibt standardmäßig aus. Partierestpunkte und Bummerl stehen weiterhin getrennt.
- Formatierung, Cargo-Check, alle 54 Tests und Clippy mit `-D warnings` im GNOME SDK 51 erfolgreich. Der Flatpak-Release-Build einschließlich Meson-Testlauf war erfolgreich. Abgedeckt sind unter anderem die Simulation über 768 Hände, genau und mehr als 66 Augen, beide Seiten, 20er und 40er, Talonsperre, Mehrhandwertung, Stapelabstand und numerisches Format.
- Ein isolierter GTK-Test unter Wayland spielte mit der tatsächlichen GTK-Zeitsteuerung eine vollständige Hand. Der Mensch gewann automatisch bei 66 Augen zwei Spielpunkte. „Nächste Hand“ behielt den Partiestand. Physischer Touch und HiDPI bleiben manuell zu prüfen.

### 2026-11-24 – Auftrag 17

- Die Zeichenfläche bleibt bei Kontextaktionen, Handergebnis und modalen Bestätigungen gleich groß. Im mittleren Tischbereich stehen kontextabhängige Schaltflächen mit mindestens 44 logischen Pixeln Höhe. Bestätigungen sperren Eingaben, pausieren geplante Spielzüge und geben den Fokus zurück.
- Doppelklick verwendet die ursprünglich angeklickte Karte auch nach Verschiebung durch die Auswahl. Vorbereitete Paaransagen haben Vorrang. Im weichen Profil liegt der erste Computerstich offen beim Stapel. Wirksame eigene Augen stehen am eigenen Stapel, gegnerische Augen sind eine getrennte, standardmäßig ausgeschaltete Lernhilfe.
- Formatierung, Cargo-Check, alle 54 Tests einschließlich der Simulation über 768 Hände und Clippy im GNOME SDK 51 erfolgreich. Neue Tests prüfen Aktionszone, sichtbares Computerpaar ohne doppelte Darstellung, Doppelklick und Augenanzeige. Flatpak-Release-Build und Wayland-Fensterstart waren erfolgreich. Das Settings-Portal und der Session-D-Bus meldeten beim Start die bekannten Warnungen.
- Isolierte GTK-Tests unter Wayland bei Fenstergrößen 560 × 475 (Zeichenfläche 560 × 400), 680 × 560 und 920 × 680 bestätigten stabile Zeichenfläche, zentrierte Aktion, Eingabesperre und Fokusrückgabe nach Abbrechen. Das mittige Ausspielen hat in jeder Größe genau eine Karte ausgespielt. Die Escape-Abbruchantwort war konfiguriert. Ein separates Escape-Ereignis an der Bestätigung wurde nicht eingespeist. Synthetische GTK-Eingaben unter Broadway prüften Einzel- und Doppelklick, vorbereitete Ansage, Enter und Leertaste, Rechtsklick, Tippen und Wischen sowie Auswahl beim Größenwechsel.
- Physischer Touch und HiDPI bleiben manuell zu prüfen.

### 2026-11-24 – Auftrag 16

- Gemeinsame Hausregel für Trumpftausch und Zudrehen umgesetzt. Beide Profile verlangen einen vollständig aufgelösten ersten Stich, aktuelles Ausspielrecht vor Stichbeginn und mindestens vier Talonkarten einschließlich Trumpfkarte. Ein eigener Stich ist keine zusätzliche Voraussetzung.
- Regressionen prüfen beide Profile, die Grenze bei drei und vier Karten, einen begonnenen oder noch nicht aufgelösten Stich, die Talonsperre sowie unveränderte Zustände nach Ablehnung. Bestehende Testaufbauten ziehen den ersten Stich vor.
- Formatierung, Cargo-Check, alle 49 Tests einschließlich der Simulation über 768 Hände und Clippy waren im GNOME SDK 51 erfolgreich. Der Flatpak-Release-Build einschließlich Tests war erfolgreich. Maus- und Touchbedienung wurden für diesen Auftrag nicht manuell geprüft.

### 2026-11-23 – Auftrag 15

- Lokales Git-Repository auf Branch `main` initialisiert. `origin` verweist auf `https://github.com/linux-ds/schnapsen.git`.
- Initial-Commit: `7c55c4c20d5b3d0a6b07a372e2f569d3d3922e41`.
- Quellcode, Buildkonfiguration, Desktop-Dateien und Projektdokumentation für den Initial-Commit geprüft. Generierte Rust-, Meson-, Builder- und Flatpak-Dateien werden lokal ignoriert.
- Im GNOME SDK 51 waren `cargo fmt --check`, `cargo check`, `cargo test` und `cargo clippy` erfolgreich. Alle 49 Tests bestanden. Der Flatpak-Release-Build einschließlich Meson-Testlauf war erfolgreich.
- Der erste Push zu GitHub steht aus und erfolgt manuell über VS Code.

### 2026-11-23 – Auftrag 14

- Scharfes Zudrehen verlangt einen eigenen Stich. In beiden Profilen brauchen Trumpftausch und Zudrehen mindestens zwei verdeckte Talonkarten. Weiche Sticheinsicht bleibt auch nach Handende auf eigene Stiche und den ersten Gegnerstich begrenzt.
- Eine bestätigte falsche Ausmeldung zum zulässigen Zeitpunkt beendet die Hand mit 1 bis 3 Spielpunkten für die Gegenseite. Bei zugedrehtem Talon hat die eingefrorene Wienerisch-Wertung Vorrang. Ausmeldezeitpunkt und Erfolgsschwelle werden getrennt geprüft. Die Bestätigung ist von der Spielhilfe unabhängig.
- Drei Tests ergänzt und die bisherigen Regressionen an die verbindlichen Regeln angepasst. Die deterministische Simulation durchläuft weiterhin 768 Hände. Formatierung, Cargo-Check, alle 49 Tests und Clippy liefen im GNOME SDK 51 erfolgreich.
- Flatpak-Release-Build einschließlich Tests erfolgreich. Das Wayland-Fenster öffnete sich und lieferte gerenderte Puffer. Settings-Portal und Session-D-Bus meldeten die bekannten Startwarnungen. Ein isolierter GTK-Test unter Wayland betätigte die Dialogschaltflächen und bestätigte Spielhilfe-Unabhängigkeit, folgenlosen Abbruch, drei Strafpunkte nach Bestätigung und die nächste Hand mit erhaltenem Punktestand.
- [RULE_AUDIT.md](RULE_AUDIT.md) dokumentiert die abweichenden Quellenregeln zu Zeitpunkt, Einsicht und Strafwertung sowie die Projektfestlegungen. Außerhalb des Auftrags bleiben die physische Ausnahme bei ausmeldender Paaransage und die Augenanzeige im scharfen Profil zur späteren Abnahme offen.

### 2026-11-23 – Auftrag 13

- Austeilen, Stichablauf, Ansagen, Trumpftausch, Zudrehen, Ausmeldung, Hand- und Partiewertung, Computerzüge sowie GTK-Eingaben und Zustandswechsel geprüft.
- Reproduzierten Fehler beim zugedrehten Talon behoben. Das finale Ausmeldefenster beginnt erst nach der Auflösung des letzten Stichs. Der Regressionstest schlug vor der Korrektur fehl.
- Ein deterministischer Test spielt 768 Hände über beide Profile mit allen vier Abschlussgründen und prüft Kartenpartition, Zugfolge und unveränderten Zustand bei Ablehnung. Ein GLib-Test prüft das Entfernen bereits fälliger Quellen für Spielaktionen und sichtbare Effekte.
- Formatierung, Cargo-Check, 46 Tests und Clippy im GNOME SDK 51 erfolgreich. Der Flatpak-Release-Build samt Tests war erfolgreich. Unter Wayland öffnete sich das Fenster und lieferte gerenderte Puffer. Settings-Portal und Session-D-Bus meldeten die bekannten Startwarnungen.
- Eine synthetisch bediente GTK-Partie lief über fünf Hände bis zum Schneiderbummerl und startete anschließend mit erhaltenen Bummerln neu. Auswahl, Größenwechsel, Maus, Tastatur und synthetischer Touch wurden geprüft. Der Wayland-Dialogtest prüfte die Zugriffsprofile, Fokus und Eingabesperre bei zwei Fensterbreiten.
- Offen sind Profilentscheidungen zu Zudrehzeitpunkt, letztem Tausch und Zudrehen bei nur einer verdeckten Karte, gegnerischer Sticheinsicht und Strafwertung. Die bestehenden Projektregeln gelten weiter. Physischer Touch, Stichstapel per Klick oder Tippen, Scrollen, HiDPI und zeitkritische Dialogwechsel bleiben manuell zu prüfen.

### 2026-11-23 – Auftrag 12

- Die Engine gibt im weichen Profil alle eigenen Stichpaare oder genau das erste gegnerische Stichpaar frei. Im scharfen Profil verweigert sie die Einsicht. Dieselben Grenzen gelten nach Handende. Die globale Stichreihenfolge speichert nur die Gewinner. Die Karten bleiben in den vorhandenen Stichpaaren.
- Beide Stichstapel sowie **S** und **Umschalt+S** öffnen die jeweilige Übersicht. **Escape** und **Schließen** geben den Fokus zurück. Die Auswahl bleibt erhalten. Die Übersicht wartet auf einen stabilen Spielzustand und sperrt die Eingabe am Tisch.
- Formatierung, Cargo-Check, 44 Tests und Clippy liefen im SDK 51 erfolgreich. Der Flatpak-Release-Build einschließlich Tests und der Wayland-Fensterstart waren erfolgreich. Beim Start erschienen die bekannten Warnungen zum Settings-Portal und Session-D-Bus.
- Ein isolierter GTK-Test unter Wayland bestätigte bei Fensterbreiten von 626 und 920 logischen Pixeln die eigene Übersicht und genau ein gegnerisches Paar. Über GTK-Tastatursignale wurden **S**, **Umschalt+S**, **Escape**, blockiertes Ausspielen sowie der Erhalt von Fokus und Auswahl geprüft. Im scharfen Profil blieb der Dialog geschlossen. Ein separater Broadway-Versuch bestätigte die Ablehnung, konnte den erlaubten Dialog aber nicht vollständig anzeigen.
- Eine vollständige Analyse nach Handende wurde nicht ergänzt. Die manuelle Prüfung des Handwechsels nach geöffneter Übersicht und der Bedienung auf einem physischen Touchscreen bleibt offen.

### 2026-11-23 – Auftrag 11

- `core::match_play` zieht Handspielpunkte ausschließlich beim Handgewinner ab und begrenzt den Reststand auf null. Bei Partieabschluss erhält der Verlierer ein Bummerl oder beim Reststand sieben und aktivierter Schneiderregel zwei. Der Handwertungsmechanismus bleibt unverändert.
- Der Mensch ist beim ersten Start Vorhand. Geber und Vorhand wechseln nach jeder abgeschlossenen Hand unabhängig vom Gewinner, auch über die Partiengrenze. `SPEC.md` legt die erste Vorhand nicht fest. [Turnierregeln der Landjugend Bezirk Kirchdorf](https://www.bezirkkirchdorf.landjugend.at/uploads/tx_siwaeventinfo/Schnapsen.pdf)
- Separate Aktionen für nächste Hand, neue Partie und bestätigtes Zurücksetzen der kumulierten Bummerl eingebunden. Solange eine Partie läuft, kann sie weder ersetzt noch neu gewertet werden.
- `cargo fmt --check`, `cargo check`, `cargo test` und `cargo clippy` im GNOME SDK 51 erfolgreich. Alle 43 Tests bestanden, einschließlich Mehrhand-Partie, Schneiderregel und Abrechnung bei gelungenem oder fehlgeschlagenem Zudrehen.
- Ein isolierter GTK-Test spielte fünf Hände bis zum Stand 7:0. Er bestätigte den automatischen Beginn der Computerhand, die Schneideranzeige und den Neustart bei 7:7 unter Beibehaltung der zwei Bummerl.
- Flatpak-Release-Build mit Tests und Wayland-Fensterstart erfolgreich. Die bekannten Warnungen zum Settings-Portal und Session-D-Bus erschienen erneut.

### 2026-11-23 – Auftrag 10

- Die GTK-unabhängige Engine prüft und sperrt den Talon atomar. Der gegnerische Stand bleibt bis zum Ergebnis unverändert gespeichert und bestimmt die Wienerisch-Wertung. Nachziehen und Trumpftausch enden mit dem Zudrehen.
- Mit geschlossenem Talon entscheidet der letzte Stich die Hand nur insofern, als nach ihm eine letzte Ausmeldung möglich sein kann. Erreicht der Zudreher dabei 66 Augen und hat das Ausspielrecht, kann er ausmelden oder die Hand ohne Ausmeldung beenden. Ein letzter Stich allein bringt keinen Gewinn.
- Tastatur, Klick und Touch öffnen eine libadwaita-Bestätigung. Das Fenster zeigt danach eine querliegende Kartenrückseite auf dem Stapel. Das bestehende libadwaita-Paket nutzt dafür das Bindings-Feature `v1_5`.
- `cargo fmt --check`, `cargo check`, `cargo test` und `cargo clippy` im GNOME SDK 51 waren erfolgreich. Alle 38 Tests bestanden.
- Isolierte GTK-Prüfungen bestätigten Dialogöffnung per Maus, **Z** und synthetischem Touch, Abbrechen, die unveränderte Stapelgröße und beide Handergebnisse. Die gelungene Hand endete bei 77 zu 0 Augen, die fehlgeschlagene bei 6 zu 63 Augen. Der jeweilige Sieger erhielt drei Spielpunkte.
- Flatpak-Release-Build mit Tests und Wayland-Fensterstart erfolgreich. Die bekannten Warnungen zum Settings-Portal und Session-D-Bus erschienen erneut.

### 2026-11-23 – Auftrag 09

- Atomare Tauschaktion für den offenen Talon implementiert. Weich erlaubt den Tausch ohne eigenen Stich und ab einer verdeckten Karte. Scharf verlangt einen eigenen Stich und mindestens zwei verdeckte Karten.
- Maus, Touch und **T** nutzen dieselbe Tauschaktion. Handreihenfolge, Fokus und Kartenbestand bleiben konsistent. Der Computer tauscht eine höherwertige offene Karte vor seinem gewöhnlichen Zug.
- Ein weiterer Tausch wird anhand des aktuellen Zustands geprüft. Mit den derzeitigen Aktionen gelangt der einzige Trumpf-Unter nach einem Tausch erst beim letzten Nachziehen wieder auf eine Hand. Zu diesem Zeitpunkt ist der Talon leer. Der Test für einen weiteren berechtigten Spieler stellt dessen Kartenbesitz deshalb gezielt her.
- `cargo fmt --check`, `cargo check`, `cargo test` und `cargo clippy` im GNOME SDK 51 erfolgreich. Alle 32 Tests bestanden.
- Isolierte GTK-Prüfungen bestätigten Maus, synthetischen Touch, Tastatur, Computerzug nach Tausch und den Hinweis bei abgelehntem Tausch. Flatpak-Release-Build und Wayland-Fensterstart waren erfolgreich. Die bekannten Warnungen zum Settings-Portal und Session-D-Bus erschienen erneut.

### 2026-11-23 – Auftrag 08

- Atomare 20/40-Ansage mit hergeleiteten wirksamen und vorgemerkten Augen für weich und scharf ergänzt. Nach wirksamer Ansage ist die direkte Ausmeldung vor der Antwort möglich.
- Profilwahl für neue Hände, eigene Schaltflächen für König oder Ober, kurze Markierung des Paars und Anzeige des Ansagewerts eingebunden. Der Computer nutzt dieselbe Aktion.
- `cargo fmt --check`, `cargo check`, `cargo test` und `cargo clippy` im GNOME SDK 51 erfolgreich. Alle 27 Tests bestanden.
- Der isolierte GTK-Test bestätigte einen menschlichen Vierziger mit Ober und einen Computer-Vierziger mit König. Die Einblendung und Markierung beider Karten erschienen in beiden Fällen.
- Flatpak-Release-Build mit Tests und Wayland-Fensterstart erfolgreich. Die bekannten Warnungen zum Settings-Portal und Session-D-Bus traten erneut auf.

### 2026-11-23 – Auftrag 07

- Augen aus gewonnenen Stichpaaren berechnet. Ausmeldung vor dem nächsten Ausspiel geprüft und Handergebnis mit Sieger, Verlierer, Grund, Augen, Stichzahlen und Spielpunkten ergänzt.
- Ausmeldung und Augenanzeige an GTK angebunden. Der Computer verwendet dieselbe Regelaktion. Der Stichgewinner kann sich nach dem Nachziehen und vor dem nächsten Ausspiel ausmelden. Der letzte Stich entscheidet sofort, wenn die Hand noch läuft.
- Im GNOME SDK 51 waren `cargo fmt --check`, `cargo check`, `cargo test` und `cargo clippy` erfolgreich. Alle 23 Tests bestanden, einschließlich der Grenzen bei 65, 66 und 33 Augen, der automatischen Ausmeldung des Computers und des letzten Stichs.
- Der isolierte GTK-Test zeigte eine Ausmeldung bei 66 zu 8 Augen mit zwei Spielpunkten und einen Gewinn durch den letzten Stich bei 54 zu 66 Augen mit einem Spielpunkt. Beide Neustarts waren erfolgreich.
- Flatpak-Release-Build samt Tests und Wayland-Fensterstart erfolgreich. Die bekannten Warnungen zum Settings-Portal und Session-D-Bus traten erneut auf.

### 2026-11-23 – Auftrag 06

- Gemeinsame Zugermittlung für Engine, Computer und Spielhilfe ergänzt. Die Antwortpriorität ist höherstechen, Farbe bedienen, Trumpf spielen, abwerfen. Bei offenem Talon und beim Ausspielen bleibt die Wahl frei.
- Nach dem letzten Nachziehen laufen weitere fünf Stiche ohne Kartenaufnahme. Der Gewinner eröffnet den nächsten Stich. Der Abschlusszustand blockiert weitere Ausspielaktionen und zeigt einen Hinweis zur ausstehenden Wertung.
- `cargo fmt --check`, `cargo check`, `cargo test` und `cargo clippy` im GNOME SDK 51 erfolgreich. Alle 20 Tests bestanden, darunter 16 deterministische Kartenfolgen über zehn Stiche und Computerzüge mit genau einer zulässigen Antwort.
- Der isolierte GTK-Test bestätigte zehn Stiche, einen abgewiesenen Zug bei unveränderten Karten, den Abschluss mit sichtbarem Hinweis und den Neustart. Flatpak-Release-Build samt Tests und Wayland-Fensterstart waren erfolgreich. Die bekannten Warnungen zum Settings-Portal und Session-D-Bus erschienen erneut.

### 2026-11-23 – Auftrag 05

- Geprüftes Ausspielen, Stichentscheidung, Stichpaare, Augen, Nachziehen und Stopp bei leerem Talon implementiert. Die Engine weist ungültige Aktionen mit strukturierten Fehlern ab.
- Deterministischen Computerzug über dieselbe Engine eingebunden. Rechtsklick, Leertaste und Wischgeste sind wieder aktiv. GLib-Timer zeigen beide Tischkarten vor der Auflösung und werden bei einem Neustart entfernt.
- `cargo fmt --check`, `cargo check`, `cargo test` und `cargo clippy` waren im GNOME SDK 51 erfolgreich. Alle 17 Tests bestanden. Der Flatpak-Release-Build samt Tests war erfolgreich.
- Isolierter GTK-Test mit synthetischen Maus-, Tastatur- und Touchereignissen bestätigte beide offenen Tischkarten, den Neustart während der laufenden Stichpause, fünf Stiche mit Nachziehen sowie den sichtbaren Hinweis bei leerem Talon.
- Der Flatpak-Fensterstart unter Wayland war erfolgreich. Die bekannten Warnungen zum Settings-Portal und Session-D-Bus erschienen erneut.

### 2026-11-23 – Auftrag 04

- Die Anfangsverteilung verwendet die Abhebevariante mit je drei Karten, Aufdecken der siebten Karte als Trumpf und danach je zwei Karten. Trumpf liegt am Ende der Talon-Ziehreihenfolge. [Turnierregeln der Landjugend Bezirk Kirchdorf](https://www.bezirkkirchdorf.landjugend.at/uploads/tx_siwaeventinfo/Schnapsen.pdf)
- Spielzustand, gefilterte Sicht und **Neues Spiel** ergänzt. Die Anwendung mischt mit GLib über die steuerbare Schnittstelle von `core::game`. Cargo-Abhängigkeiten und Spezifikation bleiben unverändert.
- `cargo fmt --check`, `cargo check`, `cargo test` und `cargo clippy` im GNOME SDK 51 bestanden. Alle zwölf Tests waren erfolgreich, einschließlich Kartenpartition, reproduzierbarer Verteilung, Trumpf, Neustart und gefilterter Darstellung.
- Synthetische GTK-Ereignisse bestätigten Auswahl per Maus, Tastatur und Touch bei unveränderter Kartenanzahl, auch nach einer Größenänderung. Rechtsklick, Leertaste und Wischen lösten keine Kartenbewegung aus.
- Flatpak-Release-Build samt Tests und Fensterstart unter Wayland erfolgreich. Das Settings-Portal und Session-D-Bus meldeten die bekannten Warnungen.

### 2026-11-23 – Auftrag 03

- Kartenauswahl, Fokusnavigation, Ausspielen und Touch-Gesten am Demo-Tisch implementiert. Zwei ausgewählte Karten sowie eine belegte Tischmitte mit zwei Karten blockieren das Ausspielen.
- Im GNOME SDK 51 liefen `cargo fmt --check`, `cargo check`, `cargo test` und `cargo clippy` erfolgreich. Alle zehn Tests bestanden. Die neuen Fälle decken Auswahlgrenze, Zurücksetzen, Reihenfolge, Kartenerhalt, Fokus, Touch-Schwellen sowie Trefferprüfung bei Überlappung und Größenänderung ab.
- Isolierter GTK-Test mit synthetischen Broadway-Ereignissen erfolgreich für Linksklick, Rechtsklick, Pfeilnavigation, Enter, Escape, Leertaste, Tippen und Wischen. Die Auswahl blieb bei einer neuen Widget-Zuteilung von 560 × 400 auf 920 × 600 Pixel erhalten. Die Touchprüfung verwendete synthetische Ereignisse.
- Flatpak-Release-Build samt Tests und Fensterstart unter Wayland erfolgreich. Die bekannten Warnungen zum Settings-Portal und Session-D-Bus traten beim Teststart erneut auf.

### 2026-11-23 – Auftrag 02

- Zusammenhängenden 2D-Kartentisch mit allen sechs Bereichen implementiert. Die Kartenproportion beträgt 5:7. Ausgespielte Karten, Gegnerhand, Talon und Stichstapel sind 95, 55, 70 bzw. 60 Prozent der Referenzgröße.
- Flatpak-Release-Build mit SDK 51 erfolgreich. `cargo fmt --check`, `cargo check`, `cargo test` und `cargo clippy` offline im SDK erfolgreich. Zwei Kartenmodelltests und drei neue Tests für Eindeutigkeit, Stapelversatz, Kartenproportionen, Zeichenflächengrenzen und Überdeckungen bestanden.
- Cairo-Ausgaben visuell bei 560 × 400, 920 × 600, 560 × 900 und 1440 × 400 geprüft. Beim Fensterstart über `flatpak-builder --run` wurde ein Puffer an Wayland übergeben. Im Testlauf erschienen die bekannten Warnungen zum Settings-Portal und Session-D-Bus.
- Die AppStream-Beschreibung nennt den Demo-Tisch. Die Spezifikation bleibt unverändert.

### 2026-11-23 – Auftrag 01b

- Flatpak-Manifest, Meson-Anbindung, Cargo-Quellenliste und Desktop-Integration ergänzt. Alle 75 Crate-Archive stimmen mit `Cargo.lock` überein.
- Auf Flathub sind Platform und SDK in den Zweigen 50 und 51 für x86_64 und aarch64 vorhanden. Der Build wurde auf x86_64 mit Zweig 51 ausgeführt. Das SDK wurde als Benutzer-Flatpak installiert.
- Flatpak-Build mit dem in GNOME Builder 50.0 enthaltenen `flatpak-builder` 1.4.7 erfolgreich. Cargo-Release-Build, Cargo-Tests, AppStream-Komposition und Export der Desktop-Datei, des Icons und der Metadaten waren erfolgreich. Die Cargo-Quellen wurden aus dem fertigen Paket entfernt.
- Der erste Cargo-Testlauf überschritt Mesons Standardzeitlimit beim erneuten Kompilieren der Abhängigkeiten. Der erfolgreiche Lauf verwendet das Release-Profil des Builds.
- `cargo clippy --all-targets --locked --offline` im SDK und `cargo fmt --check` erfolgreich. Desktop-Datei und XML-Syntax validiert. Die separate AppStream-Validierung meldet fehlende Homepage, Entwicklerangaben und Release-Metadaten sowie Großbuchstaben in der vorläufigen App-ID.
- Wayland-Smoke-Test via `flatpak-builder --run` erfolgreich. Das Fenster meldete den Titel „Schnapsen“ und übergab einen gerenderten Puffer an den Compositor. Im Testlauf gab es Warnungen zum Settings-Portal und Session-D-Bus. Bauen und Ausführen über die grafische Builder-Oberfläche stehen zur Abnahme aus.

### 2026-11-23 – Auftrag 01

- Rust-Projekt, Fenstercode und Kartenmodell angelegt. `SPEC.md` und `PROJECT.md` sind bytegleich.
- `cargo fmt --check` erfolgreich. Beide Kartenmodelltests bestanden mit `rustc --edition=2021 --test src/lib.rs` und anschließendem Start des Testprogramms.
- `cargo check`, `cargo test` und `cargo clippy` ausgeführt. Alle drei Befehle endeten beim Build der GTK4-Bindings wegen fehlender GTK4-Entwicklungsbibliotheken.
- Für die Prüfungen wurde Rust 1.100.0 temporär unter `/tmp` bereitgestellt. Es wurden keine Systempakete installiert.

### 2026-11-22
- `SPEC.md` und `PLAN.md` als initiale Projektunterlagen angelegt.
- Noch kein Code, Build oder Test ausgeführt.

### Vorlage für spätere Einträge

```text
### YYYY-MM-DD – Kurzbeschreibung
- Implementiert: ...
- Geändert: ...
- Tests: Befehl und Ergebnis / nicht ausgeführt + Grund
- Offene Punkte: ...
- Nächster Schritt: ...
```
