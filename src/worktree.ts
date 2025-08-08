import { execSync } from "node:child_process";

export interface Worktree {
  path: string;
  branch: string;
  HEAD: string;
}

export function listWorktrees(baseDir: string): Worktree[] {
  try {
    const output = execSync("git worktree list --porcelain", {
      cwd: baseDir,
      encoding: "utf8",
    });

    const worktrees: Worktree[] = [];
    const lines = output.trim().split("\n");

    let currentWorktree: Partial<Worktree> = {};

    for (const line of lines) {
      if (line.startsWith("worktree ")) {
        currentWorktree.path = line.substring(9);
      } else if (line.startsWith("HEAD ")) {
        currentWorktree.HEAD = line.substring(5);
      } else if (line.startsWith("branch ")) {
        currentWorktree.branch = line.substring(7).replace("refs/heads/", "");
      } else if (line === "") {
        // Empty line indicates end of current worktree entry
        if (currentWorktree.path && currentWorktree.HEAD) {
          worktrees.push({
            path: currentWorktree.path,
            branch: currentWorktree.branch || "(detached)",
            HEAD: currentWorktree.HEAD,
          });
        }
        currentWorktree = {};
      }
    }

    // Handle the last worktree if there's no trailing empty line
    if (currentWorktree.path && currentWorktree.HEAD) {
      worktrees.push({
        path: currentWorktree.path,
        branch: currentWorktree.branch || "(detached)",
        HEAD: currentWorktree.HEAD,
      });
    }

    return worktrees;
  } catch (_error) {
    // If git command fails (not a git repo, no worktrees, etc.), return empty array
    return [];
  }
}
