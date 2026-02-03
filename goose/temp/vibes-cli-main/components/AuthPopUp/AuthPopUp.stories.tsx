import type { Meta, StoryObj } from "@storybook/react";
import { useState } from "react";
import { AuthPopUp } from "./AuthPopUp";
import { VibesButton, BLUE } from "../VibesButton/VibesButton";

const meta = {
  title: "Components/AuthPopUp",
  component: AuthPopUp,
  parameters: {
    layout: "fullscreen",
    docs: {
      story: {
        inline: false,
        iframeHeight: 700,
      },
    },
  },
  tags: ["autodocs"],
  argTypes: {
    isOpen: {
      control: "boolean",
      description: "Whether the popup is open",
    },
    appName: {
      control: "text",
      description: "Name of the app to install",
    },
  },
} satisfies Meta<typeof AuthPopUp>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {} as any,
  render: () => {
    const [isOpen, setIsOpen] = useState(true);
    return (
      <div style={{ height: "100vh", background: "var(--vibes-bg-secondary)" }}>
        <div style={{ padding: "1rem" }}>
          <VibesButton variant={BLUE} onClick={() => setIsOpen(true)}>
            Open Auth PopUp
          </VibesButton>
        </div>
        <AuthPopUp
          isOpen={isOpen}
          onClose={() => setIsOpen(false)}
          appName="303 Synth App"
        />
      </div>
    );
  },
};

export const CustomAppName: Story = {
  args: {} as any,
  render: () => {
    const [isOpen, setIsOpen] = useState(true);
    return (
      <div style={{ height: "100vh", background: "var(--vibes-bg-secondary)" }}>
        <div style={{ padding: "1rem" }}>
          <VibesButton variant={BLUE} onClick={() => setIsOpen(true)}>
            Open Auth PopUp
          </VibesButton>
        </div>
        <AuthPopUp
          isOpen={isOpen}
          onClose={() => setIsOpen(false)}
          appName="My Todo List"
        />
      </div>
    );
  },
};

export const Closed: Story = {
  args: {} as any,
  render: () => {
    const [isOpen, setIsOpen] = useState(false);
    return (
      <div style={{ height: "100vh", background: "var(--vibes-bg-secondary)", padding: "1rem" }}>
        <VibesButton variant={BLUE} onClick={() => setIsOpen(true)}>
          Open Auth PopUp
        </VibesButton>
        <AuthPopUp
          isOpen={isOpen}
          onClose={() => setIsOpen(false)}
          appName="Vibes App"
        />
      </div>
    );
  },
};
