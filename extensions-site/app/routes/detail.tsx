import { useParams, Link } from "react-router";
import {
  Download,
  Star,
  Terminal,
  ArrowLeft,
  Info
} from "lucide-react";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { Card, CardContent, CardHeader } from "../components/ui/card";
import { useEffect, useState } from "react";
import { fetchMCPServers } from "../mcp-servers";

interface Server {
  id: string;
  name: string;
  description: string;
  command: string;
  link: string;
  installation_notes: string;
  is_builtin: boolean;
  endorsed: boolean;
  githubStars: number;
  environmentVariables: {
    name: string;
    description: string;
    required: boolean;
  }[];
}

function getGooseInstallLink(server: Server): string {
  const parts = server.command.split(" ");
  const baseCmd = parts[0];
  const args = parts.slice(1);
  const queryParams = [
    `cmd=${encodeURIComponent(baseCmd)}`,
    ...args.map((arg) => `arg=${encodeURIComponent(arg)}`),
    `description=${encodeURIComponent(server.description)}`,
    ...server.environmentVariables
      .filter((env) => env.required)
      .map(
        (env) => `env=${encodeURIComponent(`${env.name}=${env.description}`)}`
      ),
  ].join("&");

  return `goose://extension?${queryParams}`;
}

export default function DetailPage() {
  const { id } = useParams();
  const [server, setServer] = useState<Server | null>(null);
  const [isCommandVisible, setIsCommandVisible] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const loadServer = async () => {
      try {
        setIsLoading(true);
        setError(null);
        const servers = await fetchMCPServers();
        const foundServer = servers.find((s) => s.id === id);
        if (!foundServer) {
          setError(`Server with ID "${id}" not found`);
          return;
        }
        setServer(foundServer);
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : "Unknown error";
        setError(`Failed to load server: ${errorMessage}`);
        console.error("Error loading server:", err);
      } finally {
        setIsLoading(false);
      }
    };

    loadServer();
  }, [id]);

  if (!server) {
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
              Goose Extensions
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
              {/* <svg
              width="13"
              height="12"
              viewBox="0 0 13 12"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              <g id="Response Icons">
                <path
                  id="Vector"
                  d="M6.29521 0.0644531C3.24885 0.0644531 3.43906 1.37819 3.43906 1.37819L3.44245 2.73927H6.34953V3.1479H2.2878C2.2878 3.1479 0.33844 2.92804 0.33844 5.98471C0.33844 9.04147 2.03989 8.93307 2.03989 8.93307H3.05531V7.51461C3.05531 7.51461 3.00057 5.82262 4.7296 5.82262H7.6129C7.6129 5.82262 9.23283 5.84864 9.23283 4.26571V1.64842C9.23283 1.64842 9.47886 0.0644531 6.29521 0.0644531ZM4.69225 0.979748C4.76094 0.979686 4.82898 0.993097 4.89245 1.01921C4.95593 1.04533 5.01361 1.08363 5.06219 1.13194C5.11076 1.18024 5.14928 1.2376 5.17554 1.30073C5.2018 1.36385 5.21529 1.43151 5.21523 1.49982C5.21529 1.56814 5.2018 1.63579 5.17554 1.69892C5.14928 1.76205 5.11076 1.8194 5.06219 1.86771C5.01361 1.91601 4.95593 1.95432 4.89245 1.98044C4.82898 2.00655 4.76094 2.01996 4.69225 2.0199C4.62355 2.01996 4.55551 2.00655 4.49204 1.98044C4.42856 1.95432 4.37088 1.91601 4.3223 1.86771C4.27373 1.8194 4.23521 1.76205 4.20895 1.69892C4.18269 1.63579 4.1692 1.56814 4.16926 1.49982C4.1692 1.43151 4.18269 1.36385 4.20895 1.30073C4.23521 1.2376 4.27373 1.18024 4.3223 1.13194C4.37088 1.08363 4.42856 1.04533 4.49204 1.01921C4.55551 0.993097 4.62355 0.979686 4.69225 0.979748Z"
                  fill="currentColor"
                />
                <path
                  id="Vector_2"
                  d="M6.38171 11.9358C9.42806 11.9358 9.23786 10.622 9.23786 10.622L9.23447 9.26098H6.32735V8.85235H10.3891C10.3891 8.85235 12.3384 9.07221 12.3384 6.01544C12.3384 2.95872 10.637 3.06713 10.637 3.06713H9.62156V4.48554C9.62156 4.48554 9.6763 6.17754 7.94727 6.17754H5.06397C5.06397 6.17754 3.44404 6.15151 3.44404 7.73449V10.3518C3.44404 10.3518 3.19806 11.9358 6.38166 11.9358H6.38171ZM7.98467 11.0206C7.91598 11.0206 7.84794 11.0072 7.78447 10.9811C7.72099 10.955 7.66331 10.9167 7.61473 10.8684C7.56616 10.8201 7.52764 10.7627 7.50138 10.6996C7.47512 10.6364 7.46163 10.5688 7.46169 10.5005C7.46163 10.4322 7.47511 10.3645 7.50136 10.3014C7.52762 10.2382 7.56614 10.1809 7.61472 10.1326C7.66329 10.0842 7.72097 10.0459 7.78445 10.0198C7.84794 9.9937 7.91597 9.98029 7.98467 9.98035C8.05337 9.98029 8.1214 9.9937 8.18488 10.0198C8.24836 10.0459 8.30604 10.0842 8.35462 10.1325C8.40319 10.1808 8.44171 10.2382 8.46797 10.3013C8.49423 10.3645 8.50772 10.4321 8.50766 10.5004C8.50772 10.5687 8.49423 10.6364 8.46797 10.6995C8.44171 10.7627 8.40319 10.82 8.35462 10.8683C8.30604 10.9166 8.24836 10.9549 8.18488 10.981C8.1214 11.0072 8.05337 11.0206 7.98467 11.0205V11.0206Z"
                  fill="currentColor"
                />
              </g>
            </svg> */}
              <h1 className="font-medium text-5xl text-textProminent detail-page-server-name">
                {server.name}
              </h1>
            </div>
          </CardHeader>
          <CardContent className="space-y-6">
            <div>
              <p className="text-xl text-textSubtle">{server.description}</p>
              {/* <Button className="mt-4">Download Goose for desktop</Button> */}
            </div>
            <div>
              <p className="text-md text-textSubtle">{server.installation_notes}</p>
            </div>

            <div className="space-y-2">
              {server.is_builtin ? (
                <div className="flex items-center gap-2 text-sm dark:text-gray-300">
                  <Info className="h-4 w-4" />
                  Can be enabled in the goose settings page
                </div>
              ) : (
                <>
                  <div className="flex items-center gap-2 text-textStandard">
                    <Terminal className="h-4 w-4" />
                    <h4 className="font-medium">Command</h4>
                  </div>
                  <code className="block bg-gray-100 dark:bg-gray-900 p-2 rounded text-sm dark:text-gray-300">
                    {`goose session --with-extension "${server.command}"`}
                  </code>
                </>
              )}
            </div>
            <div className="space-y-4">
              <h2 className="text-lg font-medium dark:text-gray-300">
                Environment Variables
              </h2>
              {server.environmentVariables.length > 0 ? (
                <div className="">
                  {server.environmentVariables.map((env) => (
                    <div
                      key={env.name}
                      className="border-b border-borderSubtle pb-4 mb-4 last:border-0"
                    >
                      <div className="text-sm dark:text-gray-300">
                        {env.name}
                      </div>
                      <div className="text-gray-600 dark:text-gray-400 text-sm mt-1">
                        {env.description}
                      </div>
                      {env.required && (
                        <Badge variant="secondary" className="mt-2">
                          Required
                        </Badge>
                      )}
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-gray-600 dark:text-gray-400 text-sm flex items-center gap-2">
                  <Info className="h-4 w-4" />
                  No environment variables needed
                </div>
              )}
            </div>

            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                <Star className="h-4 w-4" />
                <span>{server.githubStars} on Github</span>
              </div>

              <a
                href={getGooseInstallLink(server)}
                target="_blank"
                rel="noopener noreferrer"
                className="no-underline"
              >
                {server.is_builtin ? (
                  <div
                    className="inline-block"
                    title="This extension is built into goose and can be enabled in the settings page"
                  >
                    <Badge
                      variant="secondary"
                      className="ml-2 text-xs cursor-help"
                    >
                      Built-in
                    </Badge>
                  </div>
                ) : (
                  <Button
                    size="icon"
                    variant="link"
                    className="group/download flex items-center justify-center text-xs leading-[14px] text-textSubtle px-0 transition-all"
                    title="Install with Goose"
                  >
                    <span>Install</span>
                    <Download className="h-4 w-4 ml-2 group-hover/download:text-[#FA5204]" />
                  </Button>
                )}
              </a>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
