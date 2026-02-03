import type { Meta, StoryObj } from "@storybook/react";
import { fn } from "@storybook/test";
import { useState } from "react";
import { VibesSwitch } from "./VibesSwitch";

const meta: Meta<typeof VibesSwitch> = {
  title: "Components/VibesSwitch",
  component: VibesSwitch,
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
  argTypes: {
    size: {
      control: { type: "number", min: 16, max: 120, step: 4 },
      description: "Size of the switch in pixels",
    },
    isActive: {
      control: "boolean",
      description: "Controlled active state",
    },
    className: {
      control: "text",
      description: "Additional CSS class name",
    },
  },
  args: {
    onToggle: fn(),
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    size: 80,
  },
};

export const Small: Story = {
  args: {
    size: 24,
  },
};

export const Medium: Story = {
  args: {
    size: 48,
  },
};

export const Large: Story = {
  args: {
    size: 80,
  },
};

export const ExtraLarge: Story = {
  args: {
    size: 120,
  },
};

export const Active: Story = {
  args: {
    size: 80,
    isActive: true,
  },
};

export const Inactive: Story = {
  args: {
    size: 80,
    isActive: false,
  },
};

export const Controlled: Story = {
  render: function Render() {
    const [isActive, setIsActive] = useState(false);
    return (
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: "1rem" }}>
        <VibesSwitch size={80} isActive={isActive} onToggle={setIsActive} />
        <span style={{ fontFamily: "monospace" }}>
          State: {isActive ? "Active" : "Inactive"}
        </span>
      </div>
    );
  },
};

export const AllSizes: Story = {
  render: () => (
    <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
      <VibesSwitch size={24} />
      <VibesSwitch size={48} />
      <VibesSwitch size={80} />
      <VibesSwitch size={120} />
    </div>
  ),
};
