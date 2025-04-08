document.addEventListener('DOMContentLoaded', () => {
    // Function to show error message
    function showError(message) {
        const resultDiv = document.getElementById('generatedLink');
        resultDiv.innerHTML = `<div class="error">${message}</div>`;
        resultDiv.parentElement.style.display = 'block';
    }

    // Function to handle generated link (display or redirect)
    function handleGeneratedLink(link, shouldRedirect = false) {
        if (shouldRedirect) {
            window.location.href = link;
        } else {
            displayGeneratedLink(link);
        }
    }

    // Process URL parameters if present
    const urlParams = new URLSearchParams(window.location.search);
    if (urlParams.toString()) {
        try {
            // Check if this is a built-in extension request
            if (urlParams.get('cmd') === 'goosed' && urlParams.getAll('arg').includes('mcp')) {
                const args = urlParams.getAll('arg');
                const extensionId = args[args.indexOf('mcp') + 1];
                if (!extensionId) {
                    throw new Error('Missing extension ID in args');
                }

                const server = {
                    is_builtin: true,
                    id: extensionId
                };
                const link = generateInstallLink(server);
                handleGeneratedLink(link, true);
                return;
            }
            
            // Handle custom extension
            const cmd = urlParams.get('cmd');
            if (!cmd) {
                throw new Error('Missing required parameter: cmd');
            }

            const args = urlParams.getAll('arg') || [];
            const command = [cmd, ...args].join(' ');
            const id = urlParams.get('id');
            const name = urlParams.get('name');
            const description = urlParams.get('description');

            if (!id || !name || !description) {
                throw new Error('Missing required parameters. Need: id, name, and description');
            }

            const server = {
                is_builtin: false,
                id,
                name,
                description,
                command,
                environmentVariables: []
            };

            // Handle environment variables if present
            const envVars = urlParams.getAll('env');
            if (envVars.length > 0) {
                envVars.forEach(env => {
                    const [name, description] = env.split('=');
                    if (name && description) {
                        server.environmentVariables.push({
                            name,
                            description,
                            required: true
                        });
                    }
                });
            }

            const link = generateInstallLink(server);
            handleGeneratedLink(link, true);
        } catch (error) {
            showError(error.message);
            document.querySelector('.container').style.display = 'block';
        }
    } else {
        // Show the form if no parameters
        document.querySelector('.container').style.display = 'block';
    }

    // Tab switching
    const tabs = document.querySelectorAll('.tab-btn');
    tabs.forEach(tab => {
        tab.addEventListener('click', () => {
            tabs.forEach(t => t.classList.remove('active'));
            tab.classList.add('active');
            
            document.querySelectorAll('.tab-content').forEach(content => {
                content.classList.remove('active');
            });
            document.getElementById(tab.dataset.tab).classList.add('active');
        });
    });

    // Handle built-in checkbox
    const isBuiltinCheckbox = document.getElementById('isBuiltin');
    const nonBuiltinFields = document.querySelector('.non-builtin');

    isBuiltinCheckbox.addEventListener('change', () => {
        nonBuiltinFields.style.display = isBuiltinCheckbox.checked ? 'none' : 'block';
    });

    // Environment variables handling
    const envVarsContainer = document.getElementById('envVars');
    const addEnvVarBtn = document.getElementById('addEnvVar');

    function createEnvVarInputs() {
        const envVarDiv = document.createElement('div');
        envVarDiv.className = 'env-var';

        const nameInput = document.createElement('input');
        nameInput.type = 'text';
        nameInput.placeholder = 'Variable Name';
        nameInput.className = 'env-name';

        const descInput = document.createElement('input');
        descInput.type = 'text';
        descInput.placeholder = 'Description';
        descInput.className = 'env-desc';

        const removeBtn = document.createElement('button');
        removeBtn.type = 'button';
        removeBtn.textContent = 'Remove';
        removeBtn.onclick = () => envVarDiv.remove();

        envVarDiv.appendChild(nameInput);
        envVarDiv.appendChild(descInput);
        envVarDiv.appendChild(removeBtn);

        return envVarDiv;
    }

    addEnvVarBtn.addEventListener('click', () => {
        envVarsContainer.appendChild(createEnvVarInputs());
    });

    // Generate link from form
    document.getElementById('installForm').addEventListener('submit', (e) => {
        e.preventDefault();
        const formData = new FormData(e.target);
        const server = {
            is_builtin: formData.get('is_builtin') === 'on',
            id: formData.get('id'),
            name: formData.get('name'),
            description: formData.get('description'),
            command: formData.get('command'),
            environmentVariables: []
        };

        // Collect environment variables
        document.querySelectorAll('.env-var').forEach(envVar => {
            const name = envVar.querySelector('.env-name').value;
            const description = envVar.querySelector('.env-desc').value;
            if (name && description) {
                server.environmentVariables.push({
                    name,
                    description,
                    required: true
                });
            }
        });

        const link = generateInstallLink(server);
        handleGeneratedLink(link);
    });

    // Generate link from JSON
    document.getElementById('generateFromJson').addEventListener('click', () => {
        try {
            const jsonInput = document.getElementById('jsonInput').value;
            const server = JSON.parse(jsonInput);
            const link = generateInstallLink(server);
            handleGeneratedLink(link);
        } catch (error) {
            showError('Invalid JSON: ' + error.message);
        }
    });

    // Link generation logic
    function generateInstallLink(server) {
        if (server.is_builtin) {
            const queryParams = [
                'cmd=goosed',
                'arg=mcp',
                `arg=${encodeURIComponent(server.id)}`,
                `description=${encodeURIComponent(server.id)}`
            ].join('&');
            return `goose://extension?${queryParams}`;
        }

        const parts = server.command.split(" ");
        const baseCmd = parts[0];
        const args = parts.slice(1);
        const queryParams = [
            `cmd=${encodeURIComponent(baseCmd)}`,
            ...args.map((arg) => `arg=${encodeURIComponent(arg)}`),
            `id=${encodeURIComponent(server.id)}`,
            `name=${encodeURIComponent(server.name)}`,
            `description=${encodeURIComponent(server.description)}`,
            ...server.environmentVariables
                .filter((env) => env.required)
                .map(
                    (env) => `env=${encodeURIComponent(`${env.name}=${env.description}`)}`
                ),
        ].join("&");

        return `goose://extension?${queryParams}`;
    }

    function displayGeneratedLink(link) {
        const linkElement = document.getElementById('generatedLink');
        linkElement.textContent = link;
        linkElement.parentElement.style.display = 'block';
    }

    // Add sample JSON to the textarea
    const sampleJson = {
        is_builtin: false,
        id: "example-extension",
        name: "Example Extension",
        description: "An example Goose extension",
        command: "npx @gooseai/example-extension",
        environmentVariables: [
            {
                name: "API_KEY",
                description: "Your API key",
                required: true
            }
        ]
    };
    document.getElementById('jsonInput').value = JSON.stringify(sampleJson, null, 2);
});