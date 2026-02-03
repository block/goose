"use client";

import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Loader2 } from "lucide-react";
import { useChangeSeatCount, useChangePlan } from "../hooks/use-change-subscription";
import { usePlans } from "../hooks/use-plans";

interface ChangeSubscriptionDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  requestedPlanCode: string;
  changeType: "seat-change" | "plan-change";
}

export function ChangeSubscriptionDialog({
  open,
  onOpenChange,
  requestedPlanCode,
  changeType,
}: ChangeSubscriptionDialogProps) {
  const [seatCount, setSeatCount] = useState<number | "">(1);
  const { changeSeatCount, isChanging: isChangingSeatCount } = useChangeSeatCount();
  const { changePlan, isChanging: isChangingPlan } = useChangePlan();
  const { data: plans } = usePlans();

  const requestedPlan = plans?.find((plan) => plan.plan_code === requestedPlanCode);
  const maxSeats = requestedPlan?.max_seats_for_subscription;

  // Determine which loading state to show
  const isChanging = isChangingSeatCount || isChangingPlan;

  // Validation
  const isValidSeatCount = () => {
    if (typeof seatCount !== "number" || seatCount < 1) return false;
    if (maxSeats && seatCount > maxSeats) return false;
    return true;
  };

  const handleConfirm = () => {
    if (!isValidSeatCount()) return;
    if (typeof seatCount !== "number") {
      return;
    }

    if (changeType === "seat-change") {
      changeSeatCount({
        seat_count: seatCount,
      });
    } else if (changeType === "plan-change") {
      changePlan({
        plan_code: requestedPlanCode,
        seat_count: seatCount,
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            {changeType === "seat-change"
              ? "Change Seat Count"
              : "Change Plan to " + requestedPlan?.display_name}
          </DialogTitle>
          <DialogDescription>
            Enter the number of seats you need for your organization.
          </DialogDescription>
        </DialogHeader>

        {
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="seat-count">Number of Seats</Label>
              <Input
                id="seat-count"
                type="number"
                min={1}
                max={maxSeats || undefined}
                value={seatCount}
                onChange={(e) => {
                  const value = e.target.value;
                  setSeatCount(value === "" ? "" : parseInt(value));
                }}
                placeholder={`Enter number of seats ${maxSeats ? `(Max ${maxSeats})` : ""}`}
                className={!isValidSeatCount() && seatCount !== "" ? "border-destructive" : ""}
              />
              {!isValidSeatCount() && seatCount !== "" && (
                <p className="text-sm text-destructive">
                  {typeof seatCount === "number" && seatCount < 1
                    ? `Minimum 1 seat required`
                    : maxSeats && typeof seatCount === "number" && seatCount > maxSeats
                      ? `Maximum ${maxSeats} seat${maxSeats !== 1 ? "s" : ""} allowed`
                      : "Invalid seat count"}
                </p>
              )}
              <p className="text-sm text-muted-foreground">
                Price: ${(29.99 * (typeof seatCount === "number" ? seatCount : 0)).toFixed(2)}/month
              </p>
            </div>
          </div>
        }

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isChanging}>
            Cancel
          </Button>
          <Button onClick={handleConfirm} disabled={isChanging || !isValidSeatCount()}>
            {isChanging && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Continue
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
