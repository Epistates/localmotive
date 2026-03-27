import type {
  ProjectManifest,
  PipelineResult,
  ProgressEvent,
} from "$lib/api/types.js";

/** Shared reactive store using Svelte 5 class with $state fields.
 *  Mutations use .push() on arrays (in-place) so cross-module reactivity works.
 *  Direct reassignment of module-level $state breaks cross-file tracking.
 */
class ProjectStore {
  scannedProjects = $state<ProjectManifest[]>([]);
  selectedProjectId = $state<string | null>(null);
  lastResult = $state<PipelineResult | null>(null);
  progressEvents = $state<ProgressEvent[]>([]);
  isGenerating = $state(false);

  get selectedProject(): ProjectManifest | null {
    return (
      this.scannedProjects.find(
        (p) => p.id === this.selectedProjectId,
      ) ?? null
    );
  }

  addScannedProject(manifest: ProjectManifest) {
    this.scannedProjects.push(manifest);
  }

  addProgressEvent(event: ProgressEvent) {
    this.progressEvents.push(event);
  }

  clearProgressEvents() {
    this.progressEvents = [];
  }
}

export const projectStore = new ProjectStore();
