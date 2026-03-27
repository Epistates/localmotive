<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { Button } from "$lib/components/ui/button";
  import { Badge } from "$lib/components/ui/badge";
  import { Input } from "$lib/components/ui/input";
  import * as Card from "$lib/components/ui/card";
  import { Separator } from "$lib/components/ui/separator";
  import {
    Cpu,
    Play,
    FolderOpen,
    Zap,
    MessageSquare,
  } from "@lucide/svelte";
  import {
    getConfig,
    getFormats,
    startGeneration,
    pickDirectory,
  } from "$lib/api";
  import type { PipelineConfig, FormatInfo, ProgressEvent } from "$lib/api";
  import { toast } from "svelte-sonner";
  import { projectStore } from "$lib/stores/projects.svelte.js";

  let config = $state<PipelineConfig | null>(null);
  let formats = $state<FormatInfo[]>([]);
  let outputDir = $state("");
  let generating = $state(false);

  let scannedProjects = $derived(projectStore.scannedProjects);
  let selectedId = $derived(projectStore.selectedProjectId);

  onMount(async () => {
    try {
      const [cfg, fmts] = await Promise.all([getConfig(), getFormats()]);
      config = cfg;
      formats = fmts;
      outputDir = cfg.export.outputDirectory;
    } catch (e) {
      toast.error(`Failed to load config: ${e}`);
    }
  });

  async function handlePickOutputDir() {
    const dir = await pickDirectory();
    if (dir) {
      outputDir = dir;
      if (config) config.export.outputDirectory = dir;
    }
  }

  function toggleFormat(format: string) {
    if (!config) return;
    const idx = config.outputFormats.indexOf(format as any);
    if (idx >= 0) {
      config.outputFormats = config.outputFormats.filter((_, i) => i !== idx);
    } else {
      config.outputFormats = [...config.outputFormats, format as any];
    }
  }

  async function handleGenerate() {
    if (!config || !selectedId || !outputDir) {
      toast.error("Select a project and output directory first.");
      return;
    }

    generating = true;
    projectStore.isGenerating = true;
    projectStore.clearProgressEvents();
    config.export.outputDirectory = outputDir;

    toast.info("Starting generation pipeline...");
    await goto("/progress");

    try {
      const result = await startGeneration(
        selectedId,
        config,
        outputDir,
        (event: ProgressEvent) => {
          projectStore.addProgressEvent(event);
        },
      );
      projectStore.lastResult = result;
      toast.success(
        `Generated ${result.totalSamples} samples in ${(result.totalDurationMs / 1000).toFixed(1)}s`,
      );
    } catch (e) {
      toast.error(`Generation failed: ${e}`);
    } finally {
      generating = false;
      projectStore.isGenerating = false;
    }
  }
</script>

