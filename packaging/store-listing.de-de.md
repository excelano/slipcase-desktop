# Store listing text, German

The German half of `store-listing.md`, one file per language so that a diff
shows one language and a translator opens one file. The headings are that
file's headings and `fenster`'s parser reads both the same way.

**Three fields are written in German rather than translated, and that is not a
liberty.** German runs 15 to 35 per cent longer than the same English, and the
English short description is 480 of 500, the subtitle 27 of 30, and the
description 3,687 of 4,000. A faithful translation of any of them overflows a
limit the store enforces. So each says what the English says and is composed to
fit, which is why `fenster/check-listing.ps1` is run on this file rather than
trusted.

**The terminology is the application's own, out of `po/de.po`**, because a
listing that calls the thing something the window does not is worse than a
clumsy sentence: **Container**, never *Behälter*; **Nutzlast** for the payload;
**Entpacken**, **Ersetzen**, **Speichern**, **Öffnen** for the four verbs;
**Slipcase-Dateien** for what the file dialog filters on. `Slipcase` itself is
a product name and is never translated.

**What the German says less of, deliberately.** The description is the field\nwith no room: 3,687 English characters become 3,981 German ones against a limit\nof 4,000, so two clauses the English carries are not in it - that a rejected\npayload is refused when you choose it *rather than after you have said where\nthe container goes*, and the longer phrasing of the two verdicts that are\nneither pass nor failure. Both survive in the feature bullets and the release\nnotes. Anything added to the English description from here needs this file\nre-measured, not re-translated.

**The macOS-only paragraph is cut from the Microsoft Store listing**, the same
as in English and for the same reason: an executable bit is not what makes a
file executable on Windows, so on that store the sentence describes something
nobody will see.

## Subtitle (Mac App Store, 30)

Metadaten, die mitreisen

## Promotional text (Mac App Store, 170)

Container öffnen, die mitgereisten Metadaten lesen, sie an Ort und Stelle bearbeiten und die Datei dem Programm übergeben, das sie öffnet.

## Short description (Microsoft Store, 500)

Ein Slipcase-Container ist eine Datei, die ein Dokument samt seinen Metadaten enthält. Die Metadaten reisen mit dem Dokument, statt in einem Dateinamen, einer Begleitdatei oder fremden Datenbank zu stecken.

Slipcase erstellt Container und öffnet sie: die Nutzlast, womit Ihr System sie öffnen würde, und die Metadaten als bearbeitbaren Baum, mit Öffnen, Entpacken, Ersetzen und Speichern. Wo Ihr System nicht sagt, was eine Datei öffnet, sagt Slipcase nichts, statt zu raten.

## App features (Microsoft Store, up to 20 bullets of 200 characters)

    Die Metadaten reisen mit dem Dokument: eine Datei hält die Nutzlast und die Metadaten, die sie beschreiben.
    Erstellt einen Container aus einer Datei Ihrer Wahl und öffnet ihn, damit die Metadaten dort entstehen, wo sie bearbeitet werden. Große Nutzlasten packen mit Fortschrittsbalken und Anhalten.
    Metadaten an Ort und Stelle bearbeiten und speichern. Kommentare, Schlüsselreihenfolge und Leerraum, die Sie nicht angefasst haben, überleben das Neuschreiben.
    Übergibt die Nutzlast dem Programm, das Ihr System für diesen Dateityp vorgesehen hat. Keine Vorschau, kein Raten von Typen.
    Sagt, wenn ein Container von anderswo kam, und markiert die entpackte Nutzlast, damit Ihr System sie ebenso vorsichtig behandelt.
    Nennt das Urteil des Containers gegenüber der Spezifikation in deren eigenen Worten, einschließlich unbestimmt und außerhalb des Geltungsbereichs.
    Neu Geschriebenes wird zurückgelesen und geprüft, bevor es etwas ersetzt, und ein unveränderter Container wird gar nicht neu geschrieben.
    Keinerlei Netzwerkverbindung. Kein Konto, keine Telemetrie, keine Analyse, nichts wird irgendwohin gesendet.
    Open Source, und ebenso das Format, das es liest, und die Bibliothek, die es liest.

## Description (both, written to 4,000)

Ein Slipcase-Container ist eine Datei, die ein Dokument beliebigen Typs zusammen mit den Metadaten enthält, die es beschreiben. Kopieren, senden oder verschieben Sie den Container — die Metadaten gehen mit. Statt in einem Dateinamen zu stecken, der abgeschnitten wird, in einer Begleitdatei, die verlorengeht, oder in der Datenbank eines anderen.

Slipcase öffnet einen Container und zeigt, was darin ist, und erstellt einen aus einer Datei Ihrer Wahl.

WAS SIE SEHEN

Name und Größe der Nutzlast. Womit Ihr System sie öffnen würde. Die Metadaten als Baum, jeder Wert an Ort und Stelle bearbeitbar. Und das Urteil des Containers gegenüber der Spezifikation, in deren eigenen Worten — einschließlich der beiden Antworten, die weder Bestehen noch Scheitern sind: unbestimmt, wenn die Metadaten nicht gelesen werden können, und außerhalb des Geltungsbereichs, wenn er für eine neuere Fassung des Formats geschrieben wurde.

WAS SIE TUN KÖNNEN

