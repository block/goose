import { LucideIcon } from "lucide-react";
import { IconType } from "react-icons";
import { Label } from "@/components/ui/label";
import { ReactNode } from "react";

interface SettingsItemProps {
  icon: LucideIcon | IconType;
  label: string;
  description?: ReactNode;
  children?: ReactNode;
  iconClassName?: string;
  containerClassName?: string;
}

export function SettingsItem({
  icon: Icon,
  label,
  description,
  children,
  iconClassName = "text-primary",
  containerClassName = "bg-primary/10",
}: SettingsItemProps) {
  return (
    <div className="flex items-center gap-4">
      <div
        className={`h-10 w-10 rounded-full ${containerClassName} flex items-center justify-center`}
      >
        <Icon className={`h-5 w-5 ${iconClassName}`} />
      </div>
      <div className="flex-1">
        <Label className="text-sm font-medium">{label}</Label>
        {description && <div className="text-sm text-muted-foreground">{description}</div>}
      </div>
      {children}
    </div>
  );
}
