<script lang="ts">
  import { Progress } from "$lib/components/ui/progress";
  import { Badge } from "$lib/components/ui/badge";
  import * as Card from "$lib/components/ui/card";
  import { Separator } from "$lib/components/ui/separator";
  import { Button } from "$lib/components/ui/button";
  import {
    Play,
    CheckCircle,
    XCircle,
    Loader2,
    StopCircle,
  } from "@lucide/svelte";
  import { cancelGeneration } from "$lib/api";
  import type { ProgressEvent } from "$lib/api";
  import { projectStore } from "$lib/stores/projects.svelte.js";
  import { toast } from "svelte-sonner";

  let events = $derived(projectStore.progressEvents);
  let isGenerating = $derived(projectStore.isGenerating);
  let lastResult = $derived(projectStore.lastResult);

  // Derived state from events
  let currentPhase = $derived.by(() => {
    for (let i = events.length - 1; i >= 0; i--) {
      const e = events[i];
      if (e.event === "phaseStarted") return e.data.phase;
      if (e.event === "phaseCompleted") return null;
      if (e.event === "pipelineCompleted") return null;
    }
    return null;
  });

  let latestProgress = $derived.by(() => {
    for (let i = events.length - 1; i >= 0; i--) {
      const e = events[i];
      if (e.event === "phaseProgress") return e.data;
    }
    return null;
  });

  let completedPhases = $derived.by(() => {
    const phases: { phase: string; durationMs: number; itemsProcessed: number }[] = [];
    for (const e of events) {
      if (e.event === "phaseCompleted") {
        phases.push(e.data);
      }
    }
    return phases;
  });

  let qualityReport = $derived.by(() => {
    for (let i = events.length - 1; i >= 0; i--) {
      const e = events[i];
      if (e.event === "qualityReport") return e.data;
    }
    return null;
  });

  let samplesInfo = $derived.by(() => {
    for (let i = events.length - 1; i >= 0; i--) {
      const e = events[i];
      if (e.event === "samplesGenerated") return e.data;
    }
    return null;
  });

  let pipelineComplete = $derived.by(() => {
    return events.some((e) => e.event === "pipelineCompleted");
  });

  let progressPercent = $derived.by(() => {
    if (!latestProgress) return 0;
    if (latestProgress.total === 0) return 0;
    return Math.round((latestProgress.completed / latestProgress.total) * 100);
  });

  function phaseLabel(phase: string): string {
    const labels: Record<string, string> = {
      scanning: "Scanning Files",
      analyzing: "Analyzing ASTs",
      generating: "Generating Samples",
      qualityFiltering: "Quality Filtering",
      formatting: "Formatting Output",
      exporting: "Exporting JSONL",
    };
    return labels[phase] ?? phase;
  }

  async function handleCancel() {
    try {
      await cancelGeneration();
      toast.info("Cancelling pipeline...");
    } catch (e) {
      toast.error(`Cancel failed: ${e}`);
    }
  }
</script>

