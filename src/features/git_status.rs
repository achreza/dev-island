use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRepoStatus {
    pub branch: String,
    pub modified_count: usize,
    pub untracked_count: usize,
    pub staged_count: usize,
    pub is_clean: bool,
}

impl GitRepoStatus {
    #[allow(dead_code)]
    pub fn summary_badge(&self) -> String {
        if self.is_clean {
            format!(" {} [clean]", self.branch)
        } else {
            let mut parts = Vec::new();
            if self.modified_count > 0 {
                parts.push(format!("~{}", self.modified_count));
            }
            if self.untracked_count > 0 {
                parts.push(format!("?{}", self.untracked_count));
            }
            if self.staged_count > 0 {
                parts.push(format!("+{}", self.staged_count));
            }
            format!(" {} ({})", self.branch, parts.join(" "))
        }
    }
}

pub struct GitStatusMonitor {
    pub repo_path: PathBuf,
    pub cached_status: Option<GitRepoStatus>,
    pub last_checked: Option<Instant>,
    pub cache_ttl_secs: f32,
}

impl GitStatusMonitor {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
        Self {
            repo_path: repo_path.as_ref().to_path_buf(),
            cached_status: None,
            last_checked: None,
            cache_ttl_secs: 5.0, // 5 seconds cache prevents disk thrashing
        }
    }

    pub fn get_status(&mut self) -> Option<GitRepoStatus> {
        if let Some(last) = self.last_checked {
            if last.elapsed().as_secs_f32() < self.cache_ttl_secs {
                return self.cached_status.clone();
            }
        }

        let status = self.query_status();
        self.cached_status = status.clone();
        self.last_checked = Some(Instant::now());
        status
    }

    fn query_status(&self) -> Option<GitRepoStatus> {
        let git_dir = if self.repo_path.join(".git").is_dir() {
            self.repo_path.join(".git")
        } else if self.repo_path.join(".git").is_file() {
            // Worktree or submodule
            return None;
        } else {
            return None;
        };

        let head_path = git_dir.join("HEAD");
        let branch = Self::parse_head_file(&head_path)?;

        // Query modified/untracked files via git status --porcelain
        let (modified, untracked, staged) = self.query_porcelain_counts();
        let is_clean = modified == 0 && untracked == 0 && staged == 0;

        Some(GitRepoStatus {
            branch,
            modified_count: modified,
            untracked_count: untracked,
            staged_count: staged,
            is_clean,
        })
    }

    pub fn parse_head_file(head_path: &Path) -> Option<String> {
        let content = fs::read_to_string(head_path).ok()?;
        let trimmed = content.trim();

        if let Some(branch_ref) = trimmed.strip_prefix("ref: refs/heads/") {
            Some(branch_ref.to_string())
        } else if trimmed.len() >= 7 {
            // Detached HEAD commit
            Some(trimmed[..7].to_string())
        } else {
            None
        }
    }

    fn query_porcelain_counts(&self) -> (usize, usize, usize) {
        #[cfg(windows)]
        use std::os::windows::process::CommandExt;

        let mut cmd = Command::new("git");
        cmd.arg("status").arg("--porcelain").current_dir(&self.repo_path);

        #[cfg(windows)]
        {
            // CREATE_NO_WINDOW = 0x08000000 prevents black console popup
            cmd.creation_flags(0x08000000);
        }

        let output = match cmd.output() {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
            _ => return (0, 0, 0),
        };

        Self::parse_porcelain_output(&output)
    }

    pub fn parse_porcelain_output(output: &str) -> (usize, usize, usize) {
        let mut modified = 0;
        let mut untracked = 0;
        let mut staged = 0;

        for line in output.lines() {
            if line.len() < 2 {
                continue;
            }
            let index_status = line.as_bytes()[0];
            let worktree_status = line.as_bytes()[1];

            if line.starts_with("??") {
                untracked += 1;
            } else {
                if index_status == b'M' || index_status == b'A' || index_status == b'D' {
                    staged += 1;
                }
                if worktree_status == b'M' || worktree_status == b'D' {
                    modified += 1;
                }
            }
        }

        (modified, untracked, staged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_porcelain_output() {
        let output = " M src/main.rs\n?? new_file.txt\nM  Cargo.toml\nA  new_module.rs\n";
        let (m, u, s) = GitStatusMonitor::parse_porcelain_output(output);
        assert_eq!(m, 1);
        assert_eq!(u, 1);
        assert_eq!(s, 2);
    }

    #[test]
    fn test_git_repo_summary_badge() {
        let clean = GitRepoStatus {
            branch: "main".to_string(),
            modified_count: 0,
            untracked_count: 0,
            staged_count: 0,
            is_clean: true,
        };
        assert_eq!(clean.summary_badge(), " main [clean]");

        let dirty = GitRepoStatus {
            branch: "feat/dev".to_string(),
            modified_count: 2,
            untracked_count: 1,
            staged_count: 0,
            is_clean: false,
        };
        assert_eq!(dirty.summary_badge(), " feat/dev (~2 ?1)");
    }
}
