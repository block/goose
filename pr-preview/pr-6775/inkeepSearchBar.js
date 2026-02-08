
    const inkeepSearchBarScript = document.createElement("script");
    inkeepSearchBarScript.type = "module";
    inkeepSearchBarScript.src = 'https://unpkg.com/@inkeep/uikit-js@0.3.19/dist/embed.js';
    inkeepSearchBarScript.integrity = 'sha384-NrApcNv8E5NXyoaHq8Zbyi9byJkCkCJ7BZJRlZ+8ELzfp0qgixQYy4FXfkJcVkn3';
    inkeepSearchBarScript.crossOrigin = "anonymous";
    document.body.appendChild(inkeepSearchBarScript);
    inkeepSearchBarScript.addEventListener("load", function () {
      const inkeepConfigSearchBar = {"baseSettings":{"apiKey":"5bcaf7108a8d61a3ea37ee6c7d592f3d1fe76d091aadda6c","integrationId":"cm6b6mj8v00sjs601v6u4rjnf","organizationId":"org_GW7VeRk82JIdk426","primaryBrandColor":"#1E1E1E"},"aiChatSettings":{"chatSubjectName":"goose","botAvatarSrcUrl":"","getHelpCallToActions":[{"name":"GitHub","url":"https://github.com/block/goose","icon":{"builtIn":"FaGithub"}}],"quickQuestions":["What is goose?"]}};
      (function listenerFunction({ inkeepConfig, componentType }) {
  if (!inkeepConfig) {
    throw new Error(
      "Configuration Error: inkeepConfig is missing in the Docusaurus configuration file."
    );
  }

  let inkeepWidget = null;
  const isChatButtonType = componentType === "ChatButton";

  const renderWidgets = () => {
    const inkeepWidgetContainer = isChatButtonType ? undefined : document.getElementById("inkeepSearchBar");

    if (isChatButtonType) {
      const backToTopButtonOffset =
        inkeepConfig.chatButtonType === "RECTANGLE_SHORTCUT"
          ? "6.8rem"
          : "5.4rem";
      const backToTopButton = document.querySelector(
        ".theme-back-to-top-button"
      );
      if (backToTopButton) {
        backToTopButton.style.bottom = backToTopButtonOffset;
      }
    }

    const shouldRender = !inkeepWidget && (isChatButtonType || inkeepWidgetContainer);

    const config = {
      componentType,
      targetElement: inkeepWidgetContainer,
      colorModeSync: {
        observedElement: document.documentElement,
        isDarkModeCallback: (observedElement) =>
          observedElement.dataset.theme === "dark",
        colorModeAttribute: "data-theme",
      },
      properties: {
        ...inkeepConfig,
        baseSettings: {
          ...inkeepConfig.baseSettings,
          theme: {
            ...(inkeepConfig.baseSettings?.theme || {}),
            components: {
              SearchBarTrigger: {
                defaultProps: {
                  size: "shrink",
                },
              },
              ...(inkeepConfig.baseSettings?.theme?.components || {}),
            },
          },
        },
        modalSettings: inkeepConfig.modalSettings,
        searchSettings: inkeepConfig.searchSettings,
        aiChatSettings: inkeepConfig.aiChatSettings,
      },
    };

    if (shouldRender) {
      inkeepWidget = Inkeep().embed(config);
    }
  };

  renderWidgets();

  // not totally sure this is necessary anymore but leaving for now just in case
  const observer = new MutationObserver(() => {
    renderWidgets();
  });

  observer.observe(document.documentElement, { attributes: true });
})({
        inkeepConfig: inkeepConfigSearchBar,
        componentType: 'SearchBar',
      })
    })
  