<div class="flex h-full flex-col">
  <div class="flex items-center justify-between border-b px-6 py-4">
    <div>
      <h1 class="text-lg font-semibold">Pipeline Progress</h1>
      <p class="text-sm text-muted-foreground">
        {#if isGenerating}
          Generation in progress...
        {:else if pipelineComplete}
          Generation complete
        {:else}
          Waiting for generation to start
        {/if}
      </p>
    </div>
    {#if isGenerating}
      <Button variant="destructive" size="sm" onclick={handleCancel}>
        <StopCircle class="mr-2 h-4 w-4" />
        Cancel
      </Button>
    {/if}
  </div>

  <div class="flex-1 overflow-auto p-6">
    <div class="mx-auto max-w-3xl space-y-6">
      {#if events.length === 0 && !lastResult}
        <div class="flex flex-col items-center justify-center py-20 text-muted-foreground">
          <Play class="mb-4 h-12 w-12 opacity-40" />
          <p class="text-lg font-medium">No active pipeline</p>
          <p class="mt-1 text-sm">
            Go to <a href="/generate" class="underline">Generate</a> to start a pipeline run.
          </p>
        </div>
      {:else}
        <!-- Current Phase Progress -->
        {#if currentPhase && latestProgress}
          <Card.Root>
            <Card.Header>
              <div class="flex items-center justify-between">
                <Card.Title class="flex items-center gap-2">
                  <Loader2 class="h-4 w-4 animate-spin" />
                  {phaseLabel(currentPhase)}
                </Card.Title>
                <Badge>{progressPercent}%</Badge>
              </div>
            </Card.Header>
            <Card.Content class="space-y-3">
              <Progress value={progressPercent} class="h-2" />
              <div class="flex justify-between text-sm text-muted-foreground">
                <span>{latestProgress.completed} / {latestProgress.total}</span>
                {#if latestProgress.currentItem}
                  <span class="truncate ml-4 max-w-[300px]">
                    {latestProgress.currentItem}
                  </span>
                {/if}
              </div>
            </Card.Content>
          </Card.Root>
        {/if}

        <!-- Completed Phases -->
        {#if completedPhases.length > 0}
          <Card.Root>
            <Card.Header>
              <Card.Title>Completed Phases</Card.Title>
            </Card.Header>
            <Card.Content>
              <div class="space-y-2">
                {#each completedPhases as phase}
                  <div class="flex items-center justify-between text-sm">
                    <div class="flex items-center gap-2">
                      <CheckCircle class="h-4 w-4 text-green-500" />
                      <span>{phaseLabel(phase.phase)}</span>
                    </div>
                    <div class="flex items-center gap-3 text-muted-foreground">
                      <span>{phase.itemsProcessed} items</span>
                      <span>{(phase.durationMs / 1000).toFixed(1)}s</span>
                    </div>
                  </div>
                {/each}
              </div>
            </Card.Content>
          </Card.Root>
        {/if}

        <!-- Samples Generated -->
        {#if samplesInfo}
          <Card.Root>
            <Card.Header>
              <Card.Title>Samples Generated: {samplesInfo.count}</Card.Title>
            </Card.Header>
            <Card.Content>
              <div class="flex flex-wrap gap-2">
                {#each samplesInfo.byType as [type, count]}
                  <Badge variant="secondary">
                    {type}: {count}
                  </Badge>
                {/each}
              </div>
            </Card.Content>
          </Card.Root>
        {/if}

        <!-- Quality Report -->
        {#if qualityReport}
          <Card.Root>
            <Card.Header>
              <Card.Title>Quality Report</Card.Title>
            </Card.Header>
            <Card.Content>
              <div class="grid grid-cols-2 gap-4 sm:grid-cols-4">
                <div class="text-center">
                  <div class="text-2xl font-bold">{qualityReport.totalSamples}</div>
                  <div class="text-xs text-muted-foreground">Total Input</div>
                </div>
                <div class="text-center">
                  <div class="text-2xl font-bold text-green-500">{qualityReport.passed}</div>
                  <div class="text-xs text-muted-foreground">Passed</div>
                </div>
                <div class="text-center">
                  <div class="text-2xl font-bold text-yellow-500">{qualityReport.filtered}</div>
                  <div class="text-xs text-muted-foreground">Filtered</div>
                </div>
                <div class="text-center">
                  <div class="text-2xl font-bold text-red-500">{qualityReport.duplicatesRemoved}</div>
                  <div class="text-xs text-muted-foreground">Duplicates</div>
                </div>
              </div>
            </Card.Content>
          </Card.Root>
        {/if}

        <!-- Pipeline Complete -->
        {#if lastResult}
          <Card.Root class="border-green-500/30">
            <Card.Header>
              <Card.Title class="flex items-center gap-2 text-green-500">
                <CheckCircle class="h-5 w-5" />
                Pipeline Complete
              </Card.Title>
            </Card.Header>
            <Card.Content class="space-y-3">
              <div class="grid grid-cols-2 gap-4">
                <div>
                  <span class="text-sm text-muted-foreground">Total Samples</span>
                  <div class="text-xl font-bold">{lastResult.totalSamples}</div>
                </div>
                <div>
                  <span class="text-sm text-muted-foreground">Duration</span>
                  <div class="text-xl font-bold">
                    {(lastResult.totalDurationMs / 1000).toFixed(1)}s
                  </div>
                </div>
              </div>
              <Separator />
              <div>
                <span class="text-sm font-medium">Output Files</span>
                <div class="mt-1 space-y-1">
                  {#each lastResult.outputFiles as file}
                    <div class="truncate text-xs font-mono text-muted-foreground">
                      {file}
                    </div>
                  {/each}
                </div>
              </div>
              <div class="flex gap-2 pt-2">
                <Button size="sm" variant="outline" href="/preview">
                  View Samples
                </Button>
                <Button size="sm" variant="outline" href="/generate">
                  Generate Again
                </Button>
              </div>
            </Card.Content>
          </Card.Root>
        {/if}
      {/if}
    </div>
  </div>
</div>
