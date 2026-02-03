export const DATABASE_CONFIG_NOT_FOUND = "No database config present!";
export const UNAUTHORIZED_USER = "Unauthorized user!";
export const MALFORMED_INPUTS = "Malformed input! Please check the docs";
export const OPERATION_FAILED = "Operation failed!";

// API Keys
export const NO_API_KEY = "No such apiKey exists!";

// Prompts
export const PROMPT_NAME_TAKEN = "Prompt name is already taken!";
export const PROMPT_NOT_CREATED = "Prompt cannot be created!";
export const PROMPT_SAVED = "Prompt saved successfully!";
export const NO_PROMPT = "No such prompt exists or isn't released yet!";
export const PROMPT_DELETED = "Prompt deleted successfully!";
export const PROMPT_NOT_DELETED = "Error deleting prompt!";
export const VERSION_NOT_CREATED = "Version cannot be created";
export const VERSION_NOT_SAVED = "Version cannot be saved";
export const VERSION_SAVED = "Prompt Version saved successfully!";
export const DOWNLOAD_INFO_NOT_SAVED = "Download info cannot be saved!";
export const PROMPT_TIPS_TO_USE_VARIABLES = "Tip: Use {{variableName}} to add variables";

// Vault
export const SECRET_NAME_TAKEN = "Secret name is taken!";
export const SECRET_SAVED = "Secret saved successfully!";
export const SECRET_NOT_SAVED = "Secret cannot be saved";
export const SECRET_DELETED = "Secret deleted successfully!";
export const SECRET_NOT_DELETED = "Error deleting secret!";

// Evaluations
export const EVALUATION_CONFIG_NOT_FOUND = "Evaluation config not set!";
export const EVALUATION_VAULT_SECRET_NOT_FOUND =
	"Evaluation secret for provider not found!";
export const EVALUATION_CONFIG_SET_ERROR = "Evaluation config cannot be set!";
export const EVALUATION_CONFIG_NOT_SET =
	"Evaluation config not set! Please set the config first to run evaluations.";
export const EVALUATION_CONFIG_SET = "Setup Evaluation!";
export const EVALUATION_NOT_RUN_YET =
	"Evaluation not run yet! Please run the evaluation to get results.";
export const EVALUATION_RUN = "Run Evaluation";
export const EVALUATION_RUN_AGAIN = "Run Evaluation Again";
export const EVALUATION_DATA_LOADING = "Loading evaluation data...";
export const EVALUATION_CREATED = "Evaluation created successfully!";
export const EVALUATION_UPDATED = "Evaluation updated successfully!";
export const EVALUATION_CONFIG_MODIFYING = "Modifying evaluation config...";
export const EVALUATION_CONFIG_INVALID = "Invalid evaluation config!";
export const EVALUATION_CONFIG_UPDATING_FAILED =
	"Evaluation config updation failed!";
export const EVALUATION_RUN_FAILURE = "Evaluation run failed!";

// Traces
export const TRACE_NOT_FOUND = "Trace not found!";
export const TRACE_FETCHING_ERROR = "Error fetching trace!";

// Cron
export const CRON_RECURRING_TIME_INVALID =
	"Invalid cron schedule. Please check the format.";
export const CRON_JOB_UPDATION_ERROR = "Error updating cron job.";

// Manage Dashboard
export const BOARD_DATA_NOT_FOUND = "Board data not found!";
export const MANAGE_DASHBOARD_EXPLORER_EMPTY_STATE =
	"No dashboards or folders yet. Click 'Add' to create one.";
export const BOARD_UPDATE_FAILED = "Board update failed!";
export const BOARD_UPDATED_SUCCESSFULLY = "Board updated successfully!";
export const FOLDER_UPDATE_FAILED = "Folder update failed!";
export const FOLDER_UPDATED_SUCCESSFULLY = "Folder updated successfully!";
export const WIDGET_UPDATE_FAILED = "Widget update failed!";
export const WIDGET_CREATE_FAILED = "Widget create failed!";
export const WIDGET_UPDATED_SUCCESSFULLY = "Widget updated successfully!";
export const BOARD_LAYOUT_UPDATED_SUCCESSFULLY =
	"Board layout updated successfully!";
export const WIDGET_FETCH_FAILED = "Widget fetch failed!";
export const WIDGET_RUN_FAILED = "Widget run failed!";
export const BOARD_DELETE_FAILED = "Board delete failed!";
export const BOARD_DELETED_SUCCESSFULLY = "Board deleted successfully!";
export const FOLDER_DELETE_FAILED =
	"Folder cannot be deleted! It has boards or folders inside it.";
