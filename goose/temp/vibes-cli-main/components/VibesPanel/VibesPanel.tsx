import React, { useState, useEffect, useId } from "react";
import { VibesButton, BLUE, RED, YELLOW, GRAY } from "../VibesButton/VibesButton.js";
import { BrutalistCard } from "../BrutalistCard/index.js";
import { LabelContainer } from "../LabelContainer/index.js";
import {
  getOuterContainerStyle,
  getButtonContainerStyle,
  getInviteFormStyle,
  getInviteLabelStyle,
  getInviteInputStyle,
  getInviteStatusStyle,
  getInviteRowStyle,
} from "./VibesPanel.styles.js";
import { useIsMobile } from "../hooks/useIsMobile.js";

export interface VibesPanelProps {
  style?: React.CSSProperties;
  className?: string;
  baseURL?: string;
  token?: string;
}

type PanelMode = "default" | "invite";

export function VibesPanel({
  style,
  className,
  baseURL,
  token,
}: VibesPanelProps = {}) {
  const emailId = useId();
  const isMobile = useIsMobile();
  const [mode, setMode] = useState<PanelMode>("default");
  const [email, setEmail] = useState("");
  const [inviteStatus, setInviteStatus] = useState<
    "idle" | "sending" | "success" | "error"
  >("idle");
  const [inviteMessage, setInviteMessage] = useState("");

  const effectiveBaseURL = baseURL ?? (typeof window !== "undefined" ? window.location.origin : "https://vibes.diy");

  const handleInviteClick = () => {
    if (mode === "default") {
      setMode("invite");
      setEmail("");
      setInviteStatus("idle");
      setInviteMessage("");
    }
  };

  const handleBackClick = () => {
    setMode("default");
  };

  const handleChangeCodeClick = () => {
    window.open(`${effectiveBaseURL}/remix`, "_top");
  };

  const handleLogoutClick = () => {
    document.dispatchEvent(new CustomEvent("vibes-sync-disable"));
    document.dispatchEvent(new CustomEvent("vibes-logout-request"));
  };

  const handleInviteSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!email.trim()) return;

    setInviteStatus("sending");
    setInviteMessage("");

    document.dispatchEvent(
      new CustomEvent("vibes-share-request", {
        detail: {
          email: email.trim(),
          role: "member",
          right: "read",
          token,
        },
      }),
    );
  };

  useEffect(() => {
    const handleShareSuccess = (event: Event) => {
      const customEvent = event as CustomEvent<{
        email: string;
        message?: string;
      }>;
      setInviteStatus("success");
      setInviteMessage(
        customEvent.detail?.message ||
          `Invitation sent to ${customEvent.detail?.email}!`,
      );
    };

    const handleShareError = (event: Event) => {
      const customEvent = event as CustomEvent<{ error: { message: string } }>;
      setInviteStatus("error");
      setInviteMessage(
        customEvent.detail?.error?.message ||
          "Failed to send invitation. Please try again.",
      );
    };

    document.addEventListener("vibes-share-success", handleShareSuccess);
    document.addEventListener("vibes-share-error", handleShareError);

    return () => {
      document.removeEventListener("vibes-share-success", handleShareSuccess);
      document.removeEventListener("vibes-share-error", handleShareError);
    };
  }, []);

  return (
    <div style={getOuterContainerStyle(style)} className={className}>
      <LabelContainer
        label="Launcher"
        disappear
      >
        <div style={getButtonContainerStyle(isMobile)}>
          {mode === "invite" ? (
            <div style={getInviteRowStyle(isMobile)}>
              <VibesButton
                variant={YELLOW}
                onClick={() => {}}
                icon="invite"
              >
                Invite
              </VibesButton>
              {inviteStatus === "idle" ? (
                <form
                  onSubmit={handleInviteSubmit}
                  style={getInviteFormStyle(isMobile)}
                >
                  <label htmlFor={emailId} style={getInviteLabelStyle()}>
                    Invite by email
                  </label>
                  <input
                    id={emailId}
                    type="email"
                    placeholder="friend@example.com"
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    style={getInviteInputStyle()}
                    autoComplete="email"
                    required
                  />
                  <VibesButton
                    variant={YELLOW}
                    type="submit"
                    disabled={!email.trim()}
                  >
                    Submit
                  </VibesButton>
                </form>
              ) : (
                <BrutalistCard
                  id="invite-status"
                  role="status"
                  aria-live="polite"
                  size="sm"
                  variant={
                    inviteStatus === "sending"
                      ? "default"
                      : inviteStatus === "error"
                        ? "error"
                        : "success"
                  }
                  style={getInviteStatusStyle()}
                >
                  {inviteStatus === "sending" ? "Inviting..." : inviteMessage}
                </BrutalistCard>
              )}
              <VibesButton variant={GRAY} onClick={handleBackClick} icon="back">
                Back
              </VibesButton>
            </div>
          ) : (
            <>
              <VibesButton
                variant={BLUE}
                onClick={handleLogoutClick}
                icon="login"
              >
                Logout
              </VibesButton>
              {/* <VibesButton
                variant={RED}
                onClick={handleChangeCodeClick}
                icon="remix"
              >
                Remix
              </VibesButton> */}
              <VibesButton
                variant={YELLOW}
                onClick={handleInviteClick}
                icon="invite"
              >
                Invite
              </VibesButton>
              {/* <VibesButton
                variant={GRAY}
                icon="settings"
                onClick={() => (window.location.href = "https://vibes.diy/")}
              >
                Home
              </VibesButton> */}
            </>
          )}
        </div>
      </LabelContainer>
    </div>
  );
}
