<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import * as Card from "$lib/components/ui/card";
  import { Badge } from "$lib/components/ui/badge";
  import { Separator } from "$lib/components/ui/separator";
  import { FolderOpen, Plus, Scan, FileCode2 } from "@lucide/svelte";
  import { pickDirectory, discoverProjects, scanProject } from "$lib/api";
  import type { ProjectSummary, ProjectManifest } from "$lib/api";
  import { toast } from "svelte-sonner";
  import { projectStore } from "$lib/stores/projects.svelte.js";

  let projects = $state<ProjectSummary[]>([]);
  let scannedProjects = $derived(projectStore.scannedProjects);
  let scanning = $state<string | null>(null);
  let loading = $state(false);

  async function handleOpenDirectory() {
    try {
      const path = await pickDirectory();
      if (!path) return;

      loading = true;
      const discovered = await discoverProjects(path);
      projects = discovered;

      if (discovered.length === 0) {
        toast.info("No projects found in the selected directory.");
      } else if (discovered.length === 1) {
        // Single project: auto-scan immediately
        toast.info(`Found ${discovered[0].name}. Scanning...`);
        await handleScanProject(discovered[0]);
      } else {
        toast.success(`Found ${discovered.length} project(s). Click "Scan" on each to analyze.`);
      }
    } catch (e) {
      toast.error(`Failed to discover projects: ${e}`);
    } finally {
      loading = false;
    }
  }

  async function handleScanProject(project: ProjectSummary) {
    try {
      scanning = project.path;
      const manifest = await scanProject(project.path);
      projectStore.addScannedProject(manifest);
      toast.success(
        `Scanned ${manifest.name}: ${manifest.fileCount} files, ${manifest.totalLines.toLocaleString()} lines`,
      );
    } catch (e) {
      toast.error(`Failed to scan ${project.name}: ${e}`);
    } finally {
      scanning = null;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1048576).toFixed(1)} MB`;
  }

  function getLanguageDisplay(lang: unknown): string {
    if (typeof lang === "string") return lang;
    if (lang && typeof lang === "object" && "other" in lang)
      return (lang as { other: string }).other;
    return "Unknown";
  }
</script>

<div class="flex h-full flex-col">
  <!-- Header -->
  <div class="flex items-center justify-between border-b px-6 py-4">
    <div>
      <h1 class="text-lg font-semibold">Projects</h1>
      <p class="text-sm text-muted-foreground">
        Select a directory to discover and scan projects for training data
        generation.
      </p>
    </div>
    <Button onclick={handleOpenDirectory} disabled={loading}>
      {#if loading}
        <Scan class="mr-2 h-4 w-4 animate-spin" />
        Discovering...
      {:else}
        <FolderOpen class="mr-2 h-4 w-4" />
        Open Directory
      {/if}
    </Button>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-auto p-6">
    {#if projects.length === 0 && scannedProjects.length === 0}
      <!-- Empty state -->
      <div
        class="flex h-full flex-col items-center justify-center text-center text-muted-foreground"
      >
        <FileCode2 class="mb-4 h-12 w-12 opacity-40" />
        <p class="text-lg font-medium">No projects loaded</p>
        <p class="mt-1 text-sm">
          Click "Open Directory" to select a folder containing one or more
          projects.
        </p>
      </div>
    {:else}
      <div class="grid gap-4 sm:grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
        <!-- Discovered but not yet scanned -->
        {#each projects.filter((p) => !scannedProjects.some((s) => s.rootPath === p.path)) as project}
          <Card.Root>
            <Card.Header>
              <div class="flex items-center justify-between">
                <Card.Title class="text-base">{project.name}</Card.Title>
                <Badge variant="outline">{project.projectType}</Badge>
              </div>
              <Card.Description class="truncate text-xs">
                {project.path}
              </Card.Description>
            </Card.Header>
            <Card.Content>
              <div class="flex items-center gap-2 text-sm text-muted-foreground">
                {#if project.hasReadme}
                  <Badge variant="secondary" class="text-xs">README</Badge>
                {/if}
              </div>
            </Card.Content>
            <Card.Footer>
              <Button
                size="sm"
                class="w-full"
                onclick={() => handleScanProject(project)}
                disabled={scanning === project.path}
              >
                {#if scanning === project.path}
                  <Scan class="mr-2 h-3 w-3 animate-spin" />
                  Scanning...
                {:else}
                  <Plus class="mr-2 h-3 w-3" />
                  Scan Project
                {/if}
              </Button>
            </Card.Footer>
          </Card.Root>
        {/each}

        <!-- Scanned projects -->
        {#each scannedProjects as manifest}
          <Card.Root class="border-primary/20">
            <Card.Header>
              <div class="flex items-center justify-between">
                <Card.Title class="text-base">{manifest.name}</Card.Title>
                <Badge>{manifest.projectType}</Badge>
              </div>
              {#if manifest.description}
                <Card.Description class="line-clamp-2 text-xs">
                  {manifest.description}
                </Card.Description>
              {/if}
            </Card.Header>
            <Card.Content>
              <div class="space-y-2 text-sm">
                <div class="flex justify-between text-muted-foreground">
                  <span>Files</span>
                  <span class="font-medium text-foreground"
                    >{manifest.fileCount}</span
                  >
                </div>
                <div class="flex justify-between text-muted-foreground">
                  <span>Lines</span>
                  <span class="font-medium text-foreground"
                    >{manifest.totalLines.toLocaleString()}</span
                  >
                </div>
                <div class="flex justify-between text-muted-foreground">
                  <span>Size</span>
                  <span class="font-medium text-foreground"
                    >{formatBytes(manifest.totalSizeBytes)}</span
                  >
                </div>
                <Separator />
                <div class="flex flex-wrap gap-1">
                  {#each manifest.languages.slice(0, 5) as lang}
                    <Badge variant="secondary" class="text-xs">
                      {getLanguageDisplay(lang.language)}
                      ({lang.fileCount})
                    </Badge>
                  {/each}
                </div>
              </div>
            </Card.Content>
          </Card.Root>
        {/each}
      </div>
    {/if}
  </div>
</div>
