# GNOME Schnapsen – Projektspezifikation

**Status:** Konzept / Grundlage für die erste Implementierung  
**Zielversion:** 0.1 (spielbarer Prototyp)  
**Plattform:** GNOME auf Linux, Verteilung als Flatpak  
**Sprache dieser Spezifikation:** Deutsch

## 1. Projektziel

Entwicklung eines nativen, schlanken Schnapsenspiels für GNOME. Das Spiel soll sich wie ein physischer Kartentisch in Draufsicht anfühlen, ohne eine klassische Game-Engine oder eine physikalische Simulation einzusetzen. Im Vordergrund stehen korrekte Spielregeln, natürliche Bedienung und ein nachvollziehbarer Spielzustand. Grafische Perfektion ist **kein** Ziel der ersten Version.

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
- Einfacher, legal spielender Computergegner; Architektur für spätere Verbesserung offenhalten.
- Tischdarstellung mit eigener Hand, kleiner verdeckter Gegnerhand, Talon mit Trumpfkarte, Tischmitte und sichtbaren gewonnenen Stichstapeln.
- Auswahl einzelner Karten, gegebenenfalls zweier Karten zur Vorbereitung einer Ansage.
- Ausspielen per Tastatur und Touchgeste sowie eindeutige Mausbedienung.
- Einsicht in Stichstapel entsprechend den gewählten Regeln; optionale Spielhilfen.
- Grundlegende Animationen: Auswählen, kurzes Verschieben, Ein-/Ausblenden und sichtbare Aktualisierung von Stapeln.
- Lokale Einstellungen; keine Netzwerkverbindung erforderlich.

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

### Aktionen (fachliche Schnittstelle)

Beispiele: `PlayCard`, `AnnounceMarriage`, `ExchangeTrump`, `CloseTalon`, `StartNextDeal`, `NewMatch`. Nur im aktuellen Zustand gültige Aktionen werden akzeptiert. Ungültige Aktionen geben einen verständlichen Fehler zurück, ohne den Zustand zu ändern.

Eine 20/40-Ansage bedeutet **nicht**, dass zwei Karten ausgespielt werden: Die Auswahl von König und Ober unterstützt die Bedienung; tatsächlich wird nur eine Karte gespielt.

### Regelqualität

Die exakten Abläufe und Sonderfälle müssen anhand einer ausdrücklich gewählten, nachvollziehbaren Regelfassung spezifiziert werden, insbesondere:

- Karten- und Stichzwang vor/nach dem Ende beziehungsweise Zudrehen des Talons.
- Bedingungen für Ansagen und Trumpftausch.
- Zeitpunkt der Augenwertung und der 66-Augen-Meldung.
- Spielpunkte und Sonderwertung bei zugedrehtem Talon.
- Letzter Stich, Talonende und sonstige Rundenausgänge.
- Unterschiede von *weich* und *scharf*, insbesondere das Einsehen alter Stiche.

**Keine Hausregel stillschweigend als allgemeingültig implementieren.** Offene Varianten werden dokumentiert, entschieden und mit gezielten Tests abgesichert.

## 5. Spielfeld und Interaktion

### Grundlayout: Draufsicht

- **Oben:** Gegnerische Hand, nur Kartenrückseiten, kleiner als die eigene Hand. Anzahl ist jederzeit sichtbar.
- **Mitte:** Bereich für die aktuell ausgespielten Karten.
- **Seitlich:** Talon mit erkennbarer Kartenanzahl und sichtbar liegender Trumpfkarte.
- **Unten:** Eigene Hand, größer und gut lesbar.
- **Rechts unten:** Eigener gewonnener Stichstapel; gegnerischer Stapel in dessen Nähe auf der Gegenseite.

Die Bereiche sollen sich an Fenstergröße und Seitenverhältnis anpassen. Kartengrößen sind **absichtlich nicht überall gleich**. Priorität hat die Lesbarkeit der eigenen Karten und des aktuellen Stichs.

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
- Für reine Mausbedienung muss es ebenfalls eine klare Bestätigung ohne Tastatur geben (z. B. Doppelklick oder kontextabhängige Aktion; konkret im Prototyp validieren).
- `Esc` schließt Overlays und hebt gegebenenfalls eine Auswahl auf.
- Interaktive Tischobjekte wie der Talon dürfen kontextbezogene Aktionen anbieten, z. B. Zudrehen oder Trumpftausch; kritische Aktionen benötigen eine erkennbare Bestätigung, wo sinnvoll.

