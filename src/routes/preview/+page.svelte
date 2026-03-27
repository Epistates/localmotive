<script lang="ts">
  import * as Card from "$lib/components/ui/card";
  import { Badge } from "$lib/components/ui/badge";
  import { Separator } from "$lib/components/ui/separator";
  import { Button } from "$lib/components/ui/button";
  import { ScrollArea } from "$lib/components/ui/scroll-area";
  import { Eye, FileJson, BarChart3 } from "@lucide/svelte";
  import { projectStore } from "$lib/stores/projects.svelte.js";

  let lastResult = $derived(projectStore.lastResult);
  let activeTab = $state<"files" | "stats">("files");
</script>

<div class="flex h-full flex-col">
  <div class="flex items-center justify-between border-b px-6 py-4">
    <div>
      <h1 class="text-lg font-semibold">Preview</h1>
      <p class="text-sm text-muted-foreground">
        {#if lastResult}
          {lastResult.totalSamples} samples generated across {lastResult.outputFiles.length} file(s)
        {:else}
          No generation results to preview
        {/if}
      </p>
    </div>
    <div class="flex gap-1">
      <Button
        variant={activeTab === "files" ? "default" : "outline"}
        size="sm"
        onclick={() => (activeTab = "files")}
      >
        <FileJson class="mr-2 h-3 w-3" />
        Output Files
      </Button>
      <Button
        variant={activeTab === "stats" ? "default" : "outline"}
        size="sm"
        onclick={() => (activeTab = "stats")}
      >
        <BarChart3 class="mr-2 h-3 w-3" />
        Statistics
      </Button>
    </div>
  </div>

  <div class="flex-1 overflow-auto p-6">
    {#if !lastResult}
      <div class="flex h-full flex-col items-center justify-center text-muted-foreground">
        <Eye class="mb-4 h-12 w-12 opacity-40" />
        <p class="text-lg font-medium">No results yet</p>
        <p class="mt-1 text-sm">
          Go to <a href="/generate" class="underline">Generate</a> to run the pipeline first.
        </p>
      </div>
    {:else if activeTab === "files"}
      <div class="mx-auto max-w-3xl space-y-4">
        <!-- Output Files -->
        <Card.Root>
          <Card.Header>
            <Card.Title>Generated Files</Card.Title>
            <Card.Description>
              {lastResult.outputFiles.length} file(s) written in {(lastResult.totalDurationMs / 1000).toFixed(1)}s
            </Card.Description>
          </Card.Header>
          <Card.Content>
            <div class="space-y-2">
              {#each lastResult.outputFiles as file}
                <div class="flex items-center gap-3 rounded-md border p-3">
                  <FileJson class="h-5 w-5 text-muted-foreground" />
                  <div class="flex-1 min-w-0">
                    <div class="truncate text-sm font-mono">{file.split("/").pop()}</div>
                    <div class="truncate text-xs text-muted-foreground">{file}</div>
                  </div>
                </div>
              {/each}
            </div>
          </Card.Content>
        </Card.Root>

        <!-- Samples by Type -->
        <Card.Root>
          <Card.Header>
            <Card.Title>Samples by Type</Card.Title>
          </Card.Header>
          <Card.Content>
            <div class="space-y-2">
              {#each Object.entries(lastResult.samplesByType) as [type, count]}
                <div class="flex items-center justify-between">
                  <span class="text-sm">{type}</span>
                  <div class="flex items-center gap-2">
                    <div
                      class="h-2 rounded-full bg-primary"
                      style="width: {Math.max(4, (count / lastResult.totalSamples) * 200)}px"
                    ></div>
                    <span class="text-sm font-medium w-12 text-right">{count}</span>
                  </div>
                </div>
              {/each}
            </div>
          </Card.Content>
        </Card.Root>

        <!-- Quality Stats -->
        <Card.Root>
          <Card.Header>
            <Card.Title>Quality Pipeline</Card.Title>
          </Card.Header>
          <Card.Content>
            <div class="grid grid-cols-2 gap-4 sm:grid-cols-4">
              <div class="text-center">
                <div class="text-2xl font-bold">{lastResult.qualityStats.totalInput}</div>
                <div class="text-xs text-muted-foreground">Generated</div>
              </div>
              <div class="text-center">
                <div class="text-2xl font-bold text-green-500">{lastResult.qualityStats.passed}</div>
                <div class="text-xs text-muted-foreground">Passed</div>
              </div>
              <div class="text-center">
                <div class="text-2xl font-bold text-yellow-500">{lastResult.qualityStats.filtered}</div>
                <div class="text-xs text-muted-foreground">Filtered</div>
              </div>
              <div class="text-center">
                <div class="text-2xl font-bold text-red-500">{lastResult.qualityStats.duplicatesRemoved}</div>
                <div class="text-xs text-muted-foreground">Deduped</div>
              </div>
            </div>
          </Card.Content>
        </Card.Root>
      </div>
    {:else}
      <!-- Statistics Tab -->
      <div class="mx-auto max-w-3xl">
        <Card.Root>
          <Card.Header>
            <Card.Title>Pipeline Summary</Card.Title>
          </Card.Header>
          <Card.Content class="space-y-4">
            <div class="grid grid-cols-3 gap-4">
              <div class="rounded-lg border p-4 text-center">
                <div class="text-3xl font-bold">{lastResult.totalSamples}</div>
                <div class="text-sm text-muted-foreground">Total Samples</div>
              </div>
              <div class="rounded-lg border p-4 text-center">
                <div class="text-3xl font-bold">{lastResult.outputFiles.length}</div>
                <div class="text-sm text-muted-foreground">Output Files</div>
              </div>
              <div class="rounded-lg border p-4 text-center">
                <div class="text-3xl font-bold">
                  {(lastResult.totalDurationMs / 1000).toFixed(1)}s
                </div>
                <div class="text-sm text-muted-foreground">Duration</div>
              </div>
            </div>
            <Separator />
            <div>
              <h3 class="mb-2 text-sm font-medium">Sample Distribution</h3>
              {#each Object.entries(lastResult.samplesByType) as [type, count]}
                <div class="flex items-center justify-between py-1">
                  <Badge variant="secondary">{type}</Badge>
                  <span class="text-sm font-mono">
                    {count}
                    <span class="text-muted-foreground">
                      ({((count / lastResult.totalSamples) * 100).toFixed(0)}%)
                    </span>
                  </span>
                </div>
              {/each}
            </div>
          </Card.Content>
        </Card.Root>
      </div>
    {/if}
  </div>
</div>
