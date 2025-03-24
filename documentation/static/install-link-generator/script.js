document.addEventListener('DOMContentLoaded', () => {
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
        displayGeneratedLink(link);
    });

    // Generate link from JSON
    document.getElementById('generateFromJson').addEventListener('click', () => {
        try {
            const jsonInput = document.getElementById('jsonInput').value;
            const server = JSON.parse(jsonInput);
            const link = generateInstallLink(server);
            displayGeneratedLink(link);
        } catch (error) {
            alert('Invalid JSON: ' + error.message);
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