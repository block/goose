import { useEffect, useRef, useState } from 'react';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { GooseApp, iterateApp, storeApp } from '../../api';

interface GooseAppEditorProps {
  app?: GooseApp;
  onReturn: () => void;
}

const DEFAULT_JS = `class HelloWidget extends GooseWidget {
    render() {
        return \`
            <div class="hello-container">
                Hello World
            </div>
        \`;
    }
}`;

export default function GooseAppEditor({ app, onReturn }: GooseAppEditorProps) {
  const [name, setName] = useState(app?.name || '');
  const [description, setDescription] = useState(app?.description || '');
  const [width, setWidth] = useState(app?.width?.toString() || '320');
  const [height, setHeight] = useState(app?.height?.toString() || '200');
  const [resizable, setResizable] = useState(app?.resizable ?? true);
  const [prd, setPrd] = useState(app?.prd || '');
  const iframeRef = useRef<React.ComponentRef<'iframe'>>(null);
  const [iframeErrors, setIframeErrors] = useState<string[]>([]);
  const [iframeReady, setIframeReady] = useState(false);
  const [jsImplementation, setJsImplementation] = useState(app?.jsImplementation || DEFAULT_JS);
  const [isIterating, setIsIterating] = useState(false);
  const [iterationMessage, setIterationMessage] = useState('');
  const [iframeKey, setIframeKey] = useState(0);
  const [containerHtml, setContainerHtml] = useState('<p style="color: #0f6636">Loading ...</p>');
  const [detailsOpen, setDetailsOpen] = useState(!name || !width || !height);

  useEffect(() => {
    const handleMessage = (event: globalThis.MessageEvent) => {
      if (event.data.type === 'error') {
        setIframeErrors((prev) => [...prev, event.data.message]);
      } else if (event.data.type === 'ready') {
        setIframeReady(true);
      }
    };
    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, []);

  useEffect(() => {
    const loadHtml = async () => {
      const gooseApp: GooseApp = { jsImplementation, name, prd: '' };
      const html = await window.electron.previewGooseApp(gooseApp);
      console.log(html);
      setContainerHtml(html);
      setIframeKey((prev) => prev + 1);
    };
    loadHtml();
  }, [jsImplementation, name]);

  const captureScreenshot = async (): Promise<Blob> => {
    if (!iframeRef.current) throw new Error('Iframe not ready');
    if (!iframeReady) throw new Error('Iframe not loaded yet');

    const rect = iframeRef.current.getBoundingClientRect();
    const bounds = {
      x: Math.round(rect.x),
      y: Math.round(rect.y),
      width: Math.round(rect.width),
      height: Math.round(rect.height),
    };

    const pngBuffer = await window.electron.captureScreenShot(bounds);
    return new Blob([pngBuffer.buffer as ArrayBuffer], { type: 'image/png' });
  };

  const handleUpdate = async () => {
    setIsIterating(true);
    setIterationMessage('Starting iteration...');

    let currentJs = jsImplementation;
    let done = false;

    while (!done) {
      const screenshot = await captureScreenshot();
      const arrayBuffer = await screenshot.arrayBuffer();
      const base64 = globalThis.btoa(String.fromCharCode(...new Uint8Array(arrayBuffer)));

      const response = await iterateApp({
        body: {
          jsImplementation: currentJs,
          prd,
          screenshotBase64: base64,
          errors: iframeErrors.join('\n'),
        },
        throwOnError: true,
      });

      setIterationMessage(response.data.message);

      if (response.data.done) {
        done = true;
        setIterationMessage('Done! ' + response.data.message);
      } else {
        currentJs = response.data.jsImplementation!;
        setJsImplementation(currentJs);
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
    }

    setIsIterating(false);
  };

  const handleSave = async () => {
    try {
      await storeApp({
        path: { name },
        body: {
          app: {
            name,
            description: description || null,
            width: width ? parseInt(width) : null,
            height: height ? parseInt(height) : null,
            resizable,
            prd,
            jsImplementation,
          },
        },
      });
    } catch (_e) {
      console.log(_e);
    }
    onReturn();
  };

  return (
    <MainPanelLayout>
      <div className="flex flex-col min-w-0 flex-1 overflow-y-auto">
        <div className="bg-background-default px-8 pb-4 pt-16">
          <div className="flex items-center gap-4 mb-6">
            <h1 className="text-4xl font-light">{app ? 'Edit App' : 'Create App'}</h1>
            <button
              onClick={() => setDetailsOpen(!detailsOpen)}
              className="text-sm font-medium cursor-pointer flex items-center gap-1 text-text-muted hover:text-text-default"
            >
              <span
                className={`inline-block transition-transform ${detailsOpen ? 'rotate-90' : ''}`}
              >
                ▶
              </span>
              Details
            </button>
          </div>

          {detailsOpen && (
            <div className="grid grid-cols-2 gap-3 mb-6">
              <div className="col-span-2">
                <Input
                  placeholder="App Name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
              </div>
              <div className="col-span-2">
                <Input
                  placeholder="Description (optional)"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                />
              </div>
              <div>
                <Input
                  type="number"
                  placeholder="Width"
                  value={width}
                  onChange={(e) => setWidth(e.target.value)}
                />
              </div>
              <div>
                <Input
                  type="number"
                  placeholder="Height"
                  value={height}
                  onChange={(e) => setHeight(e.target.value)}
                />
              </div>
              <div className="col-span-2 flex items-center gap-2">
                <input
                  type="checkbox"
                  id="resizable"
                  checked={resizable}
                  onChange={(e) => setResizable(e.target.checked)}
                  className="w-4 h-4"
                />
                <label htmlFor="resizable" className="text-sm">
                  Resizable
                </label>
              </div>
            </div>
          )}

          <div className="mb-6">
            <h2 className="text-lg font-medium mb-2">Preview</h2>
            <iframe
              key={iframeKey}
              ref={iframeRef}
              srcDoc={containerHtml}
              style={{
                width: width ? `${width}px` : '300px',
                height: height ? `${height}px` : '180px',
                border: '1px solid #ccc',
              }}
              sandbox="allow-scripts"
            />
          </div>

          <div className="mb-6">
            <h2 className="text-lg font-medium mb-2">Product Requirements</h2>
            <textarea
              placeholder="Describe what the widget should do..."
              value={prd}
              onChange={(e) => setPrd(e.target.value)}
              className="w-full min-h-[150px] p-3 rounded border border-border-muted bg-background-panel text-text-default"
            />
          </div>

          {iterationMessage && (
            <div className="mb-4 p-3 bg-background-panel rounded border border-border-muted">
              {iterationMessage}
            </div>
          )}

          <div className="flex gap-3 pb-8">
            <Button onClick={handleUpdate} disabled={isIterating || !prd} variant="default">
              Update
            </Button>
            <Button onClick={handleSave} disabled={isIterating || !name} variant="default">
              Save
            </Button>
            <Button onClick={onReturn} disabled={isIterating} variant="outline">
              Cancel
            </Button>
          </div>
        </div>
      </div>
    </MainPanelLayout>
  );
}
