"use client";

import React, { useState, useCallback, useRef, useEffect } from "react";
import { cn } from "@/lib/utils";

interface ResizeableHandleProps {
	/** Position of the resize handle relative to its container */
	position?: "left" | "right" | "top" | "bottom";
	/** Custom CSS classes for the handle */
	className?: string;
	/** Callback fired during resize with delta movement and resize state */
	onResize: (delta: number, isResizing: boolean) => void;
	/** Whether the handle is disabled */
	disabled?: boolean;
}

/**
 * A reusable resize handle component that can be positioned on any side of a container.
 * 
 * @example
 * ```tsx
 * // Basic horizontal resizing
 * <ResizableHandle 
 *   position="right" 
 *   onResize={(delta, isResizing) => {
 *     if (delta !== 0) {
 *       setWidth(prev => prev + delta);
 *     }
 *   }} 
 * />
 * 
 * // Vertical resizing
 * <ResizableHandle 
 *   position="bottom" 
 *   onResize={(delta, isResizing) => {
 *     if (delta !== 0) {
 *       setHeight(prev => prev + delta);
 *     }
 *   }} 
 * />
 * ```
 */
export function ResizeableHandle({
	position = "right",
	className,
	onResize,
	disabled = false,
}: ResizeableHandleProps) {
	const [isResizing, setIsResizing] = useState(false);
	const startMousePos = useRef<{ x: number; y: number }>({ x: 0, y: 0 });
	const lastMousePos = useRef<{ x: number; y: number }>({ x: 0, y: 0 });

	const isHorizontal = position === "left" || position === "right";
	const isVertical = position === "top" || position === "bottom";

	// Handle mouse down on resize handle
	const handleMouseDown = useCallback(
		(e: React.MouseEvent) => {
			if (disabled) return;
			
			setIsResizing(true);
			startMousePos.current = { x: e.clientX, y: e.clientY };
			lastMousePos.current = { x: e.clientX, y: e.clientY };
			onResize(0, true);
			e.preventDefault();
		},
		[disabled, onResize]
	);

	// Handle mouse move for resizing
	const handleMouseMove = useCallback(
		(e: MouseEvent) => {
			if (!isResizing) return;

			const currentPos = { x: e.clientX, y: e.clientY };
			
			let delta = 0;
			if (isHorizontal) {
				const mouseDelta = currentPos.x - lastMousePos.current.x;
				delta = position === "left" ? -mouseDelta : mouseDelta;
			} else if (isVertical) {
				const mouseDelta = currentPos.y - lastMousePos.current.y;
				delta = position === "top" ? -mouseDelta : mouseDelta;
			}

			lastMousePos.current = currentPos;
			onResize(delta, true);
		},
		[isResizing, position, isHorizontal, isVertical, onResize]
	);

	// Handle mouse up to stop resizing
	const handleMouseUp = useCallback(() => {
		if (isResizing) {
			setIsResizing(false);
			onResize(0, false);
		}
	}, [isResizing, onResize]);

	// Add global event listeners for mouse move and mouse up
	useEffect(() => {
		if (isResizing) {
			document.addEventListener("mousemove", handleMouseMove);
			document.addEventListener("mouseup", handleMouseUp);
			
			const cursor = isHorizontal ? "col-resize" : "row-resize";
			document.body.style.cursor = cursor;
			document.body.style.userSelect = "none";

			return () => {
				document.removeEventListener("mousemove", handleMouseMove);
				document.removeEventListener("mouseup", handleMouseUp);
				document.body.style.cursor = "";
				document.body.style.userSelect = "";
			};
		}
	}, [isResizing, handleMouseMove, handleMouseUp, isHorizontal]);

	const getPositionClasses = () => {
		switch (position) {
			case "left":
				return "left-0 top-1/2 -translate-y-1/2 -translate-x-1/2 cursor-col-resize";
			case "right":
				return "right-0 top-1/2 -translate-y-1/2 translate-x-1/2 cursor-col-resize";
			case "top":
				return "top-0 left-1/2 -translate-x-1/2 -translate-y-1/2 cursor-row-resize";
			case "bottom":
				return "bottom-0 left-1/2 -translate-x-1/2 translate-y-1/2 cursor-row-resize";
			default:
				return "right-0 top-1/2 -translate-y-1/2 translate-x-1/2 cursor-col-resize";
		}
	};

	const isHorizontalHandle = position === "left" || position === "right";

	const defaultHandleClass = cn(
		"absolute rounded border border-stone-200 dark:border-stone-700 bg-white dark:bg-stone-800 opacity-0 group-hover:opacity-100 transition-all duration-200 group-hover:border-stone-400 dark:group-hover:border-stone-500 group-hover:scale-110",
		isResizing && "opacity-100 border-stone-400 dark:border-stone-500 scale-110 bg-stone-50 dark:bg-stone-700",
		disabled && "cursor-not-allowed opacity-30",
		isHorizontalHandle ? "w-3 h-6 p-0.5" : "w-6 h-3 p-0.5",
		getPositionClasses()
	);

	// SVG grip pattern with 6 circles
	const GripIcon = () => (
		<svg 
			viewBox="0 0 12 12" 
			className={cn(
				"w-full h-full",
				isHorizontalHandle ? "rotate-90" : ""
			)}
		>
			{/* First row of 3 circles */}
			<circle cx="2" cy="3" r="1" className="fill-stone-400 dark:fill-stone-500" />
			<circle cx="6" cy="3" r="1" className="fill-stone-400 dark:fill-stone-500" />
			<circle cx="10" cy="3" r="1" className="fill-stone-400 dark:fill-stone-500" />
			
			{/* Second row of 3 circles */}
			<circle cx="2" cy="9" r="1" className="fill-stone-400 dark:fill-stone-500" />
			<circle cx="6" cy="9" r="1" className="fill-stone-400 dark:fill-stone-500" />
			<circle cx="10" cy="9" r="1" className="fill-stone-400 dark:fill-stone-500" />
		</svg>
	);

	return (
		<div
			className={cn(defaultHandleClass, className)}
			onMouseDown={handleMouseDown}
		>
			<GripIcon />
		</div>
	);
}