<div class="flex h-full flex-col">
  <div class="flex items-center justify-between border-b px-6 py-4">
    <div>
      <h1 class="text-lg font-semibold">Generate Training Data</h1>
      <p class="text-sm text-muted-foreground">
        Configure and run the generation pipeline.
      </p>
    </div>
    <Button
      onclick={handleGenerate}
      disabled={generating || !selectedId || !outputDir || !config}
    >
      <Play class="mr-2 h-4 w-4" />
      Generate
    </Button>
  </div>

  {#if config}
    <div class="flex-1 overflow-auto p-6">
      <div class="mx-auto max-w-3xl space-y-6">
        <!-- Project Selection -->
        <Card.Root>
          <Card.Header>
            <Card.Title>Project</Card.Title>
            <Card.Description>
              Select a scanned project to generate training data from.
            </Card.Description>
          </Card.Header>
          <Card.Content>
            {#if scannedProjects.length === 0}
              <p class="text-sm text-muted-foreground">
                No projects scanned yet. Go to
                <a href="/" class="underline">Projects</a> to scan one first.
              </p>
            {:else}
              <div class="grid gap-2">
                {#each scannedProjects as project}
                  <button
                    type="button"
                    class="flex items-center justify-between rounded-lg border p-3 text-left transition-colors {selectedId ===
                    project.id
                      ? 'border-primary bg-primary/5'
                      : 'border-border hover:border-muted-foreground/30'}"
                    onclick={() => projectStore.selectedProjectId = project.id}
                  >
                    <div>
                      <span class="text-sm font-medium">{project.name}</span>
                      <span class="ml-2 text-xs text-muted-foreground">
                        {project.fileCount} files, {project.totalLines.toLocaleString()} lines
                      </span>
                    </div>
                    <Badge variant="outline">{project.projectType}</Badge>
                  </button>
                {/each}
              </div>
            {/if}
          </Card.Content>
        </Card.Root>

        <!-- Output Formats -->
        <Card.Root>
          <Card.Header>
            <Card.Title>Output Formats</Card.Title>
          </Card.Header>
          <Card.Content>
            <div class="grid gap-2 sm:grid-cols-2">
              {#each formats as fmt}
                <button
                  type="button"
                  class="flex items-center gap-3 rounded-lg border p-3 text-left transition-colors {config.outputFormats.includes(
                    fmt.format,
                  )
                    ? 'border-primary bg-primary/5'
                    : 'border-border hover:border-muted-foreground/30'}"
                  onclick={() => toggleFormat(fmt.format)}
                >
                  <div class="flex-1">
                    <div class="flex items-center gap-2">
                      <span class="text-sm font-medium">{fmt.name}</span>
                      {#if fmt.supportsTools}
                        <Badge variant="outline" class="text-[10px]">tools</Badge>
                      {/if}
                    </div>
                    <span class="text-xs text-muted-foreground">{fmt.models.join(", ")}</span>
                  </div>
                </button>
              {/each}
            </div>
          </Card.Content>
        </Card.Root>

        <!-- Export Mode -->
        <Card.Root>
          <Card.Header>
            <Card.Title>Export Mode</Card.Title>
          </Card.Header>
          <Card.Content>
            <div class="flex gap-3">
              <button
                type="button"
                class="flex flex-1 items-center gap-3 rounded-lg border p-4 transition-colors {config
                  .generation.exportMode === 'agentic'
                  ? 'border-primary bg-primary/5'
                  : 'border-border hover:border-muted-foreground/30'}"
                onclick={() => (config!.generation.exportMode = "agentic")}
              >
                <Zap class="h-5 w-5 text-primary" />
                <div>
                  <div class="text-sm font-medium">Agentic</div>
                  <div class="text-xs text-muted-foreground">
                    Structured tool use with function-calling schema
                  </div>
                </div>
              </button>
              <button
                type="button"
                class="flex flex-1 items-center gap-3 rounded-lg border p-4 transition-colors {config
                  .generation.exportMode === 'conversational'
                  ? 'border-primary bg-primary/5'
                  : 'border-border hover:border-muted-foreground/30'}"
                onclick={() => (config!.generation.exportMode = "conversational")}
              >
                <MessageSquare class="h-5 w-5 text-primary" />
                <div>
                  <div class="text-sm font-medium">Conversational</div>
                  <div class="text-xs text-muted-foreground">
                    Clean text-only, no tool noise
                  </div>
                </div>
              </button>
            </div>
          </Card.Content>
        </Card.Root>

        <!-- Token Budget & Output -->
        <Card.Root>
          <Card.Header>
            <Card.Title>Output Settings</Card.Title>
          </Card.Header>
          <Card.Content class="space-y-4">
            <div class="flex gap-4">
              <div>
                <label class="text-sm font-medium" for="token-budget">
                  Token budget per example
                </label>
                <Input
                  id="token-budget"
                  type="number"
                  bind:value={config.generation.tokenBudget}
                  class="mt-1 w-36"
                />
              </div>
              <div>
                <label class="text-sm font-medium" for="max-context">
                  Max context tokens
                </label>
                <Input
                  id="max-context"
                  type="number"
                  bind:value={config.generation.maxContextTokens}
                  class="mt-1 w-36"
                />
              </div>
            </div>
            <Separator />
            <div>
              <label class="text-sm font-medium">Output Directory</label>
              <div class="mt-1 flex gap-2">
                <Input
                  value={outputDir || "Select a directory..."}
                  disabled
                  class="flex-1"
                />
                <Button
                  variant="outline"
                  size="sm"
                  onclick={handlePickOutputDir}
                >
                  <FolderOpen class="h-4 w-4" />
                </Button>
              </div>
            </div>
          </Card.Content>
        </Card.Root>
      </div>
    </div>
  {:else}
    <div class="flex h-full items-center justify-center text-muted-foreground">
      <Cpu class="mr-2 h-5 w-5 animate-spin" />
      Loading configuration...
    </div>
  {/if}
</div>
