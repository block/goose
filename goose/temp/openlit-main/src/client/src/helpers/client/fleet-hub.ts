import { Agent, AgentAttribute } from "@/types/fleet-hub"
import { get } from "lodash"

export const getAttributeValue = (agent: Agent, path:string, key: string, defaultValue: string = "N/A") => {
	const attributes: AgentAttribute[] = get(agent, path);
	return get(attributes.find((attr) => attr.key === key), "value.Value.StringValue", defaultValue)
}