interface ResizeablePanelProps {
	/** Content to render inside the panel */
	children: React.ReactNode;
	/** Initial width of the panel in pixels */
	defaultWidth?: number;
	/** Minimum allowed width in pixels */
	minWidth?: number;
	/** Maximum allowed width in pixels */
	maxWidth?: number;
	/** Custom CSS classes for the panel container */
	className?: string;
	/** Position of the resize handle */
	handlePosition?: "left" | "right";
	/** Custom CSS classes for the resize handle */
	handleClassName?: string;
	/** Callback fired when the width changes */
	onWidthChange?: (width: number) => void;
	/** Whether resizing is disabled */
	disabled?: boolean;
}

/**
 * A resizable panel component with built-in width management and constraints.
 * Uses ResizableHandle internally for the drag functionality.
 * 
 * @example
 * ```tsx
 * // Basic resizable sidebar
 * <ResizablePanel 
 *   defaultWidth={300}
 *   minWidth={200}
 *   maxWidth={500}
 *   handlePosition="right"
 *   onWidthChange={(width) => console.log('New width:', width)}
 * >
 *   <div>Sidebar content</div>
 * </ResizablePanel>
 * 
 * // Custom styling
 * <ResizablePanel 
 *   className="bg-gray-100 border"
 *   handleClassName="bg-blue-500"
 *   handlePosition="left"
 * >
 *   <div>Panel content</div>
 * </ResizablePanel>
 * ```
 */
export function ResizeablePanel({
	children,
	defaultWidth = 400,
	minWidth = 200,
	maxWidth = 600,
	className,
	handlePosition = "right",
	handleClassName,
	onWidthChange,
	disabled = false,
}: ResizeablePanelProps) {
	const [width, setWidth] = useState(defaultWidth);

	// Handle resize from the ResizableHandle component
	const handleResize = useCallback(
		(delta: number, isResizing: boolean) => {
			if (!isResizing && delta === 0) {
				// Resize ended, no action needed
				return;
			}

			if (delta !== 0) {
				setWidth((prevWidth) => {
					const newWidth = prevWidth + delta;
					const constrainedWidth = Math.max(minWidth, Math.min(maxWidth, newWidth));
					
					// Only call onWidthChange if width actually changed
					if (constrainedWidth !== prevWidth) {
						onWidthChange?.(constrainedWidth);
					}
					
					return constrainedWidth;
				});
			}
		},
		[minWidth, maxWidth, onWidthChange]
	);

	useEffect(() => {
		setWidth(defaultWidth);
	}, [defaultWidth])

	return (
		<div
			className={cn("relative flex group", className)}
			style={{ width: `${width}px` }}
		>
			{/* Resize handle */}
			<ResizeableHandle
				position={handlePosition}
				className={handleClassName}
				onResize={handleResize}
				disabled={disabled}
			/>
			
			{/* Content */}
			<div className="flex-1 overflow-hidden">
				{children}
			</div>
		</div>
	);
} 