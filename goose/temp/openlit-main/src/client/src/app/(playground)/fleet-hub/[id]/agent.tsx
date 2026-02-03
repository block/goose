"use client"

import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Clock, HeartPulse, Package, Server } from "lucide-react"
import { Agent } from "@/types/fleet-hub"
import { formatDate } from "@/utils/date"
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion"
import { cn } from "@/lib/utils"
import { CodeEditor } from "@/components/(playground)/manage-dashboard/board-creator"
import { useCallback, useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import useFetchWrapper from "@/utils/hooks/useFetchWrapper"
import { jsonStringify } from "@/utils/json"
import { consoleLog } from "@/utils/log"
import { getAttributeValue } from "@/helpers/client/fleet-hub"
import { usePageHeader } from "@/selectors/page"
import { usePostHog } from "posthog-js/react"
import { CLIENT_EVENTS } from "@/constants/events"
import { toast } from "sonner"

interface AgentDetailProps {
  agent: Agent,
  fetchAgentInfo: () => void
}

const InfoItem = ({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) => (
  <div className="space-y-1">
    <p className="text-xs text-muted-foreground">{label}</p>
    <p className={cn("text-sm font-medium", mono && "font-mono")}>{value}</p>
  </div>
);

export default function AgentDetail({ agent, fetchAgentInfo }: AgentDetailProps) {
  const isUnhealthy = !agent.Status.health.healthy;
  const { setHeader } = usePageHeader();

  useEffect(() => {
    setHeader({
      title: getAttributeValue(agent, "Status.agent_description.identifying_attributes", "service.name"),
      breadcrumbs: [
        {
          title: "Fleet Hub",
          href: "/fleet-hub"
        }
      ]
    })
  }, [agent]);

  return (
    <div className="space-y-6 overflow-auto w-full flex flex-col grow">
      <Card>
        <CardHeader className="flex flex-row items-center gap-8 space-y-0 p-4">
          <CardTitle>{getAttributeValue(agent, "Status.agent_description.identifying_attributes", "service.name")}</CardTitle>
        </CardHeader>
        <CardContent className="p-4 pt-0">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <div className="flex items-center gap-3 p-3 rounded-lg bg-stone-200/50 dark:bg-stone-700/50">
              <Package className="h-4 w-4 text-primary" />
              <div>
                <p className="text-xs text-muted-foreground">Version</p>
                <p className="font-mono text-sm font-medium">{getAttributeValue(agent, "Status.agent_description.identifying_attributes", "service.version")}</p>
              </div>
            </div>
            <div className="flex items-center gap-3 p-3 rounded-lg bg-stone-200/50 dark:bg-stone-700/50">
              <Clock className="h-4 w-4 text-primary" />
              <div>
                <p className="text-xs text-muted-foreground">Started At</p>
                <p className="font-mono text-sm font-medium">{formatDate(agent.StartedAt, { time: true })}</p>
              </div>
            </div>
            <div className="flex items-center gap-3 p-3 rounded-lg bg-stone-200/50 dark:bg-stone-700/50">
              <Server className="h-4 w-4 text-primary" />
              <div>
                <p className="text-xs text-muted-foreground">Host Name</p>
                <p className="font-mono text-sm font-medium">{getAttributeValue(agent, "Status.agent_description.non_identifying_attributes", "host.name")}</p>
              </div>
            </div>
            <div className="flex items-center gap-3 p-3 rounded-lg bg-stone-200/50 dark:bg-stone-700/50">
              <HeartPulse className="h-4 w-4 text-primary" />
              <div>
                <p className="text-xs text-muted-foreground">Health Status</p>
                <p className={`font-mono text-xs font-medium px-2 text-center ${agent.Status.health.healthy ? "bg-green-500 text-white dark:bg-green-500 dark:text-white" : "bg-red-500 text-white dark:bg-red-500 dark:text-white"}`}>{agent.Status.health.status || "Error"}</p>
              </div>
            </div>
            {
              isUnhealthy && agent.Status.health.last_error ? (
                <div className="bg-red-500 text-white text-center text-xs p-2 col-span-4">{agent.Status.health.last_error} </div>
              ) : null
            }
          </div>
        </CardContent>
        <CardFooter className="p-0">
          <Accordion type="single" collapsible className="w-full">
            <AccordionItem value="item-1" className="flex flex-col-reverse w-full border-0">
              <AccordionTrigger className="p-0 flex w-full items-center gap-2 justify-center py-1 bg-stone-200/60 dark:bg-stone-800 text-xs text-stone-500 dark:text-stone-500">Show more</AccordionTrigger>
              <AccordionContent className="p-4 pt-0">
                <div className="pt-4 border-t border-stone-200 dark:border-stone-700 space-y-4">
                  <h4 className="font-semibold text-sm">Detailed Information</h4>
                  <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                    {agent.Status.agent_description.identifying_attributes.map((attr, index) => (
                      <InfoItem key={index} label={attr.key} value={attr.value.Value.StringValue} />
                    ))}
                    {agent.Status.agent_description.non_identifying_attributes.map((attr, index) => (
                      <InfoItem key={index} label={attr.key} value={attr.value.Value.StringValue} />
                    ))}
                  </div>

                  {Object.keys(agent.Status.health.component_health_map || {}).length > 0 ? <h4 className="font-semibold text-sm">Component Health Status</h4> : null}
                  <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                    {Object.entries(agent.Status.health.component_health_map || {}).map(([component, health]) => (
                      <div key={component} className={`p-4 rounded-lg border ${health.healthy ? "bg-green-500/10 border-green-500/20" : "bg-red-500/10 border-red-500/20"}`}>
                        <p className="text-xs text-muted-foreground mb-1">{component}</p>
                        <Badge variant={health.healthy ? "default" : "destructive"}>{health.status}</Badge>
                      </div>
                    ))}
                  </div>
                </div>
              </AccordionContent>
            </AccordionItem>
          </Accordion>
        </CardFooter>
      </Card>
      <ConfigDetails agent={agent} fetchAgentInfo={fetchAgentInfo} />
    </div>
  )
}

function ConfigDetails({ agent, fetchAgentInfo }: { agent: Agent, fetchAgentInfo: () => void }) {
  const [yamlInput, setYamlInput] = useState<string>(agent.CustomInstanceConfig || "");
  const { fireRequest } = useFetchWrapper();
  const posthog = usePostHog();

  const handleYamlChange = (value: string | undefined) => {
    setYamlInput(value || "");
  };

  const onSave = useCallback(() => {
    const isClearing = yamlInput.trim() === "";

    fireRequest({
      requestType: "POST",
      url: `/api/fleet-hub/${agent.InstanceIdStr}/config`,
      body: jsonStringify({
        config: yamlInput,
      }),
      successCb: () => {
        if (isClearing) {
          toast.success("Configuration cleared successfully", {
            description: "The custom collector configuration has been removed. The collector will use its default configuration."
          });
        } else {
          toast.success("Configuration saved successfully", {
            description: "The collector configuration has been updated and applied."
          });
        }
        fetchAgentInfo();
        posthog?.capture(CLIENT_EVENTS.FLEET_HUB_AGENT_CONFIG_SAVED, {
          agentId: agent.InstanceIdStr,
        });
      },
      failureCb: (resp) => {
        consoleLog(resp);
        // Extract error message from response
        let errorMessage = "Failed to save configuration";
        try {
          const errorData = typeof resp === 'string' ? JSON.parse(resp) : resp;
          errorMessage = errorData.error || errorMessage;
        } catch (e) {
          // If parsing fails, use the raw response as error message
          errorMessage = typeof resp === 'string' ? resp : errorMessage;
        }

        toast.error("Configuration validation failed", {
          description: errorMessage,
          duration: 5000
        });
      }
    })
  }, [yamlInput, agent.InstanceIdStr, fetchAgentInfo, posthog]);

  return (
    <div className="grid grid-cols-2 gap-3 grow text-stone-700 dark:text-stone-300">
      <div className="flex flex-col gap-3">
        <h4 className="font-semibold text-sm">Custom Configuration</h4>
        <CodeEditor value={yamlInput} onChange={handleYamlChange} language="yaml" />
        <Button variant="default" size={"sm"} className="rounded-none py-1 h-auto bg-primary/80 hover:bg-primary disabled:bg-stone-400 disabled:dark:text-stone-700 shrink-0" disabled={agent.CustomInstanceConfig === yamlInput} onClick={onSave}>Save</Button>
      </div>
      <div className="flex flex-col gap-3">
        <h4 className="font-semibold text-sm">Effective Configuration (readonly)</h4>
        <CodeEditor value={agent.EffectiveConfig} onChange={() => { }} language="yaml" readOnly />
      </div>
    </div>
  )
}