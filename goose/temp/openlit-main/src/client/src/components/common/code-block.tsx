import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { dracula } from "react-syntax-highlighter/dist/esm/styles/prism";
import copyToCB from "copy-to-clipboard";
import { Copy } from "lucide-react";

export default function CodeBlock({
	className = "",
	code,
	copy = true,
	language,
}: {
	className?: string;
	code: string;
	copy?: boolean;
	language: string;
}) {
	const onClickCopy = () => copyToCB(code);

	return (
		<section className="relative group">
			<SyntaxHighlighter
				language={language}
				style={dracula}
				className={`overflow-hidden ${className}`}
			>
				{code}
			</SyntaxHighlighter>
			{copy && (
				<Copy
					className="w-4 h-4 absolute right-3 top-4 hidden group-hover:inline-block z-10 text-stone-400 hover:text-primary cursor-pointer"
					onClick={onClickCopy}
				/>
			)}
		</section>
	);
}
