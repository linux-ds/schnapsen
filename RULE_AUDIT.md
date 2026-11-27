# Regel-Audit für GNOME Schnapsen

Maßstab für die Implementierung sind SPEC.md und die Festlegungen der Aufträge 04 bis 18. Die drei Referenzen beschreiben zum Teil unterschiedliche Spielweisen. „Abweichung“ bezeichnet auch gewollte Unterschiede zum Tischspiel. „Ungeklärt“ kennzeichnet eine ausstehende Profilentscheidung.

| Regel oder Sonderfall | Aktuelle Implementierung | Referenzregel | Bewertung | Empfohlene Maßnahme |
|---|---|---|---|---|
| Deck und Augen | 20 doppeldeutsche Karten, Rang und Augen 11/10/4/3/2 | K, P und T stimmen überein | Korrekt | Beibehalten |
| Geben, Trumpf und Talon | Vorhand 3, Geber 3, siebte Karte offen, Vorhand 2, Geber 2. Offene Karte ist die letzte Ziehkarte. | Abheben bei K sowie Standard bei P und T. K erlaubt auch Klopfen mit 5/5 und elfter Karte als Trumpf. | Korrekt für die gewählte Austeilfolge | Abhebevariante beibehalten |
| Geberwechsel und erste Vorhand | Mensch beginnt zuerst, danach Wechsel pro Hand, auch über die Partiegrenze. | K und T bestimmen den ersten Geber per Kartenziehen. P lässt die Erstwahl frei. Alle wechseln danach. | Abweichung, bewusste Projektentscheidung | Erste Vorhand als Projektfestlegung dokumentieren |
| Offener Talon und Stichgewinner | Freie Antwort. Höhere gleichfarbige Karte oder Trumpf sticht. Gewinner zieht vor dem Verlierer und spielt aus. | K, P und T stimmen in der Stich- und Ziehfolge überein. | Korrekt | Beibehalten |
| Leerer oder gesperrter Talon | Höherstechen in der Farbe, Farbe bedienen, trumpfen, abwerfen. Kein weiteres Ziehen. | K, P und T. T verlangt bei scharf das Ablegen vor dem Ziehen. | Korrekt | Digitale Stichauflösung vor dem Ziehen beibehalten |
| Regelwidrige Antwort | Engine weist die Aktion ohne Mutation ab. | K und T werten eine gespielte Renonce mit drei Punkten für den Gegner. | Abweichung, bewusster Eingabeschutz | Abgewiesene Eingabe weiterhin vom tatsächlich gespielten Regelverstoß unterscheiden |
| 20/40 und Gutschrift | Paar in derselben Farbe bei Ausspielrecht, einmal je Farbe. Weich: beide Karten erlaubt, Augen ab erstem eigenem Stich. Scharf: eigener Stich und König erforderlich. | Weiches Verfahren bei K, P und T. Scharfe Ansagebedingungen bei P und T. | Korrekt | Beibehalten |
| Ansage nach Talonende oder Zudrehen | Zulässig bei erfüllten Profilbedingungen. | P nennt die Ansage bei gesperrtem Talon ausdrücklich. | Korrekt | Beibehalten |
| Ansage, die 66 erreicht | Die regelkonform ausgespielte Paar-Karte bleibt offen liegen. Die Hand endet automatisch vor der Antwort. | K nimmt die ausmeldende Ansage von der Ausspielpflicht aus. P und T verbinden die Ansage grundsätzlich mit dem Ausspiel einer Paar-Karte. | Abweichung, automatische Hausregel | Ansage automatisch werten und Paar-Karte ausspielen |
| Frühester Trumpftausch und frühestes Zudrehen | Beide Profile verlangen einen vollständig aufgelösten Stich der Hand, aktuelles Ausspielrecht und einen noch nicht begonnenen Stich. Beim Tausch ist weiterhin der Trumpf-Unter erforderlich. | K und T knüpfen das Zudrehen ans Ausspielrecht. P beschreibt vorheriges Ziehen und nennt zugleich ein Beispiel zum Handbeginn. Die Regeln für den ersten Tausch unterscheiden sich nach Quelle und Profil. | Bewusste Hausregel. Ein eigener Stichgewinn ist keine zusätzliche Bedingung. | Gemeinsame Prüfung beibehalten |
| Talonuntergrenze für Tausch und Zudrehen | Mindestens vier Karten im offenen Talon einschließlich der offenen Trumpfkarte. | K verbietet beide Aktionen bei einer verdeckten Karte. T erlaubt dort den weichen Tausch. P und T nennen für scharf ein Verbot bei zwei Talonkarten. | Bewusste Hausregel mit ausdrücklich gezählter offener Trumpfkarte | Grenze von vier Karten beibehalten |
| Wienerisch-Wertung | Beim Gewinn des Zudrehers zählt der beim Zudrehen eingefrorene gegnerische Stand. Erreicht der Gegner zuerst 66 oder scheitert der Zudreher, erhält der Gegner drei Punkte, falls damals stichlos, sonst zwei. | P und K werten nach dem eingefrorenen Stand. T beschreibt zusätzlich eine Turniervariante mit Endstand. | Wienerisch-Wertung bleibt erhalten. Der Abschluss erfolgt automatisch. | Eingefrorenen Stand beibehalten |
| Abschluss bei 66 Augen | Nach vollständiger Stichauflösung oder wirksamer Ansage endet die Hand automatisch. | K, P und T verlangen eine Ausmeldung. | Bewusste Hausregel | Gemeinsame Wertung für Mensch und Computer |
| Falsches Ausmelden | Historische Regel aus Auftrag 14. Eine normale Falschmeldung wurde nach den hypothetischen Gewinnpunkten des Ausmeldenden bewertet. Bei Talonsperre hatte die eingefrorene Wienerisch-Wertung Vorrang. | K und T nennen die hypothetische Gewinnwertung. P beschreibt eine andere Strafregel. | Historisch im Git-Stand vor Auftrag 18 unter `f7b3297` erhalten. Im automatischen Profil entfällt der Ausmeldebefehl und damit die Strafwertung. | Historische Regel bei Rückkehr zur Ausmeldevariante erneut bewerten |
| Letzter Stich | Die 66-Augen-Prüfung hat Vorrang. Unter 66 entscheidet der letzte Stich regulär mit einem Punkt. Bei gesperrtem Talon greift stattdessen die Verlustwertung. | K und T nehmen die Sperre von der Letztstichregel aus. P gibt regulär einen Punkt. | Automatische Hausregel mit Letztstichwertung unterhalb der Schwelle | Reihenfolge der Abschlussprüfungen beibehalten |
| Restpunkte und Bummerl | Ab 7:7 bis null. Eine Wertung pro Hand. Verlierer erhält ein Bummerl oder bei 0:7 und aktiver Schneiderregel zwei. Bummerl summieren sich über neue Partien. | K, P und T beschreiben die 7-Punkte-Zählung. K und T nennen die traditionelle Doppelwertung, die Turnierausnahme und Begegnungen über mehrere Bummerl. | Korrekt für den Projektumfang | Einzelergebnis bis null und kumulierte Zählung beibehalten |
| Sticheinsicht | Weich eigene Stiche und genau der erste Gegnerstich, auch nach Handende. Scharf gesperrt. | T entspricht dem Umfang. K nennt den Ausspieler. P beschränkt die gegnerische Einsicht auf den gerade beendeten Stich vor dem nächsten Ausspiel. | Bewusstes Projektprofil ohne Rollenbindung und Stich-Zeitfenster. GTK wartet weiterhin auf einen stabilen Zustand. | Keine weiteren gegnerischen Stiche freigeben |
| Dauerhaft offener erster Computerstich | Weich liegt das erste tatsächlich vom Computer gewonnene Kartenpaar offen neben dem übrigen verdeckten Stapel. Scharf bleibt es verdeckt. | K, P und T beschreiben eine Einsicht in abgelegte Karten mit jeweils eigenen Bedingungen. | Bewusste Hausregel für die Tischdarstellung. | Erstes Paar aus den vorhandenen Stichen ableiten und nur einmal zeichnen. |
| Augen im scharfen Profil | Die wirksamen eigenen Augen stehen am Stapel. Gegnerische Augen der laufenden Hand erfordern eine separate Lernhilfe. | P und T beschreiben eigenes Mitzählen. | Abweichung, digitale Bedienhilfe | Bei Profilabnahme entscheiden, ob die Anzeige der Spielhilfe zugeordnet wird |

