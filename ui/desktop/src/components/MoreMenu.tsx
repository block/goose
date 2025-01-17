import {
  Popover,
  PopoverContent,
  PopoverTrigger,
  PopoverPortal,
} from "@radix-ui/react-popover";
import React, { useEffect, useState } from "react";
import { FaMoon, FaSun } from "react-icons/fa";
import VertDots from "./ui/VertDots";
import { useNavigate } from "react-router-dom";
import { More } from "./icons";
interface VersionInfo {
  current_version: string;
  available_versions: string[];
}

export default function MoreMenu() {
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [versions, setVersions] = useState<VersionInfo | null>(null);
  const [showVersions, setShowVersions] = useState(false);

  const [useSystemTheme, setUseSystemTheme] = useState(
    () => localStorage.getItem("use_system_theme") === "true"
  );

  const [isDarkMode, setDarkMode] = useState(() => {
    const systemPrefersDark = window.matchMedia(
      "(prefers-color-scheme: dark)"
    ).matches;
    if (useSystemTheme) {
      return systemPrefersDark;
    }
    const savedTheme = localStorage.getItem("theme");
    return savedTheme ? savedTheme === "dark" : systemPrefersDark;
  });

  useEffect(() => {
    // Fetch available versions when the menu opens
    const fetchVersions = async () => {
      try {
        const port = window.appConfig.get("GOOSE_PORT");
        const response = await fetch(
          `http://127.0.0.1:${port}/agent/versions`
        );
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        setVersions(data);
      } catch (error) {
        console.error("Failed to fetch versions:", error);
      }
    };

    if (open) {
      fetchVersions();
    }
  }, [open]);

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");

    // Handler for system theme changes
    const handleThemeChange = (e: { matches: boolean }) => {
      if (useSystemTheme) {
        setDarkMode(e.matches);
      }
    };

    // Add listener for system theme changes
    mediaQuery.addEventListener("change", handleThemeChange);

    // Initial setup
    if (useSystemTheme) {
      setDarkMode(mediaQuery.matches);
    } else {
      const savedTheme = localStorage.getItem("theme");
      setDarkMode(savedTheme ? savedTheme === "dark" : mediaQuery.matches);
    }

    // Cleanup
    return () => mediaQuery.removeEventListener("change", handleThemeChange);
  }, [useSystemTheme]);

  useEffect(() => {
    if (isDarkMode) {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
    if (!useSystemTheme) {
      localStorage.setItem("theme", isDarkMode ? "dark" : "light");
    }
  }, [isDarkMode, useSystemTheme]);

  const toggleTheme = () => {
    if (!useSystemTheme) {
      setDarkMode(!isDarkMode);
    }
  };

  const toggleUseSystemTheme = (event: React.ChangeEvent<HTMLInputElement>) => {
    const checked = event.target.checked;
    setUseSystemTheme(checked);
    localStorage.setItem("use_system_theme", checked.toString());

    if (checked) {
      // If enabling system theme, immediately sync with system preference
      const systemPrefersDark = window.matchMedia(
        "(prefers-color-scheme: dark)"
      ).matches;
      setDarkMode(systemPrefersDark);
      localStorage.removeItem("theme"); // Remove manual theme setting
    }
    // If disabling system theme, keep current theme state but don't update localStorage yet
  };

  const handleVersionSelect = (version: string) => {
    setOpen(false);
    setShowVersions(false);
    // Create a new chat window with the selected version
    window.electron.createChatWindow(undefined, undefined, version);
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button className="z-[100] absolute top-2 right-[10px] w-[20px] h-[20px] cursor-pointer no-drag">
          <More />
        </button>
      </PopoverTrigger>
      <PopoverPortal>
        <PopoverContent
          className="z-[200] w-48 rounded-md bg-black text-white dark:bg-gray-800 shadow-lg"
          align="end"
          sideOffset={5}
        >
          <div className="flex flex-col rounded-md">
            <div className="flex items-center justify-between p-2">
              <span className="text-sm">Use System Theme</span>
              <input
                type="checkbox"
                checked={useSystemTheme}
                onChange={toggleUseSystemTheme}
              />
            </div>
            {!useSystemTheme && (
              <div className="flex items-center justify-between p-2">
                <span className="text-sm">
                  {isDarkMode ? "Dark Mode" : "Light Mode"}
                </span>
                <button
                  className={`relative inline-flex items-center h-6 rounded-full w-11 focus:outline-none border-2 ${
                    isDarkMode
                      ? "bg-gray-600 border-gray-600"
                      : "bg-yellow-300 border-yellow-300"
                  }`}
                  onClick={() => toggleTheme()}
                >
                  <span
                    className={`inline-block w-4 h-4 transform bg-white rounded-full transition-transform ${
                      isDarkMode ? "translate-x-6" : "translate-x-1"
                    }`}
                  >
                    {isDarkMode ? (
                      <FaMoon className="text-gray-200" />
                    ) : (
                      <FaSun className="text-yellow-500" />
                    )}
                  </span>
                </button>
              </div>
            )}

            {/* Versions Menu */}
            {/* NOTE from alexhancock on 1/14/2025 - disabling temporarily until we figure out where this will go in settings */}
            {false && versions && versions.available_versions.length > 0 && (
              <>
                <button
                  onClick={() => setShowVersions(!showVersions)}
                  className="w-full text-left px-2 py-1.5 text-sm hover:bg-gray-700 flex justify-between items-center"
                >
                  <span>Versions</span>
                  <span className="text-xs">{showVersions ? "▼" : "▶"}</span>
                </button>
                {showVersions && (
                  <div className="pl-2 bg-gray-900">
                    {versions.available_versions.map((version) => (
                      <button
                        key={version}
                        onClick={() => handleVersionSelect(version)}
                        className={`w-full text-left px-2 py-1.5 text-sm hover:bg-gray-700 ${
                          version === versions.current_version
                            ? "text-green-400"
                            : ""
                        }`}
                      >
                        {version}{" "}
                        {version === versions.current_version && "(current)"}
                      </button>
                    ))}
                  </div>
                )}
              </>
            )}

            {/* Settings Menu */}
            {process.env.NODE_ENV === "development" && (
              <button
                onClick={() => {
                  setOpen(false);
                  navigate("/settings");
                }}
                className="w-full text-left px-2 py-1.5 text-sm hover:bg-gray-700"
              >
                Settings
              </button>
            )}

            <button
              onClick={() => {
                setOpen(false);
                window.electron.directoryChooser();
              }}
              className="w-full text-left px-2 py-1.5 text-sm hover:bg-gray-700"
            >
              Open Directory (cmd+O)
            </button>
            <button
              onClick={() => {
                setOpen(false);
                window.electron.createChatWindow();
              }}
              className="w-full text-left px-2 py-1.5 text-sm hover:bg-gray-700"
            >
              New Session (cmd+N)
            </button>
            <button
              onClick={() => {
                localStorage.removeItem("GOOSE_PROVIDER");
                setOpen(false);
                window.electron.createChatWindow();
              }}
              className="w-full text-left px-2 py-1.5 text-sm hover:bg-gray-700 text-red-400"
            >
              Reset Provider
            </button>
            {/* Provider keys settings */}
            {process.env.NODE_ENV === "development" && (
                <button
                    onClick={() => {
                      setOpen(false);
                      navigate("/keys");
                    }}
                    className="w-full text-left px-2 py-1.5 text-sm hover:bg-gray-700"
                >
                  Provider Settings (alpha)
                </button>
            )}
          </div>
        </PopoverContent>
      </PopoverPortal>
    </Popover>
  );
}
