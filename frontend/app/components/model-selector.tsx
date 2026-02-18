import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "~/components/ui/select";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "~/components/ui/tooltip";
import type {
  AgentInfo,
  AgentType,
  ModelConfig,
} from "~/types/websocket";
import {
  AGENT_CAPABILITIES,
  REASONING_EFFORT_OPTIONS,
} from "~/types/websocket";

interface ModelSelectorProps {
  agent: AgentInfo | undefined;
  config: ModelConfig | null;
  onChange: (config: ModelConfig) => void;
  disabled?: boolean;
}

export function ModelSelector({
  agent,
  config,
  onChange,
  disabled,
}: ModelSelectorProps) {
  if (!agent) {
    return null;
  }

  const capabilities = AGENT_CAPABILITIES[agent.agent_type as AgentType];
  const models = capabilities?.defaultModels ?? [];
  const supportsReasoningEffort = capabilities?.supportsReasoningEffort ?? false;
  const baseConfig = config ?? {};

  if (models.length === 0 && !supportsReasoningEffort) {
    return null;
  }

  return (
    <div className="flex items-center gap-2">
      <TooltipProvider>
        {models.length > 0 && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Select
                value={config?.model ?? ""}
                onValueChange={(value) =>
                  onChange({ ...baseConfig, model: value || undefined })
                }
                disabled={disabled}
              >
                <SelectTrigger className="h-8 w-auto min-w-[160px] bg-background text-xs">
                  <SelectValue placeholder="Select model">
                    {config?.model ? (
                      <span className="truncate">
                        {models.find((m) => m.id === config.model)?.display_name ??
                          config.model}
                      </span>
                    ) : (
                      "Model"
                    )}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  {models.map((model) => (
                    <SelectItem
                      key={model.id}
                      value={model.id}
                      className="text-xs"
                    >
                      <div className="flex flex-col">
                        <span>{model.display_name}</span>
                        {model.description && (
                          <span className="text-xs text-muted-foreground">
                            {model.description}
                          </span>
                        )}
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              <p className="text-xs">Select AI model for this session</p>
            </TooltipContent>
          </Tooltip>
        )}

        {supportsReasoningEffort && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Select
                value={config?.reasoning_effort ?? ""}
                onValueChange={(value) =>
                  onChange({
                    ...baseConfig,
                    reasoning_effort: value as "low" | "medium" | "high",
                  })
                }
                disabled={disabled}
              >
                <SelectTrigger className="h-8 w-auto min-w-[110px] bg-background text-xs">
                  <SelectValue placeholder="Effort">
                    {config?.reasoning_effort
                      ? REASONING_EFFORT_OPTIONS.find(
                          (e) => e.id === config.reasoning_effort
                        )?.display_name
                      : "Effort"}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  {REASONING_EFFORT_OPTIONS.map((option) => (
                    <SelectItem
                      key={option.id}
                      value={option.id}
                      className="text-xs"
                    >
                      <div className="flex flex-col">
                        <span>{option.display_name}</span>
                        <span className="text-xs text-muted-foreground">
                          {option.description}
                        </span>
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              <p className="text-xs">Set reasoning effort level</p>
            </TooltipContent>
          </Tooltip>
        )}
      </TooltipProvider>
    </div>
  );
}

interface CompactModelSelectorProps {
  agent: AgentInfo | undefined;
  config: ModelConfig | null;
  onChange: (config: ModelConfig) => void;
  disabled?: boolean;
}

export function CompactModelSelector({
  agent,
  config,
  onChange,
  disabled,
}: CompactModelSelectorProps) {
  if (!agent) {
    return null;
  }

  const capabilities = AGENT_CAPABILITIES[agent.agent_type as AgentType];
  const models = capabilities?.defaultModels ?? [];
  const supportsReasoningEffort = capabilities?.supportsReasoningEffort ?? false;

  if (models.length === 0 && !supportsReasoningEffort) {
    return null;
  }

  const currentModel = config?.model
    ? models.find((m) => m.id === config.model)?.display_name ?? config.model
    : null;

  const currentEffort = config?.reasoning_effort
    ? REASONING_EFFORT_OPTIONS.find((e) => e.id === config.reasoning_effort)
        ?.display_name
    : null;

  return (
    <div className="flex items-center gap-1">
      {currentModel && (
        <span className="rounded bg-muted px-1.5 py-0.5 text-xs">
          {currentModel}
        </span>
      )}
      {currentEffort && (
        <span className="rounded bg-muted px-1.5 py-0.5 text-xs">
          {currentEffort}
        </span>
      )}
    </div>
  );
}
