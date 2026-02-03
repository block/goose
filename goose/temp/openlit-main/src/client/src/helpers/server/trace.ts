export { getTraceMappingKeyFullPath } from "../client/trace";

export const buildHierarchy = (data: any[]) => {
	// Create a map for quick lookup of nodes by SpanId
	const nodeMap = new Map();
	data.forEach((item) => {
		nodeMap.set(item.SpanId, { ...item, children: [] });
	});

	let root = null;

	// Build the hierarchy
	data.forEach((item) => {
		const node = nodeMap.get(item.SpanId);
		if (item.ParentSpanId === "") {
			// Root node found
			root = node;
		} else if (nodeMap.has(item.ParentSpanId)) {
			// Link the node to its parent
			const parent = nodeMap.get(item.ParentSpanId);
			parent.children.push(node);
		}
	});

	return root; // Returns the hierarchical tree
};
