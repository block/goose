import type { Meta, StoryObj } from "@storybook/react";
import { BrutalistCard } from "./BrutalistCard";

const meta = {
  title: "Components/BrutalistCard",
  component: BrutalistCard,
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
  argTypes: {
    variant: {
      control: "select",
      options: ["default", "success", "error", "warning"],
      description: "Visual variant affecting shadow color",
    },
    size: {
      control: "select",
      options: ["sm", "md", "lg"],
      description: "Size affecting padding, font size, and shadow size",
    },
    messageType: {
      control: "select",
      options: [undefined, "user", "ai"],
      description: "Message type for chat bubble corner rounding",
    },
  },
} satisfies Meta<typeof BrutalistCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    children: "This is a default brutalist card with clean styling.",
    variant: "default",
    size: "md",
  },
};

export const Small: Story = {
  args: {
    children: "Small card",
    variant: "default",
    size: "sm",
  },
};

export const Medium: Story = {
  args: {
    children: "Medium card with more content to show the padding difference.",
    variant: "default",
    size: "md",
  },
};

export const Large: Story = {
  args: {
    children: "Large card with even more padding for important content.",
    variant: "default",
    size: "lg",
  },
};

export const Success: Story = {
  args: {
    children: "Operation completed successfully!",
    variant: "success",
    size: "md",
  },
};

export const Error: Story = {
  args: {
    children: "Something went wrong. Please try again.",
    variant: "error",
    size: "md",
  },
};

export const Warning: Story = {
  args: {
    children: "Please review before continuing.",
    variant: "warning",
    size: "md",
  },
};

export const UserMessage: Story = {
  args: {
    children: "This is a message from the user with a chat bubble style.",
    variant: "default",
    size: "md",
    messageType: "user",
  },
};

export const AIMessage: Story = {
  args: {
    children: "This is a response from the AI with a chat bubble style.",
    variant: "default",
    size: "md",
    messageType: "ai",
  },
};

export const AllVariants: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      <BrutalistCard variant="default">Default variant</BrutalistCard>
      <BrutalistCard variant="success">Success variant</BrutalistCard>
      <BrutalistCard variant="error">Error variant</BrutalistCard>
      <BrutalistCard variant="warning">Warning variant</BrutalistCard>
    </div>
  ),
};

export const AllSizes: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      <BrutalistCard size="sm">Small size</BrutalistCard>
      <BrutalistCard size="md">Medium size</BrutalistCard>
      <BrutalistCard size="lg">Large size</BrutalistCard>
    </div>
  ),
};

export const ChatBubbles: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", gap: "1rem", maxWidth: "400px" }}>
      <div style={{ alignSelf: "flex-end" }}>
        <BrutalistCard messageType="user" size="sm">
          Hey, how are you?
        </BrutalistCard>
      </div>
      <div style={{ alignSelf: "flex-start" }}>
        <BrutalistCard messageType="ai" size="sm">
          I'm doing great! How can I help you today?
        </BrutalistCard>
      </div>
      <div style={{ alignSelf: "flex-end" }}>
        <BrutalistCard messageType="user" size="sm">
          Can you explain how this component works?
        </BrutalistCard>
      </div>
      <div style={{ alignSelf: "flex-start" }}>
        <BrutalistCard messageType="ai" size="sm">
          Of course! The BrutalistCard uses different border radius values for user and AI messages to create a chat bubble effect.
        </BrutalistCard>
      </div>
    </div>
  ),
};

export const RichContent: Story = {
  args: {
    variant: "default",
    size: "lg",
    children: (
      <div>
        <h2 style={{ margin: "0 0 1rem 0" }}>Welcome Back!</h2>
        <p style={{ margin: "0 0 1rem 0" }}>
          This card contains rich content with multiple elements.
        </p>
        <button style={{ padding: "0.5rem 1rem", cursor: "pointer" }}>
          Get Started
        </button>
      </div>
    ),
  },
};
