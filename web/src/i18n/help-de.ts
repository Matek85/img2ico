// German help texts: the same keys and marks as help-en.ts, with the names of the buttons as they read in de.ts.
import type { helpEn } from './help-en.ts';

export const helpDe: Record<keyof typeof helpEn, string> = {
  // ---- Erste Schritte
  'help.start.01.h2': 'Ein Bild öffnen',
  'help.start.02.p':
    'Lege ein Bild auf der Seite ab, klicke auf **Datei auswählen** oder füge eines aus der Zwischenablage ein (zum Beispiel einen Screenshot) mit [[Strg]] + [[V]] ([[Cmd]] + [[V]] auf dem Mac). img2ico liest PNG-, JPG-, GIF-, WebP-, BMP-, TIFF-, SVG- und ICNS-Bilder; [Dateitypen](help:file-types) sagt, wofür sich welches eignet.',
  'help.start.03.p':
    'Mehrere Bilder oder eine ZIP-Datei voller Bilder landen auf einmal in einer Warteschlange (siehe unten). Eine .ico-Datei, die du ablegst, wird stattdessen zum Hineinschauen geöffnet: Du kannst sie prüfen, Bilder daraus entnehmen oder ihr größtes Bild als Bild bearbeiten.',
  'help.start.04.h2': 'Wähle, wofür das Icon gedacht ist',
  'help.start.05.p':
    'Die Einstellungen neben der Vorschau beginnen mit dem **Schnellstart**. Unter **Wofür ist es?** wählst du einen Satz Größen: Windows-App, Standard, Website-Icon, macOS-App oder Schnelltest. Unter **Stil** wählst du, wie das Bild in seinem Quadrat sitzt: Schlicht, Abgerundetes App-Icon oder Rund.',
  'help.start.06.p':
    'Die Vorschau ändert sich sofort. Die Einstellungen klappen zu einer schmalen Leiste aus Symbolen zusammen, damit die Vorschau Platz hat; der Pfeil an ihrem oberen Ende öffnet sie, und der **Erweiterte Editor** enthält jede Einstellung ([die Einstellungen erklärt](help:settings)).',
  'help.start.07.h2': 'Das Ergebnis prüfen',
  'help.start.08.p':
    'Die Vorschau zeigt jede Größe des Icons in ihrer echten Größe. Die Spalte aus Quadraten links davon ändert den Hintergrund hinter dem Icon: kariert, hell, dunkel, grau oder eine eigene Farbe (zeige auf eines, um es auszuprobieren, klicke, um es zu behalten). **Icon** zeigt die Icons, **Vorher und nachher** legt das Original neben das Ergebnis, mit einer Trennlinie, die du ziehen kannst, und **Pixel** zeigt das Icon Pixel für Pixel mit der genauen Farbe jedes einzelnen.',
  'help.start.09.p':
    'Hinweise unter der Vorschau sagen dir, wenn etwas wahrscheinlich schlecht aussieht, zum Beispiel wenn das Bild kleiner ist als eine Größe, die du gewählt hast.',
  'help.start.10.h2': 'Das Bild selbst ändern',
  'help.start.11.p':
    '**Bearbeiten** über der Vorschau öffnet die Bildwerkzeuge: nur einen Teil des Bildes verwenden, es drehen und sehen, wie ein runder Stil es zuschneidet. Sie sind in [Die Einstellungen erklärt](help:settings) beschrieben.',
  'help.start.12.h2': 'Herunterladen',
  'help.start.13.p':
    'Wähle über der Vorschau den Dateityp: **Windows .ico**, **macOS .icns** oder **Website-ZIP**, und drücke dann auf den Download-Knopf. Die Leiste bleibt beim Scrollen im Blick. Der Archiv-Knopf daneben speichert jede Größe als PNG-Datei, in einer ZIP-Datei.',
  'help.start.14.h2': 'Mehrere Icons machen',
  'help.start.15.p':
    '**Zur Warteschlange hinzufügen** behält das Icon und lässt dich das nächste Bild wählen. Die Warteschlange steht neben dem Editor: Ändere die Reihenfolge (ziehe eine Zeile am Griff oder nimm die Pfeilknöpfe), öffne jedes Icon, um es erneut zu ändern (die Änderungen werden von selbst darin gespeichert), vergleiche zwei Icons und lade alle als eine ZIP-Datei oder einzeln herunter. Während du ein Icon der Warteschlange bearbeitest, macht **Für alle Icons verwenden** die anderen Icons noch einmal mit den Größen, dem Aussehen oder dem Entfernen des Hintergrunds dieses Icons. Die Warteschlange gibt es nur, solange die Seite offen ist.',
  'help.start.16.h2': 'Deine Einstellungen werden gemerkt',
  'help.start.17.p':
    'Die Seite merkt sich deine letzten Einstellungen in diesem Browser, damit das nächste Bild so beginnt, wie du aufgehört hast. **Auf Standard zurücksetzen**, oben rechts im Editor, setzt alles zurück (es fragt vorher nach). Zuschnitt und Drehung gehören zu einem Bild und werden nicht gemerkt. [Datenschutz](help:privacy) sagt, was der Browser speichert.',
  'help.start.18.h2': 'Sprache und heller oder dunkler Modus',
  'help.start.19.p':
    'Das Menü **Sprache** in der oberen Leiste zeigt die Seite auf Englisch, Deutsch, Spanisch, brasilianischem Portugiesisch oder Französisch. Beim ersten Besuch wählt die Seite die Sprache deines Browsers; deine Wahl wird gemerkt. Manche Übersetzungen hat eine KI gemacht, und die Seite sagt es oben. Der halb gefüllte Kreis daneben wechselt zwischen hellem und dunklem Modus; bis du ihn benutzt, folgt die Seite deinem System.',

  // ---- Die Einstellungen erklärt
  'help.settings.01.h2': 'Schnellstart',
  'help.settings.02.h3': 'Wofür ist es?',
  'help.settings.03.dl.1.t': 'Windows-App',
  'help.settings.03.dl.1.d':
    'Die zehn Größen, die Microsoft empfiehlt (16, 20, 24, 32, 40, 48, 64, 96, 128 und 256 Pixel), damit Windows nie ein Icon dehnen muss. Eine .ico-Datei.',
  'help.settings.03.dl.2.t': 'Standard',
  'help.settings.03.dl.2.d': 'Sechs Größen von 16 bis 256 Pixeln: was die meisten Programme mitliefern. Eine .ico-Datei.',
  'help.settings.03.dl.3.t': 'Website-Icon',
  'help.settings.03.dl.3.d':
    'Alles, was eine Website braucht, in einer ZIP-Datei: favicon.ico, PNG-Icons, ein Apple-Icon und ein Web-Manifest ([Dateitypen](help:file-types) listet sie auf).',
  'help.settings.03.dl.4.t': 'macOS-App',
  'help.settings.03.dl.4.d':
    'Eine .icns-Datei für macOS. Sie hat ihre eigenen Größen (16 bis 1024 Pixel), deshalb bestimmen die Größen, die du wählst, nur, was die Vorschau zeigt.',
  'help.settings.03.dl.5.t': 'Schnelltest',
  'help.settings.03.dl.5.d': 'Nur 16 und 32 Pixel: eine kleine Datei zum Ausprobieren.',
  'help.settings.04.h3': 'Stil',
  'help.settings.05.dl.1.t': 'Schlicht',
  'help.settings.05.dl.1.d': 'Das Bild, wie es ist, ohne Rand und ohne Abrundung.',
  'help.settings.05.dl.2.t': 'Abgerundetes App-Icon',
  'help.settings.05.dl.2.d': 'Ein kleiner Rand und abgerundete Ecken, wie bei einem App-Icon.',
  'help.settings.05.dl.3.t': 'Rund',
  'help.settings.05.dl.3.d': 'Füllt einen Kreis: Die Mitte des Bildes bleibt, der Rest wird weggeschnitten.',

  'help.settings.06.h2': 'Größen',
  'help.settings.07.p':
    'Eine .ico-Datei enthält mehrere Bilder, eines für jede Größe, in Pixeln pro Seite. Windows nimmt für die Taskleiste, Titelleisten, Listen und große Vorschauen die nächstgelegene Größe, die es findet; mehr Größen bedeuten also weniger gedehnte Bilder. Im Erweiterten Editor kannst du jede der Größen 16, 20, 24, 32, 40, 48, 64, 96, 128 und 256 Pixel anhaken. Ist das Bild kleiner als eine Größe, die du gewählt hast, sagt ein Hinweis es dir: Diese Größe wird vergrößert und sieht weich aus.',

  'help.settings.08.h2': 'Aussehen',
  'help.settings.09.dl.1.t': 'Rand',
  'help.settings.09.dl.1.d':
    'Leerer Raum um das Bild, als Anteil des Icons (0 bis 40 %). Ein Icon, das sein Quadrat bis zum Rand füllt, wirkt neben anderen gedrängt.',
  'help.settings.09.dl.2.t': 'Abgerundete Ecken',
  'help.settings.09.dl.2.d': 'Wie rund die Ecken sind, als Anteil der kürzeren Seite (0 bis 50 %). 50 % ergibt einen Kreis.',
  'help.settings.09.dl.3.t': 'Anpassung',
  'help.settings.09.dl.3.d':
    '**Das ganze Bild zeigen** behält alles und lässt transparente Streifen, wo das Bild nicht quadratisch ist. **Das Quadrat füllen** schneidet ab, was übersteht, damit das Quadrat voll ist.',
  'help.settings.09.dl.4.t': 'Schwarzweiß',
  'help.settings.09.dl.4.d': 'Nimmt alle Farbe weg und behält die Helligkeit.',
  'help.settings.09.dl.5.t': 'Leeren Rand abschneiden',
  'help.settings.09.dl.5.d':
    'Schneidet den transparenten Rand um das Motiv ab, damit es das Icon ausfüllt (nach dem Zuschnitt, falls es einen gibt). Nützlich nach dem Entfernen eines Hintergrunds.',

  'help.settings.10.h2': 'Hintergrund entfernen',
  'help.settings.11.p':
    'Hake **Hintergrund entfernen** an, um einen einfarbigen Hintergrund transparent zu machen. Standardmäßig wird die Farbe am Rand des Bildes erkannt; du kannst die Farbe auch selbst wählen oder aus dem Bild aufnehmen: Klicke in der Ansicht **Pixel** auf ein Pixel, dann öffnen sich die Einstellungen und blinken dort, wo die Farbe hingekommen ist.',
  'help.settings.12.dl.1.t': 'Toleranz',
  'help.settings.12.dl.1.d':
    'Wie stark eine Farbe von der Hintergrundfarbe abweichen darf und trotzdem als Hintergrund zählt (0 bis 100). Ein höherer Wert nimmt mehr weg, kann aber ins Motiv schneiden.',
  'help.settings.12.dl.2.t': 'Weiche Kante',
  'help.settings.12.dl.2.d': 'Wie weich die Kante dessen ist, was entfernt wird (0 bis 100). 0 ist eine harte Kante.',
  'help.settings.13.p':
    'Das Entfernen beginnt am Rand des Bildes und arbeitet sich nach innen, deshalb bleibt eine Farbe, die im Motiv vorkommt, erhalten, solange sie den entfernten Bereich nicht berührt.',

  'help.settings.14.h2': 'Das Bild bearbeiten',
  'help.settings.15.p':
    '**Bearbeiten** (Taste [[C]]) zeigt das Bild mit einem Rahmen, und nur was im Rahmen liegt, wird verwendet. Drücke **Fertig** oder [[Esc]], wenn du fertig bist; **Das ganze Bild verwenden** legt den Rahmen wieder um alles.',
  'help.settings.16.dl.1.t': 'Das Schloss',
  'help.settings.16.dl.1.d':
    'Bei geschlossenem Schloss (am Anfang) bleibt der Rahmen in der Mitte der Vorschau, und du verschiebst das Bild darunter: Ziehe es, zoome mit dem Mausrad oder nimm die Pfeiltasten mit [[+]] und [[-]]. Öffne das Schloss ([[Leertaste]]), um stattdessen den Rahmen und seine Griffe zu ziehen.',
  'help.settings.16.dl.2.t': 'Form',
  'help.settings.16.dl.2.d': 'Frei, 1:1, 4:3, 3:2 oder 16:9. Eine feste Form behält ihr Seitenverhältnis, während du arbeitest.',
  'help.settings.16.dl.3.t': 'Rahmen platzieren',
  'help.settings.16.dl.3.d':
    'Das 3 × 3-Feld setzt den Rahmen an einen Rand, in eine Ecke oder in die Mitte des Bildes und zeigt, wo er ist. Der Ziffernblock tut dasselbe.',
  'help.settings.16.dl.4.t': 'Größter Rahmen, Rahmen drehen',
  'help.settings.16.dl.4.d':
    '**Größter Rahmen** macht den Rahmen so groß, wie das Bild es erlaubt, in derselben Form. **Rahmen drehen** tauscht breit und hoch.',
  'help.settings.16.dl.5.t': 'Das Bild drehen und spiegeln',
  'help.settings.16.dl.5.d':
    'Solange das Schloss geschlossen ist, dreht ein Regler das Bild von −180° bis 180° in Schritten von 1° (ein positiver Winkel dreht im Uhrzeigersinn), und Knöpfe drehen es um 90° oder 180°. Das Bild dreht sich unter dem Rahmen, und die Ecken, die die Drehung freilässt, sind transparent. Zwei Knöpfe spiegeln das Bild, von links nach rechts und von oben nach unten; es wird zuerst gespiegelt und dann gedreht.',
  'help.settings.16.dl.6.t': 'Links, Oben, Breite, Höhe',
  'help.settings.16.dl.6.d': 'Der Rahmen in Pixeln des Bildes (nach der Drehung). Gib genaue Zahlen ein, wenn du sie brauchst.',
  'help.settings.16.dl.7.t': 'Form zeigen',
  'help.settings.16.dl.7.d':
    'Beim Stil Abgerundetes App-Icon oder Rund werden die Ecken, die der Stil wegschneidet, im Rahmen abgedunkelt, damit du siehst, wie das Icon aussieht. Du kannst es ausschalten.',
  'help.settings.17.p':
    'Zuschnitt und Drehung gehören zu dem Bild, das du gerade vor dir hast; sie werden nicht für das nächste gemerkt. Jede Taste dieser Werkzeuge steht in [Tastenkürzel](help:keyboard-shortcuts).',

  'help.settings.18.h2': 'Animierte GIFs',
  'help.settings.19.p':
    'Ein animiertes GIF zeigt über der Vorschau einen kleinen Player: abspielen, pausieren, durch die Einzelbilder gehen und bei dem anhalten, das du willst. Das Icon wird aus dem Einzelbild gemacht, bei dem du anhältst.',

  'help.settings.20.h2': 'Das Website-Paket',
  'help.settings.21.p':
    'Bei **Website-ZIP** öffnet das Zahnrad die Einstellungen des Pakets: den Namen der Website (er wird angezeigt, wenn die Website zu einem Startbildschirm hinzugefügt wird), die Designfarbe und den Hintergrund des Apple-Icons (ein iPhone füllt transparente Bereiche mit Schwarz, deshalb wird das Apple-Icon auf diese Farbe gelegt).',

  'help.settings.22.h2': 'Die Einstellungsspalte',
  'help.settings.23.p':
    'Die Spalte klappt zu einer Leiste aus Symbolen zusammen, damit die Vorschau breit sein kann; der Pfeil an ihrem oberen Ende öffnet sie. Wenn du die Bildwerkzeuge startest, klappt sie von selbst zu und öffnet sich wieder, wenn du fertig bist. **Auf Standard zurücksetzen** fragt vorher nach und setzt dann Größen, Aussehen, Hintergrund, Zuschnitt und Drehung auf den Anfang zurück.',
  'help.settings.24.h3': 'Einstellungen als Datei',
  'help.settings.25.p':
    '**Einstellungen exportieren**, neben „Auf Standard zurücksetzen“, speichert Größen, Rand, Ecken, Anpassung, Schwarzweiß, das Entfernen des Hintergrunds und den Dateityp als kleine Textdatei namens `img2ico.toml`. Das ist dieselbe Art Datei, die die Kommandozeile liest, der Export ist also mit ihr kompatibel: `img2ico --config img2ico.toml logo.png`, oder lege die Datei in deinen Ordner, dann findet die Kommandozeile sie von selbst. Umgekehrt übernimmt **Einstellungen importieren** eine Datei, die hier gespeichert oder für die Kommandozeile geschrieben wurde, zeigt die Einstellungen und sagt, was nicht übernommen wurde (die Optionen der Kommandozeile, die hier keinen Zweck haben). Eine Zeile, die die Seite nicht kennt, oder ein Wert, der nicht erlaubt ist, hält den Import mit einer Liste an, und es ändert sich nichts. Das Spiegeln, die Drehung, der Zuschnitt und das Einzelbild einer Animation stehen in der Datei, wenn du sie benutzt hast. Eine hier gespeicherte Datei ergibt beim erneuten Import dasselbe Icon, und auch auf der Kommandozeile (die `--flip-horizontal`, `--flip-vertical`, `--rotate`, `--crop` und `--gif-frame` kennt). Ein Zuschnitt, der nicht zum Bild passt, in das du importierst, wird weggelassen, und ein Hinweis sagt es.',
  'help.settings.26.h3': 'Rückgängig und wiederholen',
  'help.settings.27.p':
    'Die beiden Pfeile oben rechts im Editor, **Strg+Z** (Cmd+Z auf dem Mac) und **Strg+Umschalt+Z** oder **Strg+Y** nehmen die letzten Änderungen der Einstellungen zurück und bringen sie wieder, den Zuschnittsrahmen, die Drehung und das Spiegeln eingeschlossen. Eine Reihe kleiner Änderungen, etwa beim Ziehen eines Reglers, zählt als eine. **Auf Standard zurücksetzen** und **Einstellungen importieren** lassen sich ebenfalls rückgängig machen. Der Verlauf gehört zum Bild vor dir und endet, wenn du es verlässt. Während du in einem Feld tippst, macht Strg+Z dort das Tippen rückgängig.',

  // ---- Dateitypen
  'help.types.01.h2': 'Bilder, die du öffnen kannst',
  'help.types.02.dl.1.t': 'PNG',
  'help.types.02.dl.1.d': 'Die beste Wahl: scharf und kann transparent sein.',
  'help.types.02.dl.2.t': 'JPG',
  'help.types.02.dl.2.d': 'Fotos. Ein JPG hat keine Transparenz, deshalb bleibt sein Hintergrund, solange du ihn nicht entfernst.',
  'help.types.02.dl.3.t': 'GIF',
  'help.types.02.dl.3.d': 'Stehend oder animiert. Bei einem animierten wählst du das Einzelbild, aus dem das Icon gemacht wird.',
  'help.types.02.dl.4.t': 'WebP, BMP, TIFF',
  'help.types.02.dl.4.d': 'Werden wie jedes andere Bild gelesen.',
  'help.types.02.dl.5.t': 'SVG',
  'help.types.02.dl.5.d':
    'Eine Zeichnung: Sie wird bei jeder Größe neu gezeichnet und ist deshalb bei allen scharf. Ein Zuschnitt und eine Drehung gelten für sie nicht. Text in einem SVG wird nicht gezeichnet, weil der Seite die Schriften dafür fehlen (sie warnt dich, wenn ein SVG Text enthält): Wandle Text zuerst in Pfade um.',
  'help.types.02.dl.6.t': 'ICNS',
  'help.types.02.dl.6.d': 'Ein vorhandenes macOS-Icon lässt sich als Bild öffnen.',
  'help.types.03.p':
    'Ein quadratisches Bild mit transparentem Hintergrund funktioniert am besten. Ein Bild mit mehr als 40 Megapixeln (zum Beispiel 8000 × 5000 Pixel) wird abgelehnt; das Kommandozeilen-Werkzeug nimmt größere ([Was die Seite nicht kann](help:command-line)).',

  'help.types.04.h2': 'Dateien, die du machen kannst',
  'help.types.05.dl.1.t': '.ico (Windows)',
  'help.types.05.dl.1.d': 'Eine Datei mit allen gewählten Größen, von 1 bis 256 Pixeln. Windows, Browser und viele Programme verwenden sie.',
  'help.types.05.dl.2.t': '.icns (macOS)',
  'help.types.05.dl.2.d': 'Das macOS-Icon-Format. Es hat feste Größen: 16, 32, 64, 128, 256, 512 und 1024 Pixel.',
  'help.types.05.dl.3.t': 'Website-ZIP',
  'help.types.05.dl.3.d':
    'Ein Paket für eine Webseite: favicon.ico (16, 32 und 48 Pixel), favicon-16x16.png, favicon-32x32.png, apple-touch-icon.png (180 Pixel), icon-192.png, icon-512.png, site.webmanifest und head-snippet.html mit den Zeilen zum Einfügen in deine Seite. Ein Paket aus einem SVG enthält zusätzlich favicon.svg.',
  'help.types.05.dl.4.t': 'PNG-ZIP',
  'help.types.05.dl.4.d': 'Jede Größe eines Icons als eigene PNG-Datei, in einer ZIP-Datei (der Archiv-Knopf neben dem Download-Knopf).',
  'help.types.05.dl.5.t': 'Warteschlangen-ZIP',
  'help.types.05.dl.5.d': 'Alle Icons der Warteschlange in einer ZIP-Datei.',

  'help.types.06.h2': 'Eine .ico-Datei öffnen',
  'help.types.07.p':
    'Lege statt eines Bildes eine .ico-Datei ab, um hineinzuschauen: jedes Bild mit Größe, Farbtiefe und Speicherformat sowie Hinweise auf Probleme, zum Beispiel eine Größe, die Windows gern hätte und die fehlt. Du kannst einzelne Bilder entnehmen, nur einige behalten oder das größte Bild als Bild bearbeiten, um neue Größen zu machen.',

  'help.types.08.h2': 'Viele Dateien auf einmal',
  'help.types.09.p':
    'Lege mehrere Bilder oder eine ZIP-Datei ab, dann landen sie in der Warteschlange (bis zu 500 Dateien auf einmal): Bilder werden mit deinen aktuellen Einstellungen zu Icons, und .ico-Dateien bleiben, wie sie sind. Die Warteschlange lädst du als eine ZIP-Datei herunter.',

  // ---- Datenschutz
  'help.privacy.01.h2': 'Deine Bilder bleiben auf deinem Computer',
  'help.privacy.02.p':
    'img2ico arbeitet vollständig in deinem Browser. Ein Programm (WebAssembly), das innerhalb der Seite auf deinem eigenen Computer läuft, macht die Icons. Deine Bilder und die daraus gemachten Icons werden nie hochgeladen, und die Seite hat keinen Server, der sie empfangen könnte.',
  'help.privacy.03.h2': 'Was die Seite sich merkt',
  'help.privacy.04.p': 'Um dir Arbeit zu sparen, behält die Seite einige kleine Einstellungen in deinem Browser (lokaler Speicher), nur auf deinem Computer:',
  'help.privacy.05.ul.1': 'deine letzten Einstellungen: Größen, Rand, Ecken, Anpassung, Schwarzweiß, Hintergrund entfernen, der Dateityp sowie Name und Farben des Website-Pakets (nicht das Bild und nicht Zuschnitt oder Drehung)',
  'help.privacy.05.ul.2': 'ob die Warteschlange Änderungen von selbst speichert',
  'help.privacy.05.ul.3': 'ob die Einstellungen eingeklappt sind',
  'help.privacy.05.ul.4': 'heller oder dunkler Modus, falls du einen gewählt hast',
  'help.privacy.05.ul.5': 'ob die Tastenkürzel an sind',
  'help.privacy.05.ul.6': 'die Sprache, die du gewählt hast',
  'help.privacy.06.p':
    'Die Warteschlange und deine Bilder gibt es nur im Speicher der Seite, solange sie offen ist: Neu laden oder Schließen der Seite leert sie.',
  'help.privacy.07.h2': 'Was die Seite nicht tut',
  'help.privacy.08.ul.1': 'Kein Konto und keine Anmeldung.',
  'help.privacy.08.ul.2': 'Keine Werbung, kein Tracking und keine Analyse.',
  'help.privacy.08.ul.3': 'Keine Cookies.',
  'help.privacy.08.ul.4': 'Es werden weder Bilder noch Dateinamen irgendwohin gesendet.',
  'help.privacy.09.h2': 'Wer die Seite betreibt',
  'help.privacy.10.p':
    'Wie bei jeder Website kann der Server, der die Seite ausliefert, sehen, dass die Seite abgerufen wurde (die Adresse, die Zeit und der Name deines Browsers). Deine Bilder sieht er nie.',
  'help.privacy.11.h2': 'Prüfe es selbst',
  'help.privacy.12.p':
    'Öffne die Entwicklerwerkzeuge deines Browsers, schau dir den Reiter Netzwerk an und wandle ein Bild um: Es wird nichts gesendet. Um zu entfernen, was die Seite sich merkt, lösche die Daten dieser Website in den Einstellungen deines Browsers. Der Quellcode ist offen: [img2ico auf GitHub](https://github.com/Matek85/img2ico).',

  // ---- Tastenkürzel
  'help.shortcuts.01.p':
    'Einzelne Tasten ohne Strg, Alt oder Cmd, damit sie den Tastenkürzeln des Browsers nie in die Quere kommen. Sie funktionieren, solange kein Text eingegeben wird, und nicht bei offenem Dialog.',
  'help.shortcuts.02.keys': 'Alle Tastenkürzel des Editors',
  'help.shortcuts.03.h2': 'Gut zu wissen',
  'help.shortcuts.04.ul.1':
    'Buchstaben richten sich nach der Tastatur, auf der du tippst. Die Ziffern 1 bis 3 nutzen die Reihe über den Buchstaben, deshalb funktionieren sie auch auf einer französischen Tastatur.',
  'help.shortcuts.04.ul.2':
    'Beim Bearbeiten des Bildes setzt der Ziffernblock den Rahmen so, wie das Feld auf dem Bildschirm angeordnet ist (7 oben links, 5 Mitte, 3 unten rechts), egal was NumLock macht.',
  'help.shortcuts.04.ul.3':
    'Drücke im Editor [[?]] für diese Liste. Dort kannst du die Tastenkürzel auch ausschalten; die Wahl wird gemerkt.',

  // ---- Was die Seite nicht kann
  'help.cli.01.p':
    'Die Seite und das Kommandozeilen-Werkzeug teilen sich eine Engine, deshalb erzeugen beide dieselben Icon-Dateien. Das Kommandozeilen-Werkzeug ist für das, was eine Seite nicht kann: viele Dateien und Ordner bearbeiten, ohne einen Menschen laufen und Protokolle führen.',
  'help.cli.02.h2': 'Was nur die Kommandozeile kann',
  'help.cli.03.dl.1.t': 'Ordner',
  'help.cli.03.dl.1.d':
    'Einen ganzen Ordner umwandeln, mit seinen Unterordnern (`--recursive`), nur die passenden Dateien (`--include`, `--exclude`), mit erhaltener Ordnerstruktur (`--keep-structure`) und die Icons nach einem Muster benannt (`--name`).',
  'help.cli.03.dl.2.t': 'Erst schauen, dann tun',
  'help.cli.03.dl.2.d': '`--what-if` zeigt, was passieren würde, und schreibt nichts.',
  'help.cli.03.dl.3.t': 'Berichte',
  'help.cli.03.dl.3.d':
    '`--report` schreibt ein Protokoll eines Laufs als CSV oder JSON, mit einer Zeile pro Datei und den Summen. `--json` gibt die Berichte von `--inspect` und `--validate` als JSON aus.',
  'help.cli.03.dl.4.t': 'Einstellungsdateien',
  'help.cli.03.dl.4.d':
    'Halte deine Einstellungen in einer TOML-Datei (`--config`, `--out-toml`) und nutze bei jedem Lauf auf jedem Computer dieselben. Die Webseite speichert und liest dieselbe Datei ([die Einstellungen erklärt](help:settings)).',
  'help.cli.03.dl.5.t': 'Prüfungen in der Automatisierung',
  'help.cli.03.dl.5.d':
    '`--validate` prüft den Aufbau von .ico-Dateien oder von Ordnern damit und endet mit einem Fehlercode, wenn eine ungültig ist: gemacht für Continuous Integration.',
  'help.cli.03.dl.6.t': 'Vorsichtige Stapel',
  'help.cli.03.dl.6.d':
    '`--skip-existing` lässt fertige Dateien in Ruhe, `--keep-going` macht nach einem Fehler weiter, `--force` überschreibt, und `--delete-source` entfernt die Originale nach einem Erfolg.',
  'help.cli.03.dl.7.t': 'Schwierigere Hintergründe',
  'help.cli.03.dl.7.d':
    '`--seed` fügt Startpunkte für das Entfernen des Hintergrunds hinzu, und `--find` entdeckt eingeschlossene Bereiche der Farbe, die der Rand nicht erreichen kann.',
  'help.cli.03.dl.8.t': 'Große Bilder und Tempo',
  'help.cli.03.dl.8.d': 'Keine Grenze von 40 Megapixeln (`--max-pixels`, anfangs 100 Millionen) und Arbeit auf mehreren Threads (`--jobs`).',
  'help.cli.04.h2': 'So bekommst du es',
  'help.cli.05.p':
    'Das Menü **CLI herunterladen** in der oberen Leiste enthält die neueste Version für Windows, macOS und Linux. Jede Option steht in [der Befehlsreferenz auf GitHub](https://github.com/Matek85/img2ico#command-reference).',
};
