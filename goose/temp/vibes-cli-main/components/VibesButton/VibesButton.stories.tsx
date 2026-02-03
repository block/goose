import type { Meta, StoryObj } from "@storybook/react";
import { fn } from "@storybook/test";
import { VibesButton, BLUE, RED, YELLOW, GRAY } from "./VibesButton";

const meta: Meta<typeof VibesButton> = {
  title: "Components/VibesButton",
  component: VibesButton,
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
  argTypes: {
    variant: {
      control: "select",
      options: [BLUE, RED, YELLOW, GRAY],
      description: "Visual variant of the button",
    },
    buttonType: {
      control: "select",
      options: ["square", "flat", "flat-rounded", "form"],
      description: "Button type affects styling and behavior",
    },
    icon: {
      control: "select",
      options: [undefined, "login", "remix", "invite", "settings", "back", "google", "github"],
      description: "Icon to display in the button",
    },
    ignoreDarkMode: {
      control: "boolean",
      description: "When true, button colors remain constant regardless of dark mode",
    },
    formColor: {
      control: "color",
      description: "Custom background color for form type buttons",
    },
    disabled: {
      control: "boolean",
      description: "Disable the button",
    },
  },
  args: {
    onClick: fn(),
    onHover: fn(),
    onUnhover: fn(),
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    children: "Click me",
    variant: BLUE,
  },
};

export const Blue: Story = {
  args: {
    children: "Blue Button",
    variant: BLUE,
  },
};

export const Red: Story = {
  args: {
    children: "Red Button",
    variant: RED,
  },
};

export const Yellow: Story = {
  args: {
    children: "Yellow Button",
    variant: YELLOW,
  },
};

export const Gray: Story = {
  args: {
    children: "Gray Button",
    variant: GRAY,
  },
};

export const WithLoginIcon: Story = {
  args: {
    children: "Login",
    variant: BLUE,
    icon: "login",
  },
};

export const WithRemixIcon: Story = {
  args: {
    children: "Remix",
    variant: RED,
    icon: "remix",
  },
};

export const WithInviteIcon: Story = {
  args: {
    children: "Invite",
    variant: YELLOW,
    icon: "invite",
  },
};

export const WithSettingsIcon: Story = {
  args: {
    children: "Settings",
    variant: GRAY,
    icon: "settings",
  },
};

export const WithBackIcon: Story = {
  args: {
    children: "Back",
    variant: BLUE,
    icon: "back",
  },
};

export const FormButton: Story = {
  args: {
    children: "Submit",
    variant: YELLOW,
    buttonType: "form",
  },
  decorators: [
    (Story) => (
      <div style={{ width: "300px" }}>
        <Story />
      </div>
    ),
  ],
};

export const FormButtonWhite: Story = {
  args: {
    children: "Install App",
    buttonType: "form",
    formColor: "white",
  },
  decorators: [
    (Story) => (
      <div style={{ width: "300px" }}>
        <Story />
      </div>
    ),
  ],
};

export const FlatRoundedWithGoogle: Story = {
  args: {
    children: "Continue with Google",
    variant: YELLOW,
    buttonType: "flat-rounded",
    icon: "google",
  },
  decorators: [
    (Story) => (
      <div style={{ width: "300px" }}>
        <Story />
      </div>
    ),
  ],
};

export const FlatRoundedWithGitHub: Story = {
  args: {
    children: "Continue with GitHub",
    variant: RED,
    buttonType: "flat-rounded",
    icon: "github",
  },
  decorators: [
    (Story) => (
      <div style={{ width: "300px" }}>
        <Story />
      </div>
    ),
  ],
};

export const AllVariants: Story = {
  render: () => (
    <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap" }}>
      <VibesButton variant={BLUE}>Blue</VibesButton>
      <VibesButton variant={RED}>Red</VibesButton>
      <VibesButton variant={YELLOW}>Yellow</VibesButton>
      <VibesButton variant={GRAY}>Gray</VibesButton>
    </div>
  ),
};

export const AllIcons: Story = {
  render: () => (
    <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap" }}>
      <VibesButton variant={BLUE} icon="login">Login</VibesButton>
      <VibesButton variant={RED} icon="remix">Remix</VibesButton>
      <VibesButton variant={YELLOW} icon="invite">Invite</VibesButton>
      <VibesButton variant={GRAY} icon="settings">Settings</VibesButton>
      <VibesButton variant={BLUE} icon="back">Back</VibesButton>
    </div>
  ),
};

export const AllButtonTypes: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", gap: "1rem", width: "300px" }}>
      <VibesButton variant={BLUE}>Square (default)</VibesButton>
      <VibesButton variant={YELLOW} buttonType="flat-rounded" icon="google">
        Flat Rounded
      </VibesButton>
      <VibesButton variant={RED} buttonType="form">
        Form Button
      </VibesButton>
      <VibesButton buttonType="form" formColor="white">
        Form with Custom Color
      </VibesButton>
    </div>
  ),
};

export const Disabled: Story = {
  args: {
    children: "Disabled",
    variant: BLUE,
    disabled: true,
  },
};
