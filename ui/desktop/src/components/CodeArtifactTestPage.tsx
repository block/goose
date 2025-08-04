import React, { useState } from 'react';
import { Button } from './ui/button';
import CodeArtifactView from './CodeArtifactView';

export const CodeArtifactTestPage: React.FC = () => {
  const [showArtifacts, setShowArtifacts] = useState(false);

  return (
    <div className="p-6">
      <div className="max-w-6xl mx-auto">
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-text-default mb-4">Code Artifacts Test</h1>
          <p className="text-text-muted mb-6">
            Teste die Code Artifacts-Funktionalität mit verschiedenen Code-Beispielen.
          </p>

          <Button
            onClick={() => setShowArtifacts(!showArtifacts)}
            className="bg-blue-600 hover:bg-blue-700"
          >
            {showArtifacts ? 'Artifacts ausblenden' : 'Code Artifacts öffnen'}
          </Button>
        </div>

        {showArtifacts && (
          <div className="border border-border-default rounded-lg overflow-hidden h-[800px]">
            <CodeArtifactView />
          </div>
        )}

        <div className="mt-8 grid grid-cols-1 md:grid-cols-2 gap-6">
          <div className="p-6 bg-background-muted rounded-lg">
            <h2 className="text-xl font-semibold text-text-default mb-4">Wie es funktioniert:</h2>
            <ol className="space-y-2 text-text-muted list-decimal list-inside">
              <li>Öffne die Code Artifacts-Ansicht</li>
              <li>Klicke "Beispiele erstellen" für Demo-Code</li>
              <li>Öffne ein Artifact und aktiviere die Live-Preview</li>
              <li>Bearbeite den Code und sieh die Änderungen sofort</li>
              <li>Exportiere deine Artifacts als Dateien</li>
            </ol>
          </div>

          <div className="p-6 bg-background-muted rounded-lg">
            <h2 className="text-xl font-semibold text-text-default mb-4">Unterstützte Sprachen:</h2>
            <ul className="space-y-2 text-text-muted">
              <li>
                • <strong>HTML</strong> - Vollständige Live-Preview
              </li>
              <li>
                • <strong>CSS</strong> - Angewendet auf Beispiel-Inhalte
              </li>
              <li>
                • <strong>JavaScript</strong> - Mit Console-Output
              </li>
              <li>
                • <strong>React (JSX/TSX)</strong> - Komponenten-Preview
              </li>
              <li>
                • <strong>TypeScript</strong> - Als Datei exportierbar
              </li>
              <li>
                • <strong>Python, Java, C++</strong> - Als Datei exportierbar
              </li>
            </ul>
          </div>
        </div>

        <div className="mt-8 p-6 bg-background-muted rounded-lg">
          <h2 className="text-xl font-semibold text-text-default mb-4">Test-Code Beispiele:</h2>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="p-4 bg-background-default rounded border">
              <h3 className="font-medium mb-2">Pomodoro Timer</h3>
              <p className="text-sm text-text-muted mb-3">
                Ein vollständiger Timer mit HTML, CSS und JavaScript
              </p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  // This would create a sample Pomodoro timer
                  console.log('Creating Pomodoro Timer sample...');
                }}
              >
                Als Artifact erstellen
              </Button>
            </div>

            <div className="p-4 bg-background-default rounded border">
              <h3 className="font-medium mb-2">React Counter</h3>
              <p className="text-sm text-text-muted mb-3">
                Eine React-Komponente mit Hooks und State
              </p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  console.log('Creating React Counter sample...');
                }}
              >
                Als Artifact erstellen
              </Button>
            </div>

            <div className="p-4 bg-background-default rounded border">
              <h3 className="font-medium mb-2">CSS Animation</h3>
              <p className="text-sm text-text-muted mb-3">Schöne CSS-Animationen und Transitions</p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  console.log('Creating CSS Animation sample...');
                }}
              >
                Als Artifact erstellen
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default CodeArtifactTestPage;
