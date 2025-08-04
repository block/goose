# Code Artifacts für Goose

## Übersicht

Code Artifacts ist eine neue Funktionalität in Goose (Goose Desktop), die es ermöglicht, generierten Code zu speichern, zu verwalten und mit Live-Preview zu betrachten. Wenn der AI Code generiert (z.B. einen Pomodoro Timer in HTML/JavaScript), kann dieser als "Artifact" gespeichert und später bearbeitet werden.

## Features

### 🎯 Hauptfunktionen
- **Live-Preview**: HTML, CSS, JavaScript und React-Komponenten können direkt in der Anwendung ausgeführt werden
- **Code-Bearbeitung**: Artifacts können inline bearbeitet und gespeichert werden
- **Artifact-Verwaltung**: Übersichtliche Liste aller gespeicherten Code-Artifacts
- **Export/Import**: Artifacts können als Dateien exportiert oder importiert werden
- **Suche & Filter**: Durchsuche und filtere Artifacts nach Sprache, Datum oder Namen

### 🔧 Unterstützte Sprachen
- **HTML** - Vollständige Live-Preview
- **CSS** - Angewendet auf Beispiel-Inhalte
- **JavaScript** - Ausführung mit Console-Output
- **JSX/TSX** - React-Komponenten mit Babel-Transformation
- **TypeScript** - Als Datei exportierbar
- **Python, Java, C++, C#** - Als Datei exportierbar

## Verwendung

### 1. Code als Artifact speichern
Wenn der AI Code generiert, erscheint ein "+" Button in der oberen rechten Ecke des Code-Blocks. Klicke darauf, um den Code als Artifact zu speichern.

### 2. Artifacts verwalten
- Öffne die Code Artifacts-Ansicht
- Sieh dir alle gespeicherten Artifacts an
- Suche und filtere nach Bedarf
- Klicke auf ein Artifact, um es zu bearbeiten

### 3. Live-Preview nutzen
- Öffne ein HTML/CSS/JavaScript Artifact
- Klicke auf das "Auge"-Symbol für die Live-Preview
- Der Code wird in einem sicheren iframe ausgeführt
- Änderungen werden sofort angezeigt

### 4. Exportieren
- Klicke auf das Download-Symbol
- Der Code wird als Datei mit der korrekten Erweiterung heruntergeladen
- Dateiname basiert auf dem Artifact-Titel

## Komponenten

### CodeArtifact.tsx
Die Hauptkomponente für die Anzeige und Bearbeitung einzelner Code-Artifacts.

**Features:**
- Inline-Code-Bearbeitung
- Live-Preview für unterstützte Sprachen
- Copy/Download/Edit-Funktionen
- Responsive Design

### CodeArtifactManager.tsx
Verwaltet die Liste aller Code-Artifacts.

**Features:**
- Suche und Filterung
- Sortierung nach Datum, Name oder Sprache
- Artifact-Vorschau
- Bulk-Operationen

### CodeArtifactView.tsx
Die Hauptansicht, die Manager und Detail-Ansicht zusammenbringt.

**Features:**
- Import/Export-Funktionalität
- Navigation zwischen Liste und Detail
- Responsive Layout

### useCodeArtifacts.ts
Hook für die Datenverwaltung der Code-Artifacts.

**Features:**
- Lokale Speicherung in localStorage
- CRUD-Operationen
- Import/Export-Funktionen
- Such- und Filterlogik

## Integration

### In MarkdownContent.tsx
Die bestehende Code-Block-Komponente wurde erweitert:
- "+" Button für Artifact-Erstellung
- Unterstützung für verschiedene Programmiersprachen
- Hover-Effekte für bessere UX

### In der Hauptanwendung
Die Code Artifacts können in verschiedene Bereiche integriert werden:
- Als separater Tab in der Sidebar
- Als Modal/Dialog
- Als Panel in der Chat-Ansicht

## Sicherheit

### Sandbox-Umgebung
- Code wird in isolierten iframes ausgeführt
- Beschränkte Berechtigungen für JavaScript
- Kein Zugriff auf lokale Dateien oder System-Ressourcen

### Validierung
- Eingabevalidierung für alle Benutzerdaten
- Sichere Datei-Uploads
- Größenbeschränkungen für Code-Blöcke

## Technische Details

### Speicherung
- Artifacts werden in localStorage gespeichert
- JSON-Format für einfache Portabilität
- Automatische Backup-Funktionalität

### Performance
- Lazy Loading für große Artifact-Listen
- Optimierte Rendering für Code-Blöcke
- Effiziente Such- und Filteralgorithmen

### Erweiterbarkeit
- Modulare Architektur
- Plugin-System für neue Sprachen
- Konfigurierbare Preview-Umgebungen

## Beispiel-Nutzung

### Pomodoro Timer erstellen
1. Frage den AI: "Erstelle einen Pomodoro Timer in HTML/JavaScript mit Tailwind CSS"
2. Klicke auf das "+" Symbol im generierten Code
3. Gib einen Titel ein (z.B. "Pomodoro Timer")
4. Öffne das Artifact und aktiviere die Live-Preview
5. Teste den Timer direkt in der Anwendung
6. Bearbeite den Code nach Bedarf
7. Exportiere als HTML-Datei

### React-Komponente entwickeln
1. Frage den AI: "Erstelle eine React Counter-Komponente"
2. Speichere als Artifact
3. Nutze die Live-Preview für JSX/TSX
4. Iteriere über verschiedene Versionen
5. Exportiere als JSX-Datei

## Zukünftige Erweiterungen

### Geplante Features
- **Git-Integration**: Versionierung mit Git
- **Collaboration**: Teilen von Artifacts
- **Templates**: Vorgefertigte Code-Vorlagen
- **Testing**: Automatische Tests für Code-Artifacts
- **Deployment**: Direktes Deployment zu verschiedenen Plattformen

### Neue Sprachen
- **Rust**: Mit WebAssembly-Unterstützung
- **Go**: Für Backend-Code
- **SQL**: Mit Datenbank-Preview
- **Markdown**: Mit Live-Rendering

## Troubleshooting

### Häufige Probleme

**Live-Preview funktioniert nicht:**
- Prüfe, ob die Sprache unterstützt wird
- Stelle sicher, dass der Code gültig ist
- Überprüfe die Browser-Konsole auf Fehler

**Artifacts werden nicht gespeichert:**
- Prüfe den verfügbaren localStorage-Speicher
- Stelle sicher, dass JavaScript aktiviert ist
- Überprüfe die Browser-Berechtigungen

**Import schlägt fehl:**
- Prüfe das JSON-Format der Import-Datei
- Stelle sicher, dass alle erforderlichen Felder vorhanden sind
- Überprüfe die Dateigröße (max. 10MB)

## Beitragen

### Entwicklung
1. Forke das Repository
2. Erstelle einen Feature-Branch
3. Implementiere deine Änderungen
4. Schreibe Tests
5. Erstelle einen Pull Request

### Feedback
- Melde Bugs über GitHub Issues
- Schlage neue Features vor
- Teile deine Erfahrungen mit der Community

## Lizenz

Diese Funktionalität ist Teil von Goose Desktop und unterliegt der gleichen Lizenz wie das Hauptprojekt. 