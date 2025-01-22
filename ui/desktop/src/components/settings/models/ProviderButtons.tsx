import React from 'react';
import { Button } from "../../ui/button"

// TODO: models -- update this with correct links and providers
const providers = [
    { name: "OpenAI", href: "https://platform.openai.com/docs/models" },
    { name: "Anthropic", href: "https://www.anthropic.com/models" },
    { name: "Google", href: "https://cloud.google.com/vertex-ai" },
    { name: "Mistral", href: "https://mistral.ai/models" },
    { name: "Amazon", href: "https://aws.amazon.com/bedrock/models" },
    { name: "Azure", href: "https://azure.microsoft.com/en-us/products/cognitive-services/openai-service" },
]

export function ProviderButtons() {
    return (
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
            {providers.map((provider) => (
                <Button
                    key={provider.name}
                    variant="outline"
                    className="h-20 border-muted-foreground/20"
                    onClick={() => window.open(provider.href, '_blank')}
                >
                    {provider.name}
                </Button>
            ))}
        </div>
    )
}