Öffnen übergibt die Nutzlast dem Programm, das für diesen Dateityp vorgesehen ist. Entpacken schreibt sie, wohin Sie wollen. Ersetzen tauscht sie gegen eine andere Datei. Speichern schreibt bearbeitete Metadaten zurück — und behält dabei Ihre Kommentare, Ihre Schlüsselreihenfolge und jeden Leerraum, den Sie nicht angefasst haben, samt allem anderen im Container, das Slipcase nicht kennt.

Neu Geschriebenes wird zurückgelesen und geprüft, bevor es etwas ersetzt: ein Speichern, das einen Container ergäbe, den das Format nicht annimmt, ändert nichts auf der Platte. Ein Container, den Sie nicht geändert haben, wird gar nicht erst neu geschrieben.

Neuer Container fragt, welche Datei hinein soll und wohin der Container gehört, schreibt ihn und öffnet ihn dann, damit die Metadaten dort entstehen, wo sie auch bearbeitet werden. Eine große Nutzlast packt mit Fortschrittsbalken und Anhalten, und Anhalten lässt nichts zurück. Eine Datei, die das Format nicht als Nutzlast annimmt, wird abgelehnt, sobald Sie sie wählen.

WAS ES IHNEN SAGT UND NICHT FÜR SIE ENTSCHEIDET

Slipcase berichtet. Es hält nichts auf.

Kam ein Container von anderswo — heruntergeladen oder Ihnen geschickt —, sagt Slipcase das, und auch die entpackte Nutzlast wird markiert, damit Ihr System sie mit der Vorsicht behandelt, die es allem von außen entgegenbringt, statt sie zu öffnen, als wäre sie Ihre eigene. Metadaten zu bearbeiten und zu speichern löscht das nicht still: Slipcase sagt Ihnen danach weiterhin, woher der Container kam. Es gilt auch andersherum: eine Datei von anderswo sagt das weiterhin auf dem Container, in den Sie sie packen.

Unter macOS sagt es Ihnen, wenn eine Nutzlast als ausführbare Datei abgelegt wurde und die entpackte Kopie es nicht sein wird. Das wird aus dem Container gelesen und nicht aus dem Namen geraten.

Namen werden mit ausgeschriebenen Zeichen gezeigt, die Text umordnen, damit sich eine Nutzlast nicht als ein Dateityp ausgeben kann, während sie ein anderer ist.

Wo Ihr System nicht sagt, was eine Nutzlast öffnet, sagt Slipcase nichts, statt zu raten. Es bringt keine Tabelle mit, die Dateinamen auf Typen abbildet, und sieht nie in eine Nutzlast hinein, um einen zu erraten. Was Sie bekommen, sind Auskünfte — woher die Datei kam, was sie war, ob der Container wohlgeformt ist — und die Entscheidung bleibt Ihre.

WAS ES NICHT TUT

Keinerlei Netzwerkverbindung. Kein Konto. Keine Telemetrie, keine Analyse, keine Absturzberichte. Nichts über Sie oder Ihre Dateien wird irgendwohin gesendet, weil es nirgendwohin zu senden gibt.

Es zeigt keine Vorschau: die Nutzlast wird einem anderen Programm übergeben, statt hier dargestellt zu werden. Ein Container hält eine Nutzlast, was das Format entscheidet und nicht dieses Programm.

OPEN SOURCE

Slipcase ist Open Source, und ebenso das Format, das es liest, und die Bibliothek, die es liest. Jede Aussage oben ist überprüfbar: github.com/excelano/slipcase-desktop.

## Release notes

*Neu in dieser Version*, aus `CHANGELOG.md`, neueste zuerst. Der Store liefert
0.1.2 aus, deshalb steht hier alles seither; 0.1.3 und 0.1.4 fehlen, weil das
eine eine Mac-Einreichung und das andere Linux-Paketierung betraf.

### 0.1.6

Slipcase erstellt jetzt Container. *Neuer Container …* fragt, welche Datei hinein soll und wohin der Container gehört, schreibt ihn und öffnet ihn, damit die Metadaten dort entstehen, wo sie auch bearbeitet werden. Eine große Nutzlast packt mit Fortschrittsbalken und Anhalten, und Anhalten lässt nichts zurück. Eine Datei, die das Format nicht als Nutzlast annimmt, wird abgelehnt, sobald Sie sie wählen.

Der Metadaten-Editor ist der allgemeine, den sich Slipcase mit Tommy Flyleaf teilt. Jeder Wert hat ein Menü der Umwandlungen, die er zulässt, Felder sind bearbeitbar, Kommentare lassen sich ändern, hinzufügen und entfernen, und Rückgängig und Wiederholen reichen durch alles hindurch. Ein Speichern behält weiterhin Kommentare, Schlüsselreihenfolge, Leerraum und Anführungszeichen von allem, was Sie nicht angefasst haben.

Die Schaltflächen und der Dateidialog sagen "Container öffnen", nun da das Format Slipcase geschrieben wird.

## Keywords

**Mac App Store** (100 characters, comma-separated, no spaces after commas):

    Metadaten,Container,slpc,Archiv,TOML,Datei,Dokument,Verschlagwortung,zip

**Microsoft Store** (seven terms):

    Metadaten, Container, slpc, TOML, Archiv, Dokument, Dateibetrachter

Author: David M. Anderson
Built with AI assistance (Claude, Anthropic)
