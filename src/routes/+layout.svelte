<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { Toaster } from "$lib/components/ui/sonner";
  import {
    FolderOpen,
    Cpu,
    Play,
    Eye,
    Settings,
    Train,
  } from "@lucide/svelte";

  let { children } = $props();

  const navItems = [
    { href: "/", label: "Projects", icon: FolderOpen },
    { href: "/generate", label: "Generate", icon: Cpu },
    { href: "/progress", label: "Progress", icon: Play },
    { href: "/preview", label: "Preview", icon: Eye },
    { href: "/settings", label: "Settings", icon: Settings },
  ];

  function isActive(href: string): boolean {
    if (href === "/") return page.url.pathname === "/";
    return page.url.pathname.startsWith(href);
  }
</script>

<div class="flex h-screen overflow-hidden">
  <!-- Sidebar -->
  <nav
    class="flex w-56 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground"
  >
    <div class="flex items-center gap-2 border-b border-sidebar-border px-4 py-4">
      <Train class="h-5 w-5 text-sidebar-primary" />
      <span class="text-sm font-semibold text-sidebar-primary">Localmotive</span>
    </div>

    <div class="flex flex-1 flex-col gap-1 p-2">
      {#each navItems as item}
        <a
          href={item.href}
          class="flex items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors {isActive(item.href)
            ? 'bg-sidebar-accent text-sidebar-accent-foreground font-medium'
            : 'text-sidebar-foreground hover:bg-sidebar-accent/50'}"
        >
          <item.icon class="h-4 w-4" />
          {item.label}
        </a>
      {/each}
    </div>

    <div class="border-t border-sidebar-border p-3 text-xs text-muted-foreground">
      v0.1.0
    </div>
  </nav>

  <!-- Main content -->
  <main class="flex-1 overflow-auto">
    {@render children()}
  </main>
</div>

<Toaster />
