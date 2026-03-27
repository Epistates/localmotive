<script lang="ts">
  import * as Card from "$lib/components/ui/card";
  import * as Dialog from "$lib/components/ui/dialog";
  import { Badge } from "$lib/components/ui/badge";
  import { Separator } from "$lib/components/ui/separator";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import {
    Eye,
    FileJson,
    BarChart3,
    Upload,
    Loader2,
    ExternalLink,
    Check,
  } from "@lucide/svelte";
  import { projectStore } from "$lib/stores/projects.svelte.js";
  import {
    getHfToken,
    validateHfToken,
    publishDataset,
    getConfig,
  } from "$lib/api";
  import type {
    WhoamiResponse,
    PublishProgress,
    PublishResult,
    OutputFormat,
  } from "$lib/api";
  import { toast } from "svelte-sonner";

  let lastResult = $derived(projectStore.lastResult);
  let activeTab = $state<"files" | "stats">("files");

  // Publish dialog state
  let publishOpen = $state(false);
  let hfUser = $state<WhoamiResponse | null>(null);
  let repoName = $state("");
  let selectedNamespace = $state<string | null>(null);
  let isPrivate = $state(false);
  let license = $state("mit");
  let selectedFiles = $state<string[]>([]);
  let publishing = $state(false);
  let publishProgress = $state<PublishProgress | null>(null);
  let publishResult = $state<PublishResult | null>(null);
  let publishError = $state<string | null>(null);
  let storedFormats = $state<OutputFormat[]>([]);
  let repoNameError = $state<string | null>(null);

  const licenses = [
    { value: "mit", label: "MIT" },
    { value: "apache-2.0", label: "Apache 2.0" },
    { value: "cc-by-4.0", label: "CC BY 4.0" },
    { value: "cc-by-sa-4.0", label: "CC BY-SA 4.0" },
    { value: "cc-by-nc-4.0", label: "CC BY-NC 4.0" },
  ];

  const REPO_NAME_RE = /^[a-zA-Z0-9][a-zA-Z0-9._-]{0,95}$/;

  let namespaceOptions = $derived.by(() => {
    if (!hfUser) return [];
    const options = [{ value: hfUser.name, label: hfUser.name }];
    for (const org of hfUser.orgs) {
      options.push({ value: org.name, label: org.name });
    }
    return options;
  });

  function resetDialogState() {
    publishResult = null;
    publishError = null;
    publishProgress = null;
    repoNameError = null;
    repoName = "";
    selectedNamespace = null;
    isPrivate = false;
    license = "mit";
    selectedFiles = [];
    storedFormats = [];
    hfUser = null;
  }

  async function openPublishDialog() {
    resetDialogState();

    // Validate stored token
    let token: string | null;
    try {
      token = await getHfToken();
      if (!token) {
        toast.error("No Hugging Face token configured. Go to Settings to add one.");
        return;
      }
    } catch {
      toast.error("Failed to read token. Go to Settings to reconfigure.");
      return;
    }

    try {
      hfUser = await validateHfToken(token);
    } catch (e) {
      const msg = String(e);
      if (msg.includes("Invalid") || msg.includes("expired") || msg.includes("401")) {
        toast.error("Hugging Face token is invalid or expired. Go to Settings to update it.");
      } else {
        toast.error(`Could not connect to Hugging Face: ${msg}`);
      }
      return;
    }

    // Load config for formats
    try {
      const cfg = await getConfig();
      storedFormats = cfg.outputFormats;
    } catch {
      toast.warning("Could not load config. Dataset card metadata may be incomplete.");
      storedFormats = [];
    }

    // Set defaults
    selectedNamespace = hfUser.name;
    selectedFiles = lastResult?.outputFiles.filter((f) => f.endsWith(".jsonl")) ?? [];

    // Derive repo name from the first output filename, sanitized for HF
    if (lastResult && lastResult.outputFiles.length > 0) {
      const first = lastResult.outputFiles[0].split("/").pop() ?? "";
      const parts = first.split("_");
      const raw = (parts[0] || "training-data") + "-training-data";
      repoName = raw
        .toLowerCase()
        .replace(/[^a-z0-9._-]/g, "-")
        .replace(/-{2,}/g, "-")
        .replace(/\.{2,}/g, ".")
        .replace(/^[.\-]+/, "")
        .replace(/[.\-]+$/, "")
        .slice(0, 96) || "training-data";
    }

    publishOpen = true;
  }

  function validateRepoName(name: string): string | null {
    if (!name.trim()) return "Repository name is required";
    if (!REPO_NAME_RE.test(name)) {
      return "Use lowercase letters, digits, hyphens, underscores, or dots. Must start with alphanumeric.";
    }
    return null;
  }

  function toggleFile(file: string) {
    if (selectedFiles.includes(file)) {
      selectedFiles = selectedFiles.filter((f) => f !== file);
    } else {
      selectedFiles = [...selectedFiles, file];
    }
  }

  async function handlePublish() {
    if (!lastResult || !hfUser) return;

    // Validate repo name
    const nameErr = validateRepoName(repoName);
    if (nameErr) {
      repoNameError = nameErr;
      return;
    }

    try {
      publishing = true;
      publishError = null;

      const result = await publishDataset(
        {
          repoName,
          namespace: selectedNamespace,
          private: isPrivate,
          license,
          outputFiles: selectedFiles,
        },
        lastResult,
        projectStore.selectedProject?.name ?? "dataset",
        storedFormats,
        (progress: PublishProgress) => {
          publishProgress = progress;
        },
      );

      publishResult = result;
      toast.success("Dataset published to Hugging Face!");
    } catch (e) {
      publishError = `${e}`;
      toast.error(`Publish failed: ${e}`);
    } finally {
      publishing = false;
    }
  }

  function handlePublishAgain() {
    publishResult = null;
    publishError = null;
    publishProgress = null;
  }

  let progressLabel = $derived.by(() => {
    if (!publishProgress) return "";
    switch (publishProgress.type) {
      case "creatingRepo":
        return "Creating repository...";
      case "generatingCard":
        return "Generating dataset card...";
      case "uploadingFile":
        return `Uploading ${publishProgress.name} (${publishProgress.index + 1}/${publishProgress.total})...`;
      case "uploadingLfsFile":
        return `Uploading ${publishProgress.name} (LFS)...`;
      case "committing":
        return "Committing to repository...";
      case "done":
        return "Complete";
      default:
        return "Publishing...";
    }
  });

  function handleDialogOpenChange(open: boolean) {
    // Prevent closing while publish is in progress
    if (!open && publishing) return;
    publishOpen = open;
  }
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
      {#if lastResult}
        <Button variant="outline" size="sm" onclick={openPublishDialog}>
          <Upload class="mr-2 h-3 w-3" />
          Publish to HF
        </Button>
      {/if}
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

<!-- Publish Dialog -->
<Dialog.Root open={publishOpen} onOpenChange={handleDialogOpenChange}>
  <Dialog.Content class="max-w-lg" showCloseButton={!publishing}>
    <Dialog.Header>
      <Dialog.Title>Publish to Hugging Face</Dialog.Title>
      <Dialog.Description>
        Upload your training data as a public or private dataset on the Hugging Face Hub.
      </Dialog.Description>
    </Dialog.Header>

    {#if publishResult}
      <!-- Success state -->
      <div class="space-y-4 py-4">
        <div class="flex flex-col items-center gap-3 rounded-lg border border-green-500/30 bg-green-500/5 p-6 text-center">
          <Check class="h-8 w-8 text-green-500" />
          <div>
            <p class="text-sm font-medium">Published successfully!</p>
            <p class="mt-1 text-xs text-muted-foreground">
              {publishResult.filesUploaded} file(s) uploaded
            </p>
          </div>
          <a
            href={publishResult.repoUrl}
            target="_blank"
            rel="noopener noreferrer"
            class="inline-flex items-center gap-2 text-sm font-medium text-primary underline"
          >
            Open on Hugging Face
            <ExternalLink class="h-3 w-3" />
          </a>
        </div>
      </div>
      <Dialog.Footer>
        <Button variant="outline" onclick={handlePublishAgain}>
          Publish again
        </Button>
        <Dialog.Close>
          <Button>Done</Button>
        </Dialog.Close>
      </Dialog.Footer>
    {:else}
      <!-- Config form -->
      <div class="space-y-4 py-4">
        <!-- Repo name -->
        <div>
          <label class="text-sm font-medium" for="repo-name">Repository name</label>
          <Input
            id="repo-name"
            bind:value={repoName}
            placeholder="my-training-data"
            class="mt-1 font-mono {repoNameError ? 'border-red-500' : ''}"
            oninput={() => (repoNameError = null)}
          />
          {#if repoNameError}
            <p class="mt-1 text-xs text-red-500">{repoNameError}</p>
          {:else}
            <p class="mt-1 text-xs text-muted-foreground">
              Lowercase, alphanumeric, hyphens, underscores, and dots only.
            </p>
          {/if}
        </div>

        <!-- Namespace -->
        <div>
          <label class="text-sm font-medium" for="namespace">Namespace</label>
          <select
            id="namespace"
            class="mt-1 flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            bind:value={selectedNamespace}
          >
            {#each namespaceOptions as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </div>

        <div class="flex gap-4">
          <!-- Visibility -->
          <div>
            <span class="text-sm font-medium">Visibility</span>
            <div class="mt-1 flex gap-2" role="radiogroup" aria-label="Repository visibility">
              <button
                type="button"
                role="radio"
                aria-checked={!isPrivate}
                class="rounded-md border px-3 py-1.5 text-sm transition-colors {!isPrivate
                  ? 'border-primary bg-primary/5 font-medium'
                  : 'border-border hover:border-muted-foreground/30'}"
                onclick={() => (isPrivate = false)}
              >
                Public
              </button>
              <button
                type="button"
                role="radio"
                aria-checked={isPrivate}
                class="rounded-md border px-3 py-1.5 text-sm transition-colors {isPrivate
                  ? 'border-primary bg-primary/5 font-medium'
                  : 'border-border hover:border-muted-foreground/30'}"
                onclick={() => (isPrivate = true)}
              >
                Private
              </button>
            </div>
          </div>

          <!-- License -->
          <div>
            <label class="text-sm font-medium" for="license-select">License</label>
            <select
              id="license-select"
              class="mt-1 flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              bind:value={license}
            >
              {#each licenses as lic}
                <option value={lic.value}>{lic.label}</option>
              {/each}
            </select>
          </div>
        </div>

        <!-- Files to upload -->
        <fieldset>
          <legend class="text-sm font-medium">Files to upload</legend>
          <div class="mt-2 space-y-1.5">
            {#each lastResult?.outputFiles ?? [] as file}
              <label
                class="flex items-center gap-2 rounded-md border p-2 text-sm cursor-pointer hover:bg-muted/50"
              >
                <input
                  type="checkbox"
                  checked={selectedFiles.includes(file)}
                  onchange={() => toggleFile(file)}
                  aria-label={file}
                />
                <FileJson class="h-4 w-4 text-muted-foreground" />
                <span class="truncate font-mono text-xs" title={file}
                  >{file.split("/").pop()}</span
                >
              </label>
            {/each}
          </div>
        </fieldset>

        <!-- Progress -->
        {#if publishing}
          <div class="space-y-2 rounded-lg border p-4">
            <div class="flex items-center gap-2 text-sm">
              <Loader2 class="h-4 w-4 animate-spin" />
              <span>{progressLabel}</span>
            </div>
            <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
              <div
                class="h-full bg-primary animate-pulse rounded-full"
                style="width: 100%"
              ></div>
            </div>
          </div>
        {/if}

        <!-- Error -->
        {#if publishError}
          <div class="rounded-lg border border-red-500/30 bg-red-500/5 p-3 text-sm text-red-500">
            {publishError}
          </div>
        {/if}
      </div>

      <Dialog.Footer>
        <Dialog.Close>
          <Button variant="outline" disabled={publishing}>Cancel</Button>
        </Dialog.Close>
        <Button
          onclick={handlePublish}
          disabled={publishing || !repoName.trim() || selectedFiles.length === 0}
        >
          {#if publishing}
            <Loader2 class="mr-2 h-3 w-3 animate-spin" />
            Publishing...
          {:else}
            <Upload class="mr-2 h-3 w-3" />
            Publish
          {/if}
        </Button>
      </Dialog.Footer>
    {/if}
  </Dialog.Content>
</Dialog.Root>
