import React, { useState } from "react";
import { Key } from "./types";
import { showToast } from "../ui/toast";
import { Tooltip } from "../ui/Tooltip";
import { Copy, Edit, SensitiveHidden, SensitiveVisible } from "../icons";

interface KeyItemProps {
  keyData: Key;
  onEdit: (key: Key) => void;
  onCopy: (value: string) => void;
}

export function KeyItem({ keyData, onEdit, onCopy }: KeyItemProps) {
  const [isValueVisible, setIsValueVisible] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(keyData.value);
      showToast(`${keyData.name} copied to clipboard!`, "success");
    } catch (err) {
      showToast("Failed to copy the key.", "error");
    }
  };

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg p-4 mb-2">
      <div className="flex justify-between items-center">
        <h3 className="text-lg font-medium dark:text-white">{keyData.name}</h3>
        <div className="flex items-center gap-3">
          <div className="flex items-center">
            <span className="text-gray-500">
              {isValueVisible ? keyData.value : "*".repeat(17)}
            </span>
            <Tooltip content={isValueVisible ? "Hide" : "Reveal"}>
              <button
                onClick={() => setIsValueVisible(!isValueVisible)}
                className="ml-2 text-gray-400 hover:text-gray-600"
              >
                {isValueVisible ? <SensitiveVisible /> : <SensitiveHidden />}
              </button>
            </Tooltip>
            <Tooltip content="Copy to clipboard">
              <button
                onClick={handleCopy}
                className="ml-3 text-gray-400 hover:text-gray-600"
              >
                <Copy className="h-5 w-5" />
              </button>
            </Tooltip>
          </div>
          <Tooltip content="Edit">
            <button
              onClick={() => onEdit(keyData)}
              className="text-gray-400 hover:text-gray-600"
            >
              <Edit />
            </button>
          </Tooltip>
        </div>
      </div>
    </div>
  );
}
