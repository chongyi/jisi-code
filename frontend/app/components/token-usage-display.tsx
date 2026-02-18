import { Gauge, Zap } from "lucide-react";

import { cn } from "~/lib/utils";
import type { TokenUsage } from "~/types/websocket";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "~/components/ui/tooltip";

interface TokenUsageDisplayProps {
  usage: TokenUsage | undefined;
  className?: string;
}

export function TokenUsageDisplay({ usage, className }: TokenUsageDisplayProps) {
  if (!usage) {
    return null;
  }

  const {
    input_tokens = 0,
    output_tokens = 0,
    total_tokens = 0,
    context_window = 200000,
    remaining_tokens,
  } = usage;

  const usedTokens = total_tokens || input_tokens + output_tokens;
  const remaining = remaining_tokens ?? Math.max(0, context_window - usedTokens);
  const usageRatio = context_window > 0 ? usedTokens / context_window : 0;
  const percentage = Math.min(100, Math.max(0, usageRatio * 100));

  const statusClass =
    percentage >= 90
      ? "text-red-600"
      : percentage >= 70
        ? "text-amber-600"
        : "text-emerald-600";

  const barClass =
    percentage >= 90
      ? "bg-red-500"
      : percentage >= 70
        ? "bg-amber-500"
        : "bg-emerald-500";

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <div
            className={cn(
              "flex min-w-[170px] items-center gap-2 rounded-md border bg-card px-2 py-1.5",
              className
            )}
          >
            <Gauge className={cn("size-3.5 shrink-0", statusClass)} />
            <div className="min-w-0 flex-1">
              <div className="flex items-center justify-between gap-2 text-[11px]">
                <span className="text-muted-foreground">Context left</span>
                <span className={cn("font-medium", statusClass)}>
                  {formatNumber(remaining)}
                </span>
              </div>
              <div className="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-muted">
                <div
                  className={cn("h-full transition-all duration-300", barClass)}
                  style={{ width: `${percentage}%` }}
                />
              </div>
            </div>
            <Zap className="size-3 shrink-0 text-muted-foreground" />
          </div>
        </TooltipTrigger>
        <TooltipContent side="bottom" className="text-xs">
          <div className="space-y-1">
            <div className="font-medium">Context Usage</div>
            <div className="grid grid-cols-2 gap-x-4 gap-y-0.5 text-muted-foreground">
              <span>Input:</span>
              <span className="text-right">{formatNumber(input_tokens)}</span>
              <span>Output:</span>
              <span className="text-right">{formatNumber(output_tokens)}</span>
              <span>Total:</span>
              <span className="text-right">{formatNumber(usedTokens)}</span>
              <span>Window:</span>
              <span className="text-right">{formatNumber(context_window)}</span>
            </div>
          </div>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

function formatNumber(num: number): string {
  if (num >= 1_000_000) {
    return `${(num / 1_000_000).toFixed(1)}M`;
  }
  if (num >= 1_000) {
    return `${(num / 1_000).toFixed(1)}K`;
  }
  return num.toString();
}