**Referenzen**  
K · [Landjugend Bezirk Kirchdorf, Schnapsen – Die Regeln](https://www.bezirkkirchdorf.landjugend.at/uploads/tx_siwaeventinfo/Schnapsen.pdf)  
P · [Pagat, Schnapsen](https://www.pagat.com/de/marriage/schnaps.html)  
T · [TS Spielkarten, Schnapsen Spielregeln](https://ts-spielkarten.de/spielregeln-kartenspiele/schnapsen-spielregeln/)

## Stabilität und Abdeckung

- Der deterministische Verlaufstest spielt 768 Hände in beiden Profilen mit wechselnder Vorhand. Er prüft die Kartenpartition, die Talonuntergrenze, Ansagen und die automatischen Abschlüsse.
- Der Computer erhält legale Ansagen und Karten für seine eigene Hand sowie gegebenenfalls den öffentlichen Trumpftausch. Beide Seiten gewinnen über dieselben Spielaktionen der Engine. Automatisches Zudrehen bleibt deaktiviert. Die Mehrhandtests prüfen Restpunkte, Vorhand, Schneider und Schutz gegen Doppelwertung.
- Hand- und Partiewechsel verwerfen ausstehende GLib-Quellen synchron. Der Regressionstest prüft, dass bereits fällige Spiel- und Effekttimer nach dem Abbruch beim Dispatch ausbleiben.

## Oberflächenprüfung

- GTK unter Broadway mit synthetischen Ereignissen geprüft. Linksklick, Rechtsklick, Pfeile, Enter, Escape, Leertaste, Tippen und Wischen funktionierten in den geprüften Situationen. Die Kartenauswahl blieb beim Wechsel von 560 × 400 auf 920 × 600 erhalten.
- Eine GTK-Testpartie über fünf Hände endete mit einem Schneiderbummerl. Der Computer eröffnete seine Hände. Die neue Partie begann bei 7:7 und übernahm die zwei Bummerl.
- GTK unter Wayland bei 626 und 920 logischen Pixeln Fensterbreite geprüft. Eigene Einsicht und genau ein gegnerisches Paar waren bei weich verfügbar. Scharf verweigerte die Einsicht. Synthetische Tastatursignale prüften Escape, Fokus, erhaltene Auswahl und gesperrtes Ausspielen.
- Offen bleiben physischer Touchscreen, Klick und Tippen auf beide Stichstapel, Scrollen, HiDPI sowie Einsicht und Handwechsel in zeitlicher Nähe zu einem Computerzug in einer manuellen Sitzung.

## Profilmatrix für Auftrag 27

Quellenabgleich am 26. November 2026. Die beiden Pagat-Seiten sind Sprachfassungen derselben Quelle. Die neuen Kombinationen übernehmen jeweils ausgewählte Regeln. Die Bezeichnung „Turnierregeln“ bezeichnet die hier dokumentierte Kombination aus klassischer Spielweise und Turnier-Zudrehwertung.

| Merkmal | Hausregeln – Weich / Scharf | Klassisch – Weich | Klassisch – Scharf | Turnier – Weich / Scharf |
|---|---|---|---|---|
| Technische IDs | `soft` / `sharp` | `classic_soft` | `classic_sharp` | `tournament_soft` / `tournament_sharp` |
| Ausmeldung | Automatisch ab 66 | Manuell | Manuell | Manuell |
| Ausmeldefenster | Entfällt | Stichgewinner vor Nachziehen; Ansagender vor gegnerischer Antwort | Wie weich | Wie klassisch |
| Verzicht | Entfällt | Weiter schließt das Fenster und setzt Spiel oder Letztstichwertung fort | Wie weich | Wie klassisch |
| Falsche Ausmeldung ohne Sperre | Entfällt | Gegner erhält 3 falls aktuell stichlos, sonst 2 | Wie weich | Wie klassisch |
| Falsche Ausmeldung mit Sperre | Entfällt | Gegner erhält den höheren Wert aus normaler Falschmeldung und Sperrwert: 3 bei stichlosem Nichtzudreher zum Sperrzeitpunkt, sonst 2 | Wie weich | Wie klassisch |
| Ansage und Gutschrift | Weich König oder Ober, auch ohne eigenen Stich; Augen ab erstem eigenen Stich. Scharf König und eigener Stich. | Weich wie Hausregel | Scharf wie Hausregel | Jeweils wie klassisch |
| Tausch bei offenem Talon, vor Ausspiel, mit Trumpf-Unter | Ab erstem abgeschlossenen Stich, mindestens 4 Karten insgesamt | Ab Spielbeginn, mindestens 2 insgesamt | Nach erstem abgeschlossenen Stich, mindestens 4 insgesamt | Jeweils wie klassisch |
| Zudrehen bei offenem Talon vor Ausspiel | Nach erstem abgeschlossenen Stich, mindestens 4 insgesamt | Ab Spielbeginn, mindestens 4 insgesamt | Ab Spielbeginn, mindestens 4 insgesamt | Jeweils wie klassisch |
| Erfolgreiches Zudrehen | Gegnerstand beim Zudrehen | Gegnerstand beim Zudrehen | Gegnerstand beim Zudrehen | Gegnerstand am Handende |
| Fehlschlag oder zuerst gültige gegnerische Ausmeldung | Für Nichtzudreher 3 falls beim Zudrehen stichlos, sonst 2 | Wie Hausregel | Wie Hausregel | Wie Hausregel |
| Handende ohne Ausmeldung | Bestehende automatische Prüfung, danach letzter Stich für 1 oder Verlust des Zudrehers | Letzter Stich für 1; bei Sperre Verlust des Zudrehers | Wie weich | Wie klassisch |
| Sticheinsicht während der Hand | Weich eigene und erster Gegnerstich; scharf gesperrt | Eigene Stiche | Gesperrt | Jeweils wie klassisch |
| Sticheinsicht nach der Hand | Unverändert weich wie während der Hand, scharf gesperrt | Beide Stapel | Beide Stapel | Beide Stapel |
| Erster Gegnerstich offen | Nur weich | Nein | Nein | Nein |
| Laufende Augenanzeigen | Eigene sichtbar, gegnerische als Lernhilfe | Aus | Aus | Aus |
| Legale Karten hervorheben | Option wie bisher | Gesperrt | Gesperrt | Gesperrt |
| Partie und Schneider | Sieben Punkte, Schneider-Doppelwertung wählbar | Wie Hausregel | Wie Hausregel | Sieben Punkte, Schneider-Doppelwertung fest aus |

- **Belegt:** TS beschreibt den weichen Tausch zu Beginn und bei zwei Talonkarten sowie das Sperrverbot bei zwei Karten. Pagat und Bummerl schränken den scharfen Tausch und die Ansage ein. TS, Pagat und schnapsen.at dokumentieren Varianten der Erfolgswertung beim Zudrehen. TS nennt einfache Bummerl im Turnier. Zwei Talonkarten bedeuten eine verdeckte und eine offene Karte. [TS](https://ts-spielkarten.de/spielregeln-kartenspiele/schnapsen-spielregeln/) · [Pagat DE](https://www.pagat.com/de/marriage/schnaps.html) · [Pagat EN](https://www.pagat.com/marriage/schnaps.html) · [schnapsen.at](https://schnapsen.at/spielregeln) · [Bummerl](https://www.bummerl.at/hilfe/)
- **Quellenkonflikte:** Pagat verlangt beim Zudrehen volle Hände und illustriert auch ein Zudrehen zu Beginn. schnapsen.at dokumentiert automatische Ausmeldung und eine andere Geberfolge. Die Beschreibung des Wiener Zudrehens dort sieht den Gegnergewinn anders als Pagat. TS und Pagat behandeln falsche Ausmeldungen unterschiedlich. Die Matrix nutzt das Mindeststrafmaß nach Pagat EN und lässt den Nichtzudreher ausmelden. [Pagat EN](https://www.pagat.com/marriage/schnaps.html) · [TS](https://ts-spielkarten.de/spielregeln-kartenspiele/schnapsen-spielregeln/) · [schnapsen.at](https://schnapsen.at/spielregeln)
- **Digitale Festlegung:** Beide klassischen und beide Turnierprofile erlauben Zudrehen zu Beginn. Scharfer Tausch ist ab dem ersten vollständig abgeschlossenen Stich möglich; in einer Zweierhand ist dessen Gewinner dann am Ausspiel. Falschmeldungen bei Sperre nutzen das Maximum beider Strafgrenzen. Ausmeldefenster sind ohne Kenntnis des Augenstands sichtbar, auch nach einer vorgemerkten Ansage und vor der Letztstichwertung. Die Sticheinsicht in weichen neuen Profilen ist während der Hand auf eigene Stiche beschränkt, nach der Hand werden beide Stapel freigegeben. Die sieben Punkte, Austeilfolge und der Geberwechsel bleiben die vorhandene Projektregel. Turnierverwaltung und Begegnungen über mehrere Bummerl bleiben künftiger Arbeit vorbehalten.
