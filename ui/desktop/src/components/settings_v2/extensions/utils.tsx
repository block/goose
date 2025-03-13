// TODO: copied this from old code

// import {View} from '../../../App'
// import {SettingsViewOptions} from "../SettingsView";
// import {toast} from "react-toastify";
//
//
// export async function addExtensionFromDeepLink(
//     url: string,
//     setView: (view: View, options: SettingsViewOptions) => void
// ) {
//     if (!url.startsWith('goose://extension')) {
//         handleError(
//             'Failed to install extension: Invalid URL: URL must use the goose://extension scheme'
//         );
//         return;
//     }
//
//     const parsedUrl = new URL(url);
//
//     if (parsedUrl.protocol !== 'goose:') {
//         handleError(
//             'Failed to install extension: Invalid protocol: URL must use the goose:// scheme',
//             true
//         );
//     }
//
//     // Check that all required fields are present and not empty
//     const requiredFields = ['name', 'description'];
//
//     for (const field of requiredFields) {
//         const value = parsedUrl.searchParams.get(field);
//         if (!value || value.trim() === '') {
//             handleError(
//                 `Failed to install extension: The link is missing required field '${field}'`,
//                 true
//             );
//         }
//     }
//
//     const cmd = parsedUrl.searchParams.get('cmd');
//     if (!cmd) {
//         handleError("Failed to install extension: Missing required 'cmd' parameter in the URL", true);
//     }
//
//     // Validate that the command is one of the allowed commands
//     const allowedCommands = ['npx', 'uvx', 'goosed'];
//     if (!allowedCommands.includes(cmd)) {
//         handleError(
//             `Failed to install extension: Invalid command: ${cmd}. Only ${allowedCommands.join(', ')} are allowed.`,
//             true
//         );
//     }
//
//     // Check for security risk with npx -c command
//     const args = parsedUrl.searchParams.getAll('arg');
//     if (cmd === 'npx' && args.includes('-c')) {
//         handleError(
//             'Failed to install extension: npx with -c argument can lead to code injection',
//             true
//         );
//     }
//
//     const envList = parsedUrl.searchParams.getAll('env');
//     const id = parsedUrl.searchParams.get('id');
//     const name = parsedUrl.searchParams.get('name');
//     const description = parsedUrl.searchParams.get('description');
//     const timeout = parsedUrl.searchParams.get('timeout');
//
//     // split env based on delimiter to a map
//     const envs = envList.reduce(
//         (acc, env) => {
//             const [key, value] = env.split('=');
//             acc[key] = value;
//             return acc;
//         },
//         {} as Record<string, string>
//     );
//
//     // Create a ExtensionConfig from the URL parameters
//     // Parse timeout if provided, otherwise use default
//     const parsedTimeout = timeout ? parseInt(timeout, 10) : null;
//
//     const extensionConfig: ExtensionConfig = {
//         id,
//         name,
//         type: 'stdio',
//         cmd,
//         args,
//         description,
//         enabled: true,
//         env_keys: Object.keys(envs).length > 0 ? Object.keys(envs) : [],
//         timeout:
//             parsedTimeout !== null && !isNaN(parsedTimeout) && Number.isInteger(parsedTimeout)
//                 ? parsedTimeout
//                 : DEFAULT_EXTENSION_TIMEOUT,
//     };
//
//     // Store the extension config regardless of env vars status
//     storeExtensionConfig(extensionConfig);
//
//     // Check if extension requires env vars and go to settings if so
//     if (envVarsRequired(extensionConfig)) {
//         console.log('Environment variables required, redirecting to settings');
//         setView('settings', { extensionId: extensionConfig.id, showEnvVars: true });
//         return;
//     }
//
//     // If no env vars are required, proceed with extending Goosed
//     await addExtension(extensionConfig);
// }
//
// function handleError(message: string, shouldThrow = false): void {
//     toast.error(message);
//     console.error(message);
//     if (shouldThrow) {
//         throw new Error(message);
//     }
// }

// TODO: when rust app starts, add built-in extensions to config.yaml if they aren't there already
