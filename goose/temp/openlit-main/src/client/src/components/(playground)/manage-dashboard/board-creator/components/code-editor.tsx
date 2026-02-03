"use client";

import type React from "react";
import { useRef, useEffect } from "react";
import Editor from "@monaco-editor/react";
import type { EditorProps } from "../types";
import { CLICKHOUSE_LANGUAGE_CONFIG } from "../constants";

const CodeEditor: React.FC<EditorProps> = ({
	value,
	onChange,
	language = "sql",
	height = "100%",
	fullScreen = false,
	readOnly,
}) => {
	const editorRef = useRef<any>(null);
	const completionProviderRef = useRef<any>(null);

	// Handle Monaco Editor mount
	const handleEditorDidMount = (editor: any) => {
		editorRef.current = editor;
	};

	// Cleanup completion provider on unmount
	useEffect(() => {
		return () => {
			if (completionProviderRef.current) {
				completionProviderRef.current.dispose();
				completionProviderRef.current = null;
			}
		};
	}, []);

	// Mustache binding suggestions for autocomplete
	const mustacheCompletionItems = [
		{
			label: "filter.timeLimit.start",
			kind: "Property",
			insertText: "filter.timeLimit.start",
			documentation: "Start date of the time filter"
		},
		{
			label: "filter.timeLimit.end",
			kind: "Property",
			insertText: "filter.timeLimit.end",
			documentation: "End date of the time filter"
		},
	];

	return (
		<Editor
			height={height}
			defaultLanguage={language}
			value={value}
			onChange={onChange}
			onMount={handleEditorDidMount}
			options={{
				minimap: { enabled: fullScreen },
				scrollBeyondLastLine: false,
				fontSize: 14,
				wordWrap: "on",
				automaticLayout: true,
				readOnly: !!readOnly
			}}
			beforeMount={(monaco) => {
				// Register ClickHouse SQL language
				if (language === "clickhouse-sql") {
					monaco.languages.register({ id: "clickhouse-sql" });
					monaco.languages.setMonarchTokensProvider(
						"clickhouse-sql",
						CLICKHOUSE_LANGUAGE_CONFIG.loader().language as any
					);
					monaco.editor.defineTheme("clickhouse-dark", {
						base: "vs-dark",
						inherit: true,
						rules: [
							{ token: "keyword", foreground: "569CD6", fontStyle: "bold" },
							{ token: "operator", foreground: "D4D4D4" },
							{ token: "string", foreground: "CE9178" },
							{ token: "number", foreground: "B5CEA8" },
							{ token: "comment", foreground: "6A9955", fontStyle: "italic" },
							{ token: "predefined", foreground: "DCDCAA" },
						],
						colors: {
							"editor.background": "#1E1E1E",
						},
					});
					monaco.editor.setTheme("clickhouse-dark");
				}

				// Dispose previous completion provider if it exists
				if (completionProviderRef.current) {
					completionProviderRef.current.dispose();
				}

				// Register mustache binding completion provider for all languages
				completionProviderRef.current = monaco.languages.registerCompletionItemProvider(language, {
					provideCompletionItems: (model, position) => {
						const textUntilPosition = model.getValueInRange({
							startLineNumber: position.lineNumber,
							startColumn: 1,
							endLineNumber: position.lineNumber,
							endColumn: position.column,
						});

						const textAfterPosition = model.getValueInRange({
							startLineNumber: position.lineNumber,
							startColumn: position.column,
							endLineNumber: position.lineNumber,
							endColumn: model.getLineMaxColumn(position.lineNumber),
						});

						// Check if we're inside mustache braces {{}}
						const mustacheMatch = textUntilPosition.match(/\{\{([^}]*)$/);
						if (!mustacheMatch) {
							return { suggestions: [] };
						}

						// Get the existing text inside mustache braces
						const existingText = mustacheMatch[1];

						// Check if closing braces already exist after cursor
						const hasClosingBraces = textAfterPosition.startsWith('}}');

						const word = model.getWordUntilPosition(position);
						const range = {
							startLineNumber: position.lineNumber,
							endLineNumber: position.lineNumber,
							startColumn: word.startColumn,
							endColumn: word.endColumn,
						};

						return {
							suggestions: mustacheCompletionItems.map(item => {
								// Calculate what part of the suggestion is already typed
								let insertText = item.insertText;

								// If there's existing text, check if the suggestion starts with it
								if (existingText.trim()) {
									const existingTrimmed = existingText.trim();
									if (item.insertText.startsWith(existingTrimmed)) {
										// Only insert the remaining part
										insertText = item.insertText.substring(existingTrimmed.length);
									}
								}

								// Add closing braces if needed
								if (!hasClosingBraces) {
									insertText += '}}';
								}

								return {
									label: item.label,
									kind: monaco.languages.CompletionItemKind[item.kind as keyof typeof monaco.languages.CompletionItemKind],
									insertText: insertText,
									documentation: item.documentation,
									range: range,
									// Add snippet formatting to handle cursor positioning
									insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
								};
							})
						};
					},
					triggerCharacters: ['{', '.'],
				});
			}}
		/>
	);
};

export default CodeEditor;