export const FOLDER_DELETED_SUCCESSFULLY = "Folder deleted successfully!";
export const MAIN_DASHBOARD_NOT_FOUND = "Main dashboard not found!";
export const BOARD_CREATE_FAILED = "Board create failed!";
export const BOARD_IMPORT_FAILED = "Board import failed!";
export const BOARD_IMPORT_SUCCESSFULLY = "Board import successfully!";
export const NO_WIDGETS_YET = "No widgets yet!";
export const NO_WIDGETS_YET_DESCRIPTION = "Create your first widget to start building your custom dashboard. Add charts, stats, and more to visualize your data.";
export const NO_WIDGETS_YET_ACTION_BUTTON = "Add Your First Widget";
export const NO_DASHBOARDS_YET = "No Dashboards Yet";
export const NO_DASHBOARDS_YET_DESCRIPTION = "Create your first dashboard to start visualizing your data in a meaningful way.";
export const NO_DASHBOARDS_YET_ACTION_BUTTON = "Create Dashboard";
export const NO_DASHBOARDS_YET_SEARCH_TITLE = "No Dashboards Found";
export const NO_DASHBOARDS_YET_SEARCH_DESCRIPTION = "No dashboards found matching your search.";
export const NO_WIDGETS_YET_SEARCH_TITLE = "No Widgets Found";
export const NO_WIDGETS_YET_SEARCH_DESCRIPTION = "No widgets found matching your search.";
export const ADD_DASHBOARD_OR_FOLDER = "Create a new dashboard or folder";
export const EDIT_DASHBOARD_OR_FOLDER = "Edit dashboard or folder";
export const ERROR_OCCURED = "Broken Dashboard";
export const ERROR_OCCURED_DESCRIPTION = "An error occurred while fetching the dashboard or the dashboard does not exist. Please try again later.";

