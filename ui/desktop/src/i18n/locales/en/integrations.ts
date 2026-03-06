export const integrations = {
  extensions: {
    addCustomExtension: 'Add custom extension',
    browseExtensions: 'Browse extensions',
    updateExtension: 'Update Extension',
    addExtension: 'Add Extension',
    deleteExtensionTitle: 'Delete Extension "{{name}}"',
    deleteExtensionDescription:
      'This will permanently remove this extension and all of its settings.',
    installationNotes: 'Installation Notes',
    removeExtension: 'Remove extension',
    unsavedChangesTitle: 'Unsaved Changes',
    unsavedChangesMessage:
      'You have unsaved changes to the extension configuration. Are you sure you want to close without saving?',
    fields: {
      extensionName: 'Extension Name',
      nameRequired: 'Name is required',
      type: 'Type',
      description: 'Description',
      descriptionPlaceholder: 'Optional description...',
      namePlaceholder: 'Enter extension name...',
      standardIo: 'Standard IO (STDIO)',
      streamableHttp: 'Streamable HTTP',
      unsupportedSse: 'SSE (unsupported)',
      command: 'Command',
      commandPlaceholder: 'e.g. npx -y @modelcontextprotocol/my-extension <filepath>',
      commandRequired: 'Command is required',
      endpoint: 'Endpoint',
      endpointPlaceholder: 'Enter endpoint URL...',
      endpointRequired: 'Endpoint URL is required',
      timeout: 'Timeout',
      timeoutRequired: 'Timeout is required',
      environmentVariables: 'Environment Variables',
      environmentVariablesDescription:
        'Add key-value pairs for environment variables. Click the "+" button to add after filling both fields. For existing secret values, click the edit button to modify.',
      variableName: 'Variable name',
      value: 'Value',
      add: 'Add',
      variableNameAndValueRequired: 'Both variable name and value must be entered',
      variableNameNoSpaces: 'Variable name cannot contain spaces',
      requestHeaders: 'Request Headers',
      requestHeadersDescription:
        'Add custom HTTP headers to include in requests to the MCP server. Click the "+" button to add after filling both fields.',
      headerName: 'Header name',
      headerNameAndValueRequired: 'Both header name and value must be entered',
      headerNameNoSpaces: 'Header name cannot contain spaces',
      duplicateHeader: 'A header with this name already exists',
    },
  },
  extensionsPage: {
    title: 'Extensions',
    description:
      "These extensions use the Model Context Protocol (MCP). They can expand Goose's capabilities using three main components: Prompts, Resources, and Tools. {{shortcut}} to search.",
    defaultsDescription:
      'Extensions enabled here are used as the default for new chats. You can also toggle active extensions during chat.',
    searchPlaceholder: 'Search extensions...',
    defaultExtensions: 'Default Extensions ({{count}})',
    availableExtensions: 'Available Extensions ({{count}})',
    noExtensionsAvailable: 'No extensions available',
    configureExtension: 'Configure {{name}} Extension',
    toggleExtension: 'Toggle {{name}} extension On or Off',
    builtInFallback: 'Built-in extension',
    streamableHttpExtension: 'Streamable HTTP extension',
    sseExtension: 'SSE extension',
    builtIns: {
      analyze: {
        title: 'Analyze',
        description:
          'Analyze code structure with tree-sitter: directory overviews, file details, symbol call graphs',
      },
      apps: {
        title: 'Apps',
        description:
          'Create and manage custom Goose apps through chat. Apps are HTML/CSS/JavaScript and run in sandboxed windows.',
      },
      developer: {
        title: 'Developer',
        description: 'General development tools useful for software engineering.',
      },
      extensionManager: {
        title: 'Extension Manager',
        description:
          'Enable extension management tools for discovering, enabling, and disabling extensions',
      },
      summon: {
        title: 'Summon',
        description: 'Load knowledge and delegate tasks to subagents',
      },
      todo: {
        title: 'Todo',
        description: 'Enable a todo list for goose so it can keep track of what it is doing',
      },
      topOfMind: {
        title: 'Top Of Mind',
        description:
          'Inject custom context into every turn via GOOSE_MOIM_MESSAGE_TEXT and GOOSE_MOIM_MESSAGE_FILE environment variables',
      },
      autoVisualiser: {
        title: 'Auto Visualiser',
        description: 'Automatically see your data with this UI generation extension.',
      },
      chatRecall: {
        title: 'Chat Recall',
        description:
          'Search past conversations and load session summaries for contextual memory',
      },
      codeMode: {
        title: 'Code Mode',
        description: 'Goose will make extension calls through code execution, saving tokens',
      },
      computerController: {
        title: 'Computer Controller',
        description:
          "General computer control tools that don't require you to be a developer or engineer.",
      },
      memory: {
        title: 'Memory',
        description: 'Teach goose your preferences as you go.',
      },
      tutorial: {
        title: 'Tutorial',
        description: 'Access interactive tutorials and guides',
      },
    },
  },
  providers: {
    onboardingTitle: 'Other providers',
    settingsTitle: 'Provider Configuration Settings',
    onboardingDescription:
      "Select an AI model provider to get started with goose. You'll need to use API keys generated by each provider which will be encrypted and stored locally. You can change your provider at any time in settings.",
    loadingProviders: 'Loading providers...',
    addProvider: 'Add Provider',
    addProviderDescription: 'From template or manual setup',
    editProvider: 'Edit Provider',
    configureProvider: 'Configure Provider',
    chooseModel: 'Choose Model',
    welcomeTitle: 'Welcome to Goose',
    welcomeDescription:
      "Since it's your first time here, let's get you set up with an AI provider so Goose can work its magic.",
    setupFailed: 'Setup Failed',
    setupError: 'Setup Error',
    unexpectedSetupError: 'An unexpected error occurred during setup.',
    freePrivate: 'Free & Private',
    runLocally: 'Run Locally',
    runLocallyDescription:
      'Download a model and run entirely on your machine. No API keys, no accounts.',
    recommendedChatgpt: 'Recommended if you have ChatGPT Plus/Pro',
    chatgptSubscription: 'ChatGPT Subscription',
    chatgptSubscriptionDescription:
      'Use your ChatGPT Plus/Pro subscription for GPT-5 Codex models.',
    recommendedNewUsers: 'Recommended for new users',
    tetrateDescription:
      'Access multiple AI models with automatic setup. Sign up to receive $10 credit.',
    openRouterDescription: 'Access 200+ models with one API. Pay-per-use pricing.',
    otherProvidersDescription: 'Set up additional providers manually through settings.',
    goToProviderSettings: 'Go to Provider Settings ->',
    moreOptionsBelow: 'More options below',
  },
};
