import React from "react";

export const ModelSelectionTip = ({
  text = "goose relies heavily on tool calling capabilities and currently works best with Claude 4 models.",
}) => {
  return (
    <p>{text}</p>
  );
};
