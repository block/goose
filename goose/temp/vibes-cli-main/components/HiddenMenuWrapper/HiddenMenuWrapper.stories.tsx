import type { Meta, StoryObj } from "@storybook/react";
import { HiddenMenuWrapper } from "./HiddenMenuWrapper";
import { VibesPanel } from "../VibesPanel/VibesPanel";

const meta = {
  title: "Components/HiddenMenuWrapper",
  component: HiddenMenuWrapper,
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component: `
A wrapper component with a hidden menu at the bottom.

**HOW TO USE:**
- Look for the Vibes pill button in the bottom-right corner
- Click the pill to toggle the hidden menu
- The menu slides up from the bottom with smooth animation

**Note:** The preview below shows the full component. Click the pill in the bottom-right!
        `,
      },
      story: {
        inline: false,
        iframeHeight: 500,
      },
    },
  },
  tags: ["autodocs"],
  argTypes: {
    children: {
      description: "Main content to display",
      control: false,
    },
    menuContent: {
      description: "Content to display in the hidden menu",
      control: false,
    },
    triggerBounce: {
      control: "boolean",
      description: "Trigger bounce animation",
    },
    showVibesSwitch: {
      control: "boolean",
      description: "Show the VibesSwitch toggle button",
    },
  },
} satisfies Meta<typeof HiddenMenuWrapper>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {} as any,
  parameters: {
    docs: {
      story: {
        inline: false,
        iframeHeight: 600,
      },
    },
  },
  render: () => (
    <HiddenMenuWrapper
      showVibesSwitch={true}
      triggerBounce={false}
      menuContent={<VibesPanel />}
    >
      <div
        style={{
          width: "100%",
          height: "100vh",
          backgroundColor: "#000000",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          color: "#ffffff",
          fontSize: "1.5rem",
          fontFamily: "system-ui, sans-serif",
        }}
      >
        Click the switch below to reveal the menu
      </div>
    </HiddenMenuWrapper>
  ),
};
