import type { Preview } from "@storybook/react";
import "../styles/colors.css";

const preview: Preview = {
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    backgrounds: {
      default: "light",
      values: [
        { name: "light", value: "#f5f5f4" },
        { name: "dark", value: "#1a1a1a" },
      ],
    },
  },
};

export default preview;
