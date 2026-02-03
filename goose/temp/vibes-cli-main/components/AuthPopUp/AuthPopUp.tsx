import { useState, useEffect } from "react";
import { VibesButton, YELLOW, RED } from "../VibesButton/VibesButton.js";
import {
  getOverlayStyle,
  getGridBackgroundStyle,
  getBlackBorderWrapperStyle,
  getContainerStyle,
  getBackgroundStyle,
  getCardIconStyle,
  getButtonsContainerStyle,
  getCardIconAnimationStyles,
  getButtonsCenterWrapperStyle,
  getCloseButtonStyle,
} from "./AuthPopUp.styles.js";

import card1 from "./temporalCards/card-1.png";
import card2 from "./temporalCards/card-2.png";
import card3 from "./temporalCards/card-3.png";
import card4 from "./temporalCards/card-4.png";

const cardImages = [card1, card2, card3, card4];

export interface AuthPopUpProps {
  isOpen: boolean;
  onClose: () => void;
  appName?: string;
}

export const AuthPopUp = ({ isOpen, onClose, appName = "App" }: AuthPopUpProps) => {
  const [isShredding, setIsShredding] = useState(false);
  const [selectedCard, setSelectedCard] = useState(cardImages[0]);

  useEffect(() => {
    if (isOpen) {
      setIsShredding(false);
      const randomIndex = Math.floor(Math.random() * cardImages.length);
      setSelectedCard(cardImages[randomIndex]);
    }
  }, [isOpen]);

  const handleInstallClick = () => {
    setIsShredding(true);
  };

  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <>
      <style>{getCardIconAnimationStyles()}</style>
      <div style={getOverlayStyle()} onClick={handleOverlayClick}>
        <div style={getGridBackgroundStyle()} />
        <div style={getBlackBorderWrapperStyle()}>
          <button style={getCloseButtonStyle()} onClick={onClose}>
            ×
          </button>
          <div style={getBackgroundStyle(isShredding)} />
          <div style={getContainerStyle()}>
            <div style={getCardIconStyle(isShredding)}>
              <img
                src={selectedCard}
                alt="Vibes Card"
                style={{ display: "block", width: "200px", height: "auto" }}
              />
            </div>

            <div style={getButtonsContainerStyle()}>
              <div onClick={handleInstallClick}>
                <VibesButton buttonType="form" formColor="white">
                  Install {appName}
                </VibesButton>
              </div>

              <div style={getButtonsCenterWrapperStyle()}>
                <VibesButton variant={YELLOW} buttonType="flat-rounded" icon="google">
                  Continue with Google
                </VibesButton>

                <VibesButton variant={RED} buttonType="flat-rounded" icon="github">
                  Continue with GitHub
                </VibesButton>
              </div>

              <VibesButton variant={RED} buttonType="form">
                <span style={{ color: "white" }}>Log In</span>
              </VibesButton>
            </div>
          </div>
        </div>
      </div>
    </>
  );
};
