import type { Meta, StoryObj } from "@storybook/react";
import { VibesPanel } from "./VibesPanel";

const meta: Meta<typeof VibesPanel> = {
  title: "Components/VibesPanel",
  component: VibesPanel,
  parameters: {
    layout: "centered",
    backgrounds: {
      default: "dark",
      values: [
        { name: "dark", value: "#1a1a1a" },
        { name: "light", value: "#f5f5f4" },
      ],
    },
  },
  tags: ["autodocs"],
  argTypes: {
    baseURL: {
      control: "text",
      description: "Base URL for vibes platform",
    },
    token: {
      control: "text",
      description: "Authentication token for sharing functionality",
    },
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
};
