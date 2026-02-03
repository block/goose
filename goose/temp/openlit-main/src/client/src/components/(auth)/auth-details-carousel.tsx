"use client";
import Image from "next/image";
import { Carousel, CarouselContent, CarouselItem } from "../ui/carousel";
import { useEffect, useState } from "react";
import Autoplay from "embla-carousel-autoplay";

const features: { title: string; description: string; image: string }[] = [
	{
		title: "Application and Request Tracing",
		description:
			"Provides end-to-end tracing of requests across different providers to improve performance visibility.",
		image: "/images/features/request.png",
	},
	{
		title: "Track Application Errors",
		description:
			"Monitors and logs application errors to help detect and troubleshoot issues.",
		image: "/images/features/exception.png",
	},
	{
		title: "Openlit PlayGround",
		description:
			"Test and compare different LLMs side-by-side based on performance, cost, and other key metrics",
		image: "/images/features/openground.png",
	},
	{
		title: "Centralized Prompt Repository",
		description:
			"Allows for organized storage, versioning, and usage of prompts with dynamic variables across different applications.",
		image: "/images/features/prompt.png",
	},
	{
		title: "Secure Secrets Management",
		description:
			"Vault offers a secure way to store and manage sensitive application secrets.",
		image: "/images/features/vault.png",
	},
];

export default function AuthDetailsCarousel() {
	const [api, setApi] = useState<any>();
	const [current, setCurrent] = useState(0);
	const autoplayPlugin = Autoplay({ playOnInit: true, delay: 3000 });

	useEffect(() => {
		if (!api) return;

		api.on("select", () => {
			setCurrent(api.selectedScrollSnap());
		});
	}, [api]);

	return (
		<div className="relative hidden lg:block bg-primary/[0.1]">
			<div className="absolute top-8 left-8 z-10 flex items-center">
				<Image
					src="/images/logo.png"
					alt="Image"
					width="50"
					height="50"
					className="object-cover"
				/>
			</div>
			<Carousel
				setApi={setApi}
				plugins={[autoplayPlugin]}
				className="w-full h-full"
				opts={{
					loop: true,
				}}
			>
				<CarouselContent>
					{features.map((feature, index) => (
						<CarouselItem key={index} className="h-screen">
							<div className="flex flex-col items-center justify-center h-full p-8 text-center">
								<div className="relative w-full h-3/5 mb-2">
									<Image
										src={feature.image || "/placeholder.svg"}
										alt={feature.title}
										fill
										className="object-contain"
									/>
								</div>
								<h2 className="text-2xl font-semibold mb-2">{feature.title}</h2>
								<p className="max-w-md text-sm text-stone-500">
									{feature.description}
								</p>
							</div>
						</CarouselItem>
					))}
				</CarouselContent>
				<div className="absolute bottom-8 left-0 right-0 flex justify-center gap-2">
					{features.map((_, index) => (
						<button
							key={index}
							className={`w-2 h-2 rounded-full transition-all ${
								current === index ? "bg-primary w-4" : "bg-primary/30"
							}`}
							onClick={() => api?.scrollTo(index)}
						/>
					))}
				</div>
			</Carousel>
		</div>
	);
}