// Openground
export const OPENGROUND_MIGRATION_FAILED = "Openground migration failed!";
export const OPENGROUND_CREATE_FAILED = "Failed to create Openground evaluation!";
export const OPENGROUND_FETCH_FAILED = "Failed to fetch Openground evaluation!";
export const OPENGROUND_DELETE_FAILED = "Failed to delete Openground evaluation!";
export const OPENGROUND_DATA_MIGRATION_FAILED = "Failed to migrate Openground data from Prisma to ClickHouse!";
export const OPENGROUND_RUN_DETAILS = "Run Details";
export const OPENGROUND_PROVIDER_RESPONSE = "Provider Response";
export const OPENGROUND_PROVIDER_RESPONSES = "Provider Responses";
export const OPENGROUND_SELECT_PROVIDERS = "Select Providers to Compare";
export const OPENGROUND_SELECT_PROVIDER_ERROR = "Please select at least one provider";
export const OPENGROUND_ENTER_PROMPT_ERROR = "Please enter a prompt";
export const OPENGROUND_FILL_VARIABLES_ERROR = "Please fill in all variables";
export const OPENGROUND_EVALUATION_SUCCESS = "Evaluation completed successfully!";
export const OPENGROUND_EVALUATION_FAILED = "Evaluation failed";
export const OPENGROUND_RESET_SUCCESS = "Reset complete. Ready for a new evaluation.";
export const OPENGROUND_EVALUATION_LOADED = "Evaluation loaded. Configure providers and run again.";
export const OPENGROUND_SELECT_PROVIDERS_BEGIN = "Select providers above to begin";
export const OPENGROUND_EVALUATION_COMPLETE = "Evaluation complete";
export const OPENGROUND_READY_TO_EVALUATE = "Ready to evaluate";
export const OPENGROUND_PROVIDER_ADDED = "added. Configure model/settings below.";
export const OPENGROUND_LOAD_CONFIG_FAILED = "Failed to load provider configurations";
export const OPENGROUND_LOAD_PROMPTS_FAILED = "Failed to load prompts from Prompt Hub";
export const OPENGROUND_LOAD_PROMPT_DETAILS_FAILED = "Failed to load prompt details";
export const OPENGROUND_LOAD_VAULT_KEYS_FAILED = "Failed to load API keys from Vault";
export const OPENGROUND_SELECT_API_KEY_ERROR = "Please select an API key from Vault";
export const OPENGROUND_SAVE_CONFIG_FAILED = "Failed to save configuration";
export const OPENGROUND_CONFIG_SAVED = "Configuration saved successfully!";
export const OPENGROUND_CONFIG_UPDATED = "Configuration updated successfully!";
export const OPENGROUND_SELECT_API_KEY = "Select an API key";
export const OPENGROUND_SELECT_DEFAULT_MODEL = "Select a default model";
export const OPENGROUND_SELECT_PROMPT = "Select a prompt from Prompt Hub";
export const OPENGROUND_SAVE_CONFIGURATION = "Save Configuration";
export const OPENGROUND_UPDATE_CONFIGURATION = "Update Configuration";
export const OPENGROUND_EVALUATING = "Evaluating...";
export const OPENGROUND_EVALUATING_PROVIDERS = "Evaluating providers...";
export const OPENGROUND_EVALUATE_PROVIDERS = "Evaluate providers";
export const OPENGROUND_MAY_TAKE_FEW_SECONDS = "This may take a few seconds";
export const OPENGROUND_CREATE_NEW_PLAYGROUND = "Create New Playground";
export const OPENGROUND_FASTEST_RESPONSE = "Fastest Response";
export const OPENGROUND_LOWEST_COST = "Lowest Cost";
export const OPENGROUND_MOST_EFFICIENT = "Most Efficient";
export const OPENGROUND_SUCCESS_RATE = "Success Rate";
export const OPENGROUND_PROMPT_CONFIGURATION = "Prompt Configuration";
export const OPENGROUND_CUSTOM = "Custom";
export const OPENGROUND_PROMPT_HUB = "Prompt Hub";
export const OPENGROUND_ENTER_PROMPT_PLACEHOLDER = "Enter your prompt... Use {{variable}} for dynamic values";
export const OPENGROUND_NO_PROMPTS_FOUND = "No prompts found in Prompt Hub";
export const OPENGROUND_LOADING_PROMPT_DETAILS = "Loading prompt details...";
export const OPENGROUND_RAW_RESPONSE_DATA = "Raw Response Data";
export const OPENGROUND_RESPONSE_TIME_COMPARISON = "Response Time Comparison";
export const OPENGROUND_RESPONSE_TIME_COMPARISON_DESCRIPTION = "Visual comparison of provider response times";
export const OPENGROUND_COST_BREAKDOWN = "Cost Breakdown";
export const OPENGROUND_COST_BREAKDOWN_DESCRIPTION = "Detailed cost analysis per provider";
export const OPENGROUND_CLICK_PROVIDER_CARD_TO_CHANGE_MODEL_OR_SETTINGS = "Click a provider card to change model or settings";
export const OPENGROUND_UPDATE = "Update";
export const OPENGROUND_LINK_PROVIDER_TO_VAULT_DESCRIPTION = "Link this provider to an API key from your Vault and optionally set a default model";
export const OPENGROUND_API_KEY_FROM_VAULT = "API Key (from Vault)";
export const OPENGROUND_LOADING_SECRETS = "Loading secrets...";
export const OPENGROUND_NO_API_KEYS_FOUND_IN_VAULT = "No API keys found in Vault.";
export const OPENGROUND_CREATE_NEW_API_KEY = "Create New API Key";
export const OPENGROUND_API_KEY_STORED_IN_VAULT = "API key stored in Vault";
export const OPENGROUND_DEFAULT_MODEL_OPTIONAL = "Default Model (Optional)";
export const OPENGROUND_YOU_CAN_CHANGE_PER_EVALUATION = "You can change this per evaluation";
export const OPENGROUND_API_KEY_REFERENCE_SAVED_INFO = "The API key reference is saved in ClickHouse. The actual key remains encrypted in Vault.";
export const OPENGROUND_PROVIDER_SETTINGS = "Provider Settings";
export const OPENGROUND_MODEL = "Model";
export const OPENGROUND_TEMPERATURE = "Temperature";
export const OPENGROUND_TEMPERATURE_DESCRIPTION = "Controls randomness: lower is more focused, higher is more creative";
export const OPENGROUND_MAX_TOKENS = "Max Tokens";
export const OPENGROUND_MAX_TOKENS_DESCRIPTION = "Maximum length of the response";
export const OPENGROUND_TOP_P = "Top P";
export const OPENGROUND_TOP_P_DESCRIPTION = "Nucleus sampling: controls diversity of word choice";
export const OPENGROUND_ENTER_VALUE_FOR = "Enter value for";
export const OPENGROUND_VARIABLES_SUBSTITUTED_INFO = "These values will be substituted into your prompt before evaluation";
export const OPENGROUND_CUSTOM_MODEL = "Custom Model";
export const OPENGROUND_ENTER_CUSTOM_MODEL_NAME = "Enter custom model name";
export const OPENGROUND_USE_CUSTOM_MODEL = "Use Custom Model";
export const OPENGROUND_OR_ENTER_CUSTOM = "or enter custom";
export const OPENGROUND_MANAGE_MODELS = "Manage Models";
export const OPENGROUND_ADD_NEW_MODEL = "Add New Model";
export const OPENGROUND_EDIT_MODEL = "Edit Model";
export const OPENGROUND_MODEL_ID = "Model ID";
export const OPENGROUND_MODEL_DISPLAY_NAME = "Display Name";
export const OPENGROUND_CONTEXT_WINDOW = "Context Window";
export const OPENGROUND_INPUT_PRICE_PER_M_TOKENS = "Input Price (per 1M tokens)";
export const OPENGROUND_OUTPUT_PRICE_PER_M_TOKENS = "Output Price (per 1M tokens)";
export const OPENGROUND_MODEL_CAPABILITIES = "Capabilities (comma separated)";
export const OPENGROUND_SAVE_MODEL = "Save Model";
export const OPENGROUND_MODEL_SAVED_SUCCESS = "Model saved successfully!";
export const OPENGROUND_MODEL_DELETED_SUCCESS = "Model deleted successfully!";
export const OPENGROUND_DELETE_MODEL = "Delete Model";
export const OPENGROUND_DELETE_MODEL_CONFIRMATION = "Are you sure you want to delete this model?";
export const OPENGROUND_NO_CUSTOM_MODELS_YET = "No custom models added yet. Add one to get started.";
export const OPENGROUND_MANAGE_MODELS_DESCRIPTION = "View and manage custom models for all providers. Add new models or clone existing ones with custom pricing.";
export const OPENGROUND_SELECT_MODEL_TO_VIEW = "Select a model to view details";
export const OPENGROUND_SELECT_MODEL_TO_VIEW_DESCRIPTION = "Choose a model from the sidebar to view its details, or add a new custom model for any provider.";
export const OPENGROUND_STATIC_MODEL = "Static Model";
export const OPENGROUND_STATIC_MODEL_DESCRIPTION = "This is a built-in model. Clone it to create a custom version with your own pricing.";
export const OPENGROUND_CLONE_MODEL = "Clone Model";
export const OPENGROUND_MODEL_DETAILS = "Model Details";
export const OPENGROUND_ADD_CUSTOM_MODEL = "Add Custom Model";
export const OPENGROUND_EDIT_CUSTOM_MODEL = "Edit Custom Model";
export const OPENGROUND_ALL_PROVIDERS = "All Providers";
export const OPENGROUND_SEARCH_MODELS = "Search models...";
export const OPENGROUND_NO_MODELS_FOUND = "No models found";
export const OPENGROUND_CUSTOM_MODELS = "Custom Models";

