import React, {useState, useEffect} from 'react';
import {Popover, PopoverContent, PopoverTrigger} from './ui/popover';
import VertDots from './ui/VertDots';
import {FaSun, FaMoon} from 'react-icons/fa';

export default function MoreMenu() {
    const [open, setOpen] = useState(false);

    const [useSystemTheme, setUseSystemTheme] = useState(() =>
        localStorage.getItem('use_system_theme') === 'true'
    );

    const [isDarkMode, setDarkMode] = useState(() => {
        const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        if (useSystemTheme) {
            return systemPrefersDark;
        }
        const savedTheme = localStorage.getItem('theme');
        return savedTheme ? savedTheme === 'dark' : systemPrefersDark;
    });

    useEffect(() => {
        const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        if (useSystemTheme) {
            setDarkMode(systemPrefersDark);
        } else {
            const savedTheme = localStorage.getItem('theme');
            setDarkMode(savedTheme ? savedTheme === 'dark' : systemPrefersDark);
        }
    }, [useSystemTheme]);

    useEffect(() => {
        if (isDarkMode) {
            document.documentElement.classList.add('dark');
        } else {
            document.documentElement.classList.remove('dark');
        }
        if (!useSystemTheme) {
            localStorage.setItem('theme', isDarkMode ? 'dark' : 'light');
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
        localStorage.setItem('use_system_theme', checked.toString());
    };

    return (
        <Popover open={open} onOpenChange={setOpen}>
            <PopoverTrigger asChild>
                <button
                    className="z-[100] absolute top-[-4px] right-[10px] w-[20px] h-[20px] cursor-pointer no-drag">
                    <VertDots size={18}/>
                </button>
            </PopoverTrigger>
            <PopoverContent className="w-48 rounded-md">
                <div className="flex flex-col bg-black text-white dark:bg-gray-800 rounded-md">
                    <div className="flex items-center justify-between p-2">
                        <span className="text-sm">Use System Theme</span>
                        <input
                            type="checkbox"
                            checked={useSystemTheme}
                            onChange={toggleUseSystemTheme}
                        />
                    </div>
                    {!useSystemTheme && (<div className="flex items-center justify-between p-2">
                        <span className="text-sm">{isDarkMode ? 'Dark Mode' : 'Light Mode'}</span>
                        <button
                            className={`relative inline-flex items-center h-6 rounded-full w-11 focus:outline-none border-2 ${isDarkMode
                                ? 'bg-gray-600 border-gray-600'
                                : 'bg-yellow-300 border-yellow-300'}`}
                            onClick={() => toggleTheme()}>
              <span
                  className={`inline-block w-4 h-4 transform bg-white rounded-full transition-transform ${isDarkMode
                      ? 'translate-x-6' : 'translate-x-1'}`}
              >
                {isDarkMode ? <FaMoon className="text-gray-200"/> : <FaSun
                    className="text-yellow-500"/>}
              </span>
                        </button>
                    </div>)}
                    <button
                        onClick={() => {
                            setOpen(false);
                            window.electron.directoryChooser();
                        }}
                        className="w-full text-left px-2 py-1.5 text-sm"
                    >
                        Open Directory (cmd+O)
                    </button>
                    <button
                        onClick={() => {
                            setOpen(false);
                            window.electron.createChatWindow();
                        }}
                        className="w-full text-left px-2 py-1.5 text-sm"
                    >
                        New Session (cmd+N)
                    </button>
                </div>
            </PopoverContent>
        </Popover>
    );
}