### Stichübersicht

- Ein Klick/Tippen auf einen gewonnenen Stapel oder ein Tastenkürzel öffnet ein Overlay über dem Tisch.
- Die Übersicht kann Karten nach Stichfolge darstellen; eine zusätzliche Sortierung nach Farbe ist optional.
- Ob und welche eigenen oder gegnerischen Stiche eingesehen werden dürfen, hängt vom gewählten Regelprofil ab.
- Eine mögliche Regel für den ersten gegnerischen Stich wird **nicht** ohne vorherige Verifizierung als universell angenommen.
- Spielhilfen, die mehr Informationen als die klassische Regel erlauben, müssen ausdrücklich als solche gekennzeichnet und optional sein.
- Das Overlay ist auch mit Tastatur und Touch vollständig bedienbar.

### Animationen

Kurze Animationen genügen, z. B. 120–350 ms abhängig vom Vorgang. Auswahl bewegt die Karte wenige Pixel; Ausspielen kann durch kurzes Aus- und Einblenden statt einer Flugbahn erfolgen. Ein gewonnener Stich verschwindet aus der Mitte und wächst als realer Kartenstapel an. Animationsdauer bzw. reduzierte Bewegung ist einstellbar. Keine dauerhaft laufende Spielschleife erforderlich.

## 6. Kartengrafik und Lizenzierung

**Erste Phase:** Karten programmatisch als einfache, gut lesbare Rechtecke mit Werten und deutschen Farbsymbolen darstellen. Keine externen Kartenbilder notwendig.

**Später:** Historisches doppeldeutsches Kartenbild als austauschbares Theme ergänzen. Rechte und Quelle des konkreten Kartensatzes **vor** der Übernahme prüfen; keine modernen Herstellerbilder ohne ausreichende Lizenz verwenden. Assets, Schriftarten, Texturen und Sounds mit Quelle und Lizenz dokumentieren.

Einheitliche Karten-Geometrie, Hit-Tests und Interaktionen gelten unabhängig vom Theme. Die grafische Ebene darf anstelle von Bilddateien auch einfache Vektorformen zeichnen.

## 7. Einfacher Computergegner

- Erzeugt ausschließlich regelkonforme Aktionen.
- Beginnt mit einfachen Heuristiken für Trumpf, Augen, sichere Stiche und Ansagen.
- Verhalten darf nicht auf verdeckten Informationen des Menschen beruhen.
- Entscheidungen müssen reproduzierbar testbar sein.
- Ein späterer stärkerer Gegner soll das gleiche Regel- und Aktionsinterface benutzen.

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

## 10. Abnahmekriterien für den ersten spielbaren Prototyp

Ein Prototyp ist erst dann spielbar, wenn eine vollständige Partie gegen den Computer ohne manuelle Zustandskorrekturen durchlaufen werden kann, alle Züge gültig sind, Punktezählung und Rundenausgang stimmen und der Tisch bei normaler Desktop-Fenstergröße bedienbar ist. Einfache Kartenbilder sind ausdrücklich ausreichend. Noch nicht verifizierte Regelvarianten dürfen nicht als vollständig implementiert gekennzeichnet werden.

## 11. Offene Entscheidungen

1. Verbindliche Referenz für die Varianten *weich* und *scharf* und präzise Einsichtsrechte für Stiche.
2. Exakte Mausgeste für das Ausspielen und Bestätigung einer 20/40-Ansage.
3. Minimale unterstützte Fenstergröße und Verhalten bei sehr schmalen Fenstern.
4. Konkrete App-ID, Anzeigename und Lizenz des Quellcodes.
5. Ob Partiestände bereits in v0.1 automatisch gespeichert werden sollen.

Diese Punkte blockieren nicht den Beginn der Regel-Engine, müssen aber vor dem entsprechenden Implementierungsschritt entschieden werden.