// Features Title
export const FEATURE_OPENGROUND = "Openground";
export const FEATURE_PROMPTS = "Prompt Hub";
export const FEATURE_VAULT = "Vault";
export const FEATURE_FLEET_HUB = "Fleet Hub";


// Getting Started - Openground
export const GET_STARTED_WITH_OPENGROUND = "Get Started with Openground";
export const GET_STARTED_WITH_OPENGROUND_DESCRIPTION = "Experiment and test different LLM configurations, prompts, and parameters. Compare outputs side-by-side to find the optimal setup for your use case.";
export const GET_STARTED_WITH_OPENGROUND_ACTION_BUTTON = "Create New Playground";
export const GET_STARTED_WITH_OPENGROUND_FEATURE_DETAILS = [
	{
		icon: "🔬",
		title: "Test Configurations",
		description: "Experiment with different LLM models, parameters, and settings to find the best configuration for your specific use case.",
	},
	{
		icon: "🔄",
		title: "Compare Results",
		description: "View outputs from different configurations side-by-side to make informed decisions about your LLM setup.",
	},
	{
		icon: "📝",
		title: "Prompt Testing",
		description: "Test and refine your prompts iteratively to achieve better results and more accurate AI responses.",
	},
	{
		icon: "⚡",
		title: "Quick Iteration",
		description: "Rapidly test multiple variations to optimize your LLM application before deploying to production.",
	}
];

