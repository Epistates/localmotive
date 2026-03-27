<script lang="ts">
  import { onMount } from "svelte";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Textarea } from "$lib/components/ui/textarea";
  import { Badge } from "$lib/components/ui/badge";
  import { Separator } from "$lib/components/ui/separator";
  import * as Card from "$lib/components/ui/card";
  import { Settings, Save, RotateCcw, FolderOpen } from "@lucide/svelte";
  import {
    getConfig,
    updateConfig,
    getFormats,
    getDefaultIgnorePatterns,
    pickDirectory,
  } from "$lib/api";
  import type { PipelineConfig, FormatInfo } from "$lib/api";
  import { toast } from "svelte-sonner";

  let config = $state<PipelineConfig | null>(null);
  let formats = $state<FormatInfo[]>([]);
  let defaultPatterns = $state<string[]>([]);
  let userPatternsText = $state("");
  let saving = $state(false);

  onMount(async () => {
    try {
      const [cfg, fmts, defaults] = await Promise.all([
        getConfig(),
        getFormats(),
        getDefaultIgnorePatterns(),
      ]);
      config = cfg;
      formats = fmts;
      defaultPatterns = defaults;
      userPatternsText = cfg.ignore.userPatterns.join("\n");
    } catch (e) {
      toast.error(`Failed to load config: ${e}`);
    }
  });

  async function handleSave() {
    if (!config) return;
    try {
      saving = true;
      config.ignore.userPatterns = userPatternsText
        .split("\n")
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
      await updateConfig(config);
      toast.success("Settings saved.");
    } catch (e) {
      toast.error(`Failed to save: ${e}`);
    } finally {
      saving = false;
    }
  }

  async function handleReset() {
    try {
      config = await getConfig();
      userPatternsText = config.ignore.userPatterns.join("\n");
      toast.info("Reset to defaults.");
    } catch (e) {
      toast.error(`Failed to reset: ${e}`);
    }
  }

  async function handlePickOutputDir() {
    if (!config) return;
    const dir = await pickDirectory();
    if (dir) {
      config.export.outputDirectory = dir;
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
</script>

<div class="flex h-full flex-col">
  <div class="flex items-center justify-between border-b px-6 py-4">
    <div>
      <h1 class="text-lg font-semibold">Settings</h1>
      <p class="text-sm text-muted-foreground">
        Configure pipeline defaults, ignore patterns, and export options.
      </p>
    </div>
    <div class="flex gap-2">
      <Button variant="outline" size="sm" onclick={handleReset}>
        <RotateCcw class="mr-2 h-3 w-3" />
        Reset
      </Button>
      <Button size="sm" onclick={handleSave} disabled={saving || !config}>
        <Save class="mr-2 h-3 w-3" />
        {saving ? "Saving..." : "Save"}
      </Button>
    </div>
  </div>

  {#if config}
    <div class="flex-1 overflow-auto p-6">
      <div class="mx-auto max-w-3xl space-y-6">
        <!-- Ignore Patterns -->
        <Card.Root>
          <Card.Header>
            <Card.Title>Ignore Patterns</Card.Title>
            <Card.Description>
              Files and directories to skip during scanning. Gitignore patterns
              are respected by default.
            </Card.Description>
          </Card.Header>
          <Card.Content class="space-y-4">
            <div>
              <label class="text-sm font-medium">Default Patterns</label>
              <div class="mt-2 flex flex-wrap gap-1">
                {#each defaultPatterns.slice(0, 20) as pattern}
                  <Badge variant="secondary" class="text-xs">{pattern}</Badge>
                {/each}
                {#if defaultPatterns.length > 20}
                  <Badge variant="outline" class="text-xs"
                    >+{defaultPatterns.length - 20} more</Badge
                  >
                {/if}
              </div>
            </div>
            <Separator />
            <div>
              <label class="text-sm font-medium" for="user-patterns"
                >Custom Patterns (one per line)</label
              >
              <Textarea
                id="user-patterns"
                bind:value={userPatternsText}
                placeholder={"e.g.\n*.test.ts\nfixtures/\n__mocks__"}
                rows={5}
                class="mt-2 font-mono text-sm"
              />
            </div>
            <div class="flex gap-4">
              <label class="flex items-center gap-2 text-sm">
                <input type="checkbox" bind:checked={config.ignore.useGitignore} />
                Respect .gitignore
              </label>
              <label class="flex items-center gap-2 text-sm">
                <input type="checkbox" bind:checked={config.ignore.skipBinary} />
                Skip binary files
              </label>
            </div>
            <div>
              <label class="text-sm font-medium" for="max-file-size"
                >Max file size (bytes)</label
              >
              <Input
                id="max-file-size"
                type="number"
                bind:value={config.ignore.maxFileSizeBytes}
                class="mt-1 w-48"
              />
            </div>
          </Card.Content>
        </Card.Root>

        <!-- Output Formats -->
        <Card.Root>
          <Card.Header>
            <Card.Title>Output Formats</Card.Title>
            <Card.Description>
              Select which model-specific formats to generate.
            </Card.Description>
          </Card.Header>
          <Card.Content>
            <div class="grid gap-3 sm:grid-cols-2">
              {#each formats as fmt}
                <button
                  type="button"
                  class="flex flex-col rounded-lg border p-3 text-left transition-colors {config.outputFormats.includes(
                    fmt.format,
                  )
                    ? 'border-primary bg-primary/5'
                    : 'border-border hover:border-muted-foreground/30'}"
                  onclick={() => toggleFormat(fmt.format)}
                >
                  <div class="flex items-center justify-between">
                    <span class="text-sm font-medium">{fmt.name}</span>
                    {#if fmt.supportsTools}
                      <Badge variant="outline" class="text-[10px]"
                        >tools</Badge
                      >
                    {/if}
                  </div>
                  <span class="mt-1 text-xs text-muted-foreground"
                    >{fmt.description}</span
                  >
                  <div class="mt-2 flex gap-1">
                    {#each fmt.models as model}
                      <Badge variant="secondary" class="text-[10px]"
                        >{model}</Badge
                      >
                    {/each}
                  </div>
                </button>
              {/each}
            </div>
          </Card.Content>
        </Card.Root>

        <!-- Generation Settings -->
        <Card.Root>
          <Card.Header>
            <Card.Title>Generation</Card.Title>
            <Card.Description>
              Control token budgets, export mode, and sample types.
            </Card.Description>
          </Card.Header>
          <Card.Content class="space-y-4">
            <div class="flex gap-4">
              <div>
                <label class="text-sm font-medium" for="token-budget"
                  >Token budget per example</label
                >
                <Input
                  id="token-budget"
                  type="number"
                  bind:value={config.generation.tokenBudget}
                  class="mt-1 w-36"
                />
              </div>
              <div>
                <label class="text-sm font-medium" for="max-context"
                  >Max context tokens</label
                >
                <Input
                  id="max-context"
                  type="number"
                  bind:value={config.generation.maxContextTokens}
                  class="mt-1 w-36"
                />
              </div>
            </div>
            <div>
              <label class="text-sm font-medium">Export Mode</label>
              <div class="mt-2 flex gap-3">
                <button
                  type="button"
                  class="rounded-md border px-4 py-2 text-sm transition-colors {config
                    .generation.exportMode === 'agentic'
                    ? 'border-primary bg-primary/5 font-medium'
                    : 'border-border hover:border-muted-foreground/30'}"
                  onclick={() => (config!.generation.exportMode = "agentic")}
                >
                  Agentic
                  <span class="block text-xs text-muted-foreground"
                    >With tool use</span
                  >
                </button>
                <button
                  type="button"
                  class="rounded-md border px-4 py-2 text-sm transition-colors {config
                    .generation.exportMode === 'conversational'
                    ? 'border-primary bg-primary/5 font-medium'
                    : 'border-border hover:border-muted-foreground/30'}"
                  onclick={() =>
                    (config!.generation.exportMode = "conversational")}
                >
                  Conversational
                  <span class="block text-xs text-muted-foreground"
                    >Text only</span
                  >
                </button>
              </div>
            </div>
          </Card.Content>
        </Card.Root>

        <!-- Quality Thresholds -->
        <Card.Root>
          <Card.Header>
            <Card.Title>Quality Filters</Card.Title>
            <Card.Description>
              Minimum quality thresholds for generated samples.
            </Card.Description>
          </Card.Header>
          <Card.Content>
            <div class="grid gap-4 sm:grid-cols-2">
              <div>
                <label class="text-sm font-medium" for="min-tokens"
                  >Min tokens per sample</label
                >
                <Input
                  id="min-tokens"
                  type="number"
                  bind:value={config.quality.minTokensPerSample}
                  class="mt-1"
                />
              </div>
              <div>
                <label class="text-sm font-medium" for="max-tokens"
                  >Max tokens per sample</label
                >
                <Input
                  id="max-tokens"
                  type="number"
                  bind:value={config.quality.maxTokensPerSample}
                  class="mt-1"
                />
              </div>
              <div>
                <label class="text-sm font-medium" for="dedup-threshold"
                  >Dedup Jaccard threshold</label
                >
                <Input
                  id="dedup-threshold"
                  type="number"
                  step="0.05"
                  min="0"
                  max="1"
                  bind:value={config.quality.dedupJaccardThreshold}
                  class="mt-1"
                />
              </div>
              <div>
                <label class="text-sm font-medium" for="min-alpha"
                  >Min alphanumeric fraction</label
                >
                <Input
                  id="min-alpha"
                  type="number"
                  step="0.05"
                  min="0"
                  max="1"
                  bind:value={config.quality.minAlphanumericFraction}
                  class="mt-1"
                />
              </div>
            </div>
          </Card.Content>
        </Card.Root>

        <!-- Export -->
        <Card.Root>
          <Card.Header>
            <Card.Title>Export</Card.Title>
            <Card.Description>
              Where and how to write the generated JSONL files.
            </Card.Description>
          </Card.Header>
          <Card.Content class="space-y-4">
            <div>
              <label class="text-sm font-medium">Output Directory</label>
              <div class="mt-1 flex gap-2">
                <Input
                  value={config.export.outputDirectory || "Not set"}
                  disabled
                  class="flex-1"
                />
                <Button variant="outline" size="sm" onclick={handlePickOutputDir}>
                  <FolderOpen class="h-4 w-4" />
                </Button>
              </div>
            </div>
            <div class="flex gap-4">
              <label class="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  bind:checked={config.export.splitByFormat}
                />
                Separate file per format
              </label>
              <label class="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  bind:checked={config.export.includeManifest}
                />
                Include manifest.json
              </label>
              <label class="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  bind:checked={config.export.includeStatistics}
                />
                Include statistics
              </label>
            </div>
          </Card.Content>
        </Card.Root>
      </div>
    </div>
  {:else}
    <div class="flex h-full items-center justify-center text-muted-foreground">
      <Settings class="mr-2 h-5 w-5 animate-spin" />
      Loading settings...
    </div>
  {/if}
</div>
