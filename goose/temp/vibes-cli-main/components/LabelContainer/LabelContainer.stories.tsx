import type { Meta, StoryObj } from "@storybook/react";
import { LabelContainer } from "./LabelContainer";
import { VibesButton } from "../VibesButton/VibesButton";

const meta = {
  title: "Components/LabelContainer",
  component: LabelContainer,
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
  argTypes: {
    label: {
      control: "text",
      description: "The label text to display on the side of the container",
    },
    disappear: {
      control: "boolean",
      description: "If true, label disappears on mobile. If false, label moves to top on mobile.",
    },
  },
} satisfies Meta<typeof LabelContainer>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    label: "Actions",
    children: (
      <div style={{ display: "flex", gap: "1rem" }}>
        <VibesButton variant="blue">Button 1</VibesButton>
        <VibesButton variant="red">Button 2</VibesButton>
      </div>
    ),
  },
};

export const WithLabel: Story = {
  args: {
    label: "Menu",
    children: (
      <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap" }}>
        <VibesButton variant="blue" icon="login">Login</VibesButton>
        <VibesButton variant="red" icon="remix">Remix</VibesButton>
        <VibesButton variant="yellow" icon="invite">Invite</VibesButton>
        <VibesButton variant="gray" icon="settings">Settings</VibesButton>
      </div>
    ),
  },
};

export const DisappearOnMobile: Story = {
  args: {
    label: "Launcher",
    disappear: true,
    children: (
      <div style={{ display: "flex", gap: "1rem" }}>
        <VibesButton variant="blue">Action 1</VibesButton>
        <VibesButton variant="yellow">Action 2</VibesButton>
      </div>
    ),
  },
};

export const WithoutLabel: Story = {
  args: {
    children: (
      <div style={{ display: "flex", gap: "1rem" }}>
        <VibesButton variant="blue">No Label</VibesButton>
        <VibesButton variant="red">Container</VibesButton>
      </div>
    ),
  },
};

export const SimpleContent: Story = {
  args: {
    label: "Info",
    children: (
      <div style={{ padding: "1rem", textAlign: "center" }}>
        <h3 style={{ margin: "0 0 0.5rem 0" }}>Welcome</h3>
        <p style={{ margin: 0 }}>This is a simple content example.</p>
      </div>
    ),
  },
};

export const FormContent: Story = {
  args: {
    label: "Form",
    children: (
      <form style={{ display: "flex", flexDirection: "column", gap: "1rem", padding: "1rem" }}>
        <input
          type="email"
          placeholder="Enter your email"
          style={{
            padding: "0.5rem",
            border: "2px solid var(--vibes-card-border)",
            borderRadius: "8px",
          }}
        />
        <VibesButton variant="blue">Submit</VibesButton>
      </form>
    ),
  },
};

export const AllLabels: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", gap: "2rem" }}>
      <LabelContainer label="First">
        <div style={{ padding: "1rem" }}>Content for first container</div>
      </LabelContainer>
      <LabelContainer label="Second">
        <div style={{ padding: "1rem" }}>Content for second container</div>
      </LabelContainer>
      <LabelContainer label="Third">
        <div style={{ padding: "1rem" }}>Content for third container</div>
      </LabelContainer>
    </div>
  ),
};
