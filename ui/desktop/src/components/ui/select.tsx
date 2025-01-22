import React, { ReactNode } from "react";

export const Select = ({ children }: { children: ReactNode }) => {
    return <div className="relative">{children}</div>;
};

export const SelectTrigger = ({ onClick, children }: { onClick: () => void; children: ReactNode }) => {
    return (
        <button
            onClick={onClick}
            className="p-2 border rounded-md w-full text-left bg-white"
        >
            {children}
        </button>
    );
};

export const SelectValue = ({
                                value,
                                placeholder,
                            }: {
    value?: string | null;
    placeholder: string;
}) => {
    return <span>{value || placeholder}</span>;
};

export const SelectContent = ({ isOpen, children }: { isOpen: boolean; children: ReactNode }) => {
    if (!isOpen) return null;
    return (
        <ul className="absolute bg-white border rounded-md mt-2 w-full shadow-lg">
            {children}
        </ul>
    );
};

export const SelectItem = ({
                               value,
                               onSelect,
                               children,
                           }: {
    value: string;
    onSelect: (value: string) => void;
    children: ReactNode;
}) => {
    return (
        <li
            className="p-2 cursor-pointer hover:bg-gray-100"
            onClick={() => onSelect(value)}
        >
            {children}
        </li>
    );
};
