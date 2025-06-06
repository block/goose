import { useParams, Link } from "react-router";
import {
  Copy,
  ArrowLeft,
  Info,
  Calendar,
  Wrench,
  Check
} from "lucide-react";
import { toast } from 'react-toastify';
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { Card, CardContent, CardHeader } from "../components/ui/card";
import { useEffect, useState } from "react";
import { fetchPrompts } from "../prompt-data";
import type { Prompt } from "../types/prompt";

export default function DetailPage() {
  const { id } = useParams();
  const [prompt, setPrompt] = useState<Prompt | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isCopied, setIsCopied] = useState(false);

  useEffect(() => {
    const loadPrompt = async () => {
      try {
        setIsLoading(true);
        setError(null);
        const prompts = await fetchPrompts();
        const foundPrompt = prompts.find((p) => p.id === id);
        if (!foundPrompt) {
          setError(`Prompt with ID "${id}" not found`);
          return;
        }
        setPrompt(foundPrompt);
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : "Unknown error";
        setError(`Failed to load prompt: ${errorMessage}`);
        console.error("Error loading prompt:", err);
      } finally {
        setIsLoading(false);
      }
    };

    loadPrompt();
  }, [id]);

  const handleCopy = async () => {
    if (prompt) {
      try {
        await navigator.clipboard.writeText(prompt.prompt);
        setIsCopied(true);
        toast.success('Prompt copied to clipboard', {
          position: "bottom-right",
          autoClose: 2000,
          hideProgressBar: false,
          closeOnClick: true,
          pauseOnHover: true,
          draggable: true,
        });
        setTimeout(() => setIsCopied(false), 2000);
      } catch (err) {
        toast.error('Failed to copy prompt', {
          position: "bottom-right",
          autoClose: 2000,
          hideProgressBar: false,
          closeOnClick: true,
          pauseOnHover: true,
          draggable: true,
        });
      }
    }
  };

  if (!prompt) {
    return (
      <div className="max-w-4xl mx-auto">
        <div className="flex items-center gap-2 mb-6">
          <Link to="/">
            <Button className="">
              <ArrowLeft className="h-4 w-4" />
              Back
            </Button>
          </Link>
          <div className="text-sm text-gray-500 dark:text-gray-400">
            <Link
              to="/"
              className="hover:text-accent dark:hover:text-accent/90"
            >
              Goose Prompts
            </Link>{" "}
            /
          </div>
        </div>
        <div className="animate-pulse">
          <div className="h-8 w-48 bg-gray-200 dark:bg-gray-700 rounded mb-4"></div>
          <div className="h-4 w-full bg-gray-200 dark:bg-gray-700 rounded mb-2"></div>
          <div className="h-4 w-2/3 bg-gray-200 dark:bg-gray-700 rounded"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto">
      <div className="flex gap-8 max-w-5xl mx-auto">
        <div>
          <Link to="/">
            <Button className="flex items-center gap-2">
              <ArrowLeft className="h-4 w-4" />
              Back
            </Button>
          </Link>
        </div>

        <Card className="p-8 w-full">
          <CardHeader className="flex items-center">
            <div className="flex items-center gap-2">
              <h1 className="font-medium text-5xl text-textProminent detail-page-prompt-title">
                {prompt.title}
              </h1>
            </div>
          </CardHeader>
          <CardContent className="space-y-6">
            <div>
              <p className="text-xl text-textSubtle">{prompt.description}</p>
            </div>

            <div className="space-y-2">
              <div className="flex items-center gap-2 text-textStandard">
                <Info className="h-4 w-4" />
                <h4 className="font-medium">Example Prompt</h4>
              </div>
              <div className="relative group">
                <code className="block bg-gray-100 dark:bg-gray-900 p-4 rounded text-sm dark:text-gray-300 whitespace-pre-wrap">
                  {prompt.prompt}
                </code>
                <Button 
                  variant="ghost" 
                  size="icon"
                  className="absolute top-2 right-2 opacity-50 hover:opacity-100"
                  onClick={handleCopy}
                >
                  {isCopied ? (
                    <Check className="h-4 w-4 text-green-500" />
                  ) : (
                    <Copy className="h-4 w-4" />
                  )}
                </Button>
              </div>
            </div>

            {prompt.extensions && prompt.extensions.length > 0 && (
              <div className="space-y-2">
                <div className="flex items-center gap-2 text-textStandard">
                  <Wrench className="h-4 w-4" />
                  <h4 className="font-medium">Required Extensions</h4>
                </div>
                <div className="flex gap-2 flex-wrap">
                  {prompt.extensions.map(extension => (
                    <Badge 
                      key={extension} 
                      variant="outline" 
                      className="text-gray-500 dark:text-gray-400 flex items-center gap-1"
                    >
                      {prompt.external ? (
                        <a 
                          href={prompt.extensionUrl} 
                          target="_blank" 
                          rel="noopener noreferrer"
                          className="hover:underline"
                        >
                          {extension}
                        </a>
                      ) : (
                        extension
                      )}
                      {prompt.external && (
                        <span className="text-xs text-gray-400 dark:text-gray-500">
                          (external)
                        </span>
                      )}
                    </Badge>
                  ))}
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}