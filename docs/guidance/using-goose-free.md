# Using Goose for Free

Goose is a free and open-source developer agent that you can start using right away, but not all supported [LLM Providers][providers] provide a free tier. 

Below, we outline a couple of free options and how to get started with them.


## Google Gemini
Google Gemini provides free access to its AI capabilities with some limitations. To start using the Gemini API with Goose, you need an API Key from [Google AI studio](https://aistudio.google.com/app/apikey).

Update your `~/.config/goose/profiles.yaml` file with the following configuration:

```yaml title="profiles.yaml"
default:
  provider: google
  processor: gemini-1.5-flash
  accelerator: gemini-1.5-flash
  moderator: passive
  toolkits:
  - name: developer
    requires: {}
```

When you run `goose session start`, you will be prompted to enter your Google API Key.

> [!NOTE] 
> At the moment, the `synopsis` toolkit isn't supported by Google Gemini, so we use the `developer` toolkit to interact with the API. 





## Limitations

These free options are a great way to get started with Goose and explore its capabilities. However, if you need more advanced features or higher usage limits, you can always upgrade to a paid plan.

---

This guide will continue to be updated with more free options as they become available. If you have any questions or need help with a specific provider, feel free to reach out to us on [Discord](https://discord.gg/block-opensource) or on the [Goose repo](https://github.com/block/goose).


[providers]: https://block.github.io/goose/plugins/providers.html