// Getting Started - PromptHub
export const GET_STARTED_WITH_PROMPT_HUB = "Get Started with Prompt Hub";
export const GET_STARTED_WITH_PROMPT_HUB_DESCRIPTION = "Centralized prompt management system to version, deploy, and collaborate on prompts. Track prompt usage, manage variables, and easily retrieve prompts across your applications.";
export const GET_STARTED_WITH_PROMPT_HUB_ACTION_BUTTON = "Create New Prompt";
export const GET_STARTED_WITH_PROMPT_HUB_FEATURE_DETAILS = [
	{
		icon: "📝",
		title: "Version Control",
		description: "Track and manage different versions of your prompts with complete version history and rollback capabilities.",
	},
	{
		icon: "🔄",
		title: "Variable Support",
		description: "Create dynamic prompts with variable placeholders for flexible and reusable prompt templates.",
	},
	{
		icon: "👥",
		title: "Team Collaboration",
		description: "Collaborate with your team on prompt development and track who created and modified prompts.",
	},
	{
		icon: "📊",
		title: "Usage Tracking",
		description: "Monitor prompt downloads and usage across your applications to understand which prompts are most valuable.",
	}
];

// Getting Started - Vault
export const GET_STARTED_WITH_VAULT = "Get Started with Vault";
export const GET_STARTED_WITH_VAULT_DESCRIPTION = "Centralized secret management system to store, retrieve, and manage your secrets. Track secret usage, manage variables, and easily retrieve secrets across your applications.";
export const GET_STARTED_WITH_VAULT_ACTION_BUTTON = "Create New Secret";
export const GET_STARTED_WITH_VAULT_FEATURE_DETAILS = [
	{
		icon: "🔒",
		title: "Secure Storage",
		description: "Store LLM API keys and sensitive credentials securely with encryption and access controls.",
	},
	{
		icon: "🔑",
		title: "API Access",
		description: "Access your secrets through authenticated API endpoints for seamless integration with your applications.",
	},
	{
		icon: "👤",
		title: "User Tracking",
		description: "Track who created and updated each secret for complete accountability and transparency.",
	},
	{
		icon: "⏰",
		title: "Update History",
		description: "Monitor when secrets were last updated to ensure your credentials remain current and secure.",
	}
];

// Getting Started - Tracing
export const GET_STARTED_WITH_TRACING = "Get Started with Observability";
export const GET_STARTED_WITH_TRACING_DESCRIPTION = "OpenTelemetry-native auto-instrumentation to trace LLMs, agents, vector databases and GPUs with zero-code. Visualize application flow, identify performance bottlenecks, and track errors with detailed stack traces.";
export const GET_STARTED_WITH_TRACING_FEATURE_DETAILS = [
	{
		icon: "🔍",
		title: "Zero-Code Instrumentation",
		description: "Automatically trace LLMs, agents, frameworks, vector databases, and GPUs without modifying your existing code.",
	},
	{
		icon: "⚡",
		title: "Exception Tracking",
		description: "Track and debug application errors with detailed stack traces to rapidly identify and resolve issues.",
	},
	{
		icon: "🔄",
		title: "OpenTelemetry Compatible",
		description: "Full OpenTelemetry compatibility allows you to view traces from any OpenTelemetry-instrumented tool in your stack.",
	},
	{
		icon: "📊",
		title: "Performance Insights",
		description: "Visualize application flow and identify performance bottlenecks to optimize your GenAI and LLM applications.",
	}
];

// Generic texts
export const LOADING = "Loading";
export const CREATED_AT = "Created At";
export const VARIABLES = "Variables";
export const PROMPT = "Prompt";
export const PROMPT_PREVIEW = "Prompt Preview";
export const PROMPT_HUB = "Prompt Hub";
export const PROVIDERS = "Providers";
export const NO_DATA_FOUND = "No data found!";
export const CANNOT_CONNECT_TO_SERVER = "Cannot connect to server!";
export const PLEASE_SELECT = "Please select";
export const SELECT = "Select";
export const SAVING = "Saving...";
export const UPDATING = "Updating...";
export const CONFIGURATION_STORED_SECURELY = "Configuration stored securely";
export const BEST_EFFICIENCY = "Best Efficiency";
export const SOME_ERROR_OCCURRED = "Some error occurred while performing the operation";
export const RESET = "Reset";
export const ERROR = "Error";
export const SUCCESS = "Success";
export const HIDE = "Hide";
export const SHOW = "Show";
export const FASTEST = "Fastest";
export const AVERAGE = "Average";
export const SLOWEST = "Slowest";
export const CHEAPEST = "Cheapest";
export const COMPLETION = "Completion";
export const TOTAL = "Total";
export const COST = "Cost";
export const TOKENS = "Tokens";
export const CONFIGURE = "Configure";
export const CONFIGURED = "Configured";
export const SELECTED = "Selected";
export const CANCEL = "Cancel";
export const DELETE = "Delete";
export const CLOSE = "Close";
export const ACTIONS = "Actions";