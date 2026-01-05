use crate::{Error, Result};
use git2::{
    Cred, FetchOptions, IndexAddOption, Oid, PushOptions, RemoteCallbacks, Repository, Signature,
    StatusOptions,
};
use std::path::PathBuf;

pub struct GitOperations {
    repo_path: PathBuf,
}

impl GitOperations {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    pub fn init(&self) -> Result<()> {
        if !self.repo_path.join(".git").exists() {
            Repository::init(&self.repo_path)
                .map_err(|e| Error::InvalidData(format!("Failed to init git repo: {}", e)))?;
        }
        Ok(())
    }

    fn get_repo(&self) -> Result<Repository> {
        Repository::open(&self.repo_path)
            .map_err(|e| Error::InvalidData(format!("Failed to open git repo: {}", e)))
    }

    pub fn is_initialized(&self) -> bool {
        self.repo_path.join(".git").exists()
    }

    pub fn add_all(&self) -> Result<()> {
        let repo = self.get_repo()?;
        let mut index = repo
            .index()
            .map_err(|e| Error::InvalidData(format!("Failed to get index: {}", e)))?;

        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .map_err(|e| Error::InvalidData(format!("Failed to add files: {}", e)))?;

        index
            .write()
            .map_err(|e| Error::InvalidData(format!("Failed to write index: {}", e)))?;

        Ok(())
    }

    pub fn commit(&self, message: &str) -> Result<Oid> {
        let repo = self.get_repo()?;
        let signature = Signature::now("MooTimer", "mootimer@local")
            .map_err(|e| Error::InvalidData(format!("Failed to create signature: {}", e)))?;

        let mut index = repo
            .index()
            .map_err(|e| Error::InvalidData(format!("Failed to get index: {}", e)))?;

        let tree_id = index
            .write_tree()
            .map_err(|e| Error::InvalidData(format!("Failed to write tree: {}", e)))?;

        let tree = repo
            .find_tree(tree_id)
            .map_err(|e| Error::InvalidData(format!("Failed to find tree: {}", e)))?;

        let parent_commit = match repo.head() {
            Ok(head) => {
                let commit = head.peel_to_commit().map_err(|e| {
                    Error::InvalidData(format!("Failed to get parent commit: {}", e))
                })?;
                Some(commit)
            }
            Err(_) => None,
        };

        let commit_id = if let Some(parent) = &parent_commit {
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &[parent],
            )
            .map_err(|e| Error::InvalidData(format!("Failed to create commit: {}", e)))?
        } else {
            repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])
                .map_err(|e| {
                    Error::InvalidData(format!("Failed to create initial commit: {}", e))
                })?
        };

        Ok(commit_id)
    }

    pub fn has_changes(&self) -> Result<bool> {
        let repo = self.get_repo()?;
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        opts.include_ignored(false);

        let statuses = repo
            .statuses(Some(&mut opts))
            .map_err(|e| Error::InvalidData(format!("Failed to get status: {}", e)))?;

        Ok(!statuses.is_empty())
    }

    pub fn add_remote(&self, name: &str, url: &str) -> Result<()> {
        let repo = self.get_repo()?;

        let _ = repo.remote_delete(name);

        repo.remote(name, url)
            .map_err(|e| Error::InvalidData(format!("Failed to add remote: {}", e)))?;

        Ok(())
    }

    pub fn pull(&self, remote_name: &str, branch: &str) -> Result<()> {
        let repo = self.get_repo()?;

        let mut remote = repo
            .find_remote(remote_name)
            .map_err(|e| Error::InvalidData(format!("Failed to find remote: {}", e)))?;

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        remote
            .fetch(&[branch], Some(&mut fetch_opts), None)
            .map_err(|e| Error::InvalidData(format!("Failed to fetch: {}", e)))?;

        let fetch_head = repo
            .find_reference("FETCH_HEAD")
            .map_err(|e| Error::InvalidData(format!("Failed to find FETCH_HEAD: {}", e)))?;

        let fetch_commit = repo
            .reference_to_annotated_commit(&fetch_head)
            .map_err(|e| Error::InvalidData(format!("Failed to get fetch commit: {}", e)))?;

        let analysis = repo
            .merge_analysis(&[&fetch_commit])
            .map_err(|e| Error::InvalidData(format!("Failed to analyze merge: {}", e)))?;

        if analysis.0.is_up_to_date() {
            return Ok(());
        } else if analysis.0.is_fast_forward() {
            let refname = format!("refs/heads/{}", branch);
            let mut reference = repo
                .find_reference(&refname)
                .map_err(|e| Error::InvalidData(format!("Failed to find reference: {}", e)))?;

            reference
                .set_target(fetch_commit.id(), "Fast-forward merge")
                .map_err(|e| Error::InvalidData(format!("Failed to fast-forward: {}", e)))?;

            repo.set_head(&refname)
                .map_err(|e| Error::InvalidData(format!("Failed to set HEAD: {}", e)))?;

            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .map_err(|e| Error::InvalidData(format!("Failed to checkout: {}", e)))?;
        } else {
            return Err(Error::InvalidData(
                "Merge conflicts detected. Please resolve manually.".to_string(),
            ));
        }

        Ok(())
    }

    pub fn push(&self, remote_name: &str, branch: &str) -> Result<()> {
        let repo = self.get_repo()?;

        let mut remote = repo
            .find_remote(remote_name)
            .map_err(|e| Error::InvalidData(format!("Failed to find remote: {}", e)))?;

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        let mut push_opts = PushOptions::new();
        push_opts.remote_callbacks(callbacks);

        let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);

        remote
            .push(&[&refspec], Some(&mut push_opts))
            .map_err(|e| Error::InvalidData(format!("Failed to push: {}", e)))?;

        Ok(())
    }

    pub fn current_branch(&self) -> Result<String> {
        let repo = self.get_repo()?;

        let head = repo
            .head()
            .map_err(|e| Error::InvalidData(format!("Failed to get HEAD: {}", e)))?;

        let branch = head
            .shorthand()
            .ok_or_else(|| Error::InvalidData("Failed to get branch name".to_string()))?;

        Ok(branch.to_string())
    }

    pub fn last_commit_message(&self) -> Result<String> {
        let repo = self.get_repo()?;

        let head = repo
            .head()
            .map_err(|e| Error::InvalidData(format!("Failed to get HEAD: {}", e)))?;

        let commit = head
            .peel_to_commit()
            .map_err(|e| Error::InvalidData(format!("Failed to get commit: {}", e)))?;

        let message = commit
            .message()
            .ok_or_else(|| Error::InvalidData("Failed to get commit message".to_string()))?;

        Ok(message.to_string())
    }

    pub fn get_sync_status(&self, remote_name: &str, branch: &str) -> Result<(usize, usize)> {
        let repo = self.get_repo()?;

        let local_ref = format!("refs/heads/{}", branch);
        let remote_ref = format!("refs/remotes/{}/{}", remote_name, branch);

        let local_oid = repo
            .refname_to_id(&local_ref)
            .map_err(|e| Error::InvalidData(format!("Failed to get local ref: {}", e)))?;

        let remote_oid = match repo.refname_to_id(&remote_ref) {
            Ok(oid) => oid,
            Err(_) => return Ok((0, 0)),
        };

        let (ahead, behind) = repo
            .graph_ahead_behind(local_oid, remote_oid)
            .map_err(|e| Error::InvalidData(format!("Failed to get ahead/behind: {}", e)))?;

        Ok((ahead, behind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_repository() {
        let temp_dir = TempDir::new().unwrap();
        let git_ops = GitOperations::new(temp_dir.path().to_path_buf());

        assert!(!git_ops.is_initialized());
        git_ops.init().unwrap();
        assert!(git_ops.is_initialized());
    }

    #[test]
    fn test_commit() {
        let temp_dir = TempDir::new().unwrap();
        let git_ops = GitOperations::new(temp_dir.path().to_path_buf());

        git_ops.init().unwrap();

        std::fs::write(temp_dir.path().join("test.txt"), "Hello").unwrap();

        git_ops.add_all().unwrap();
        let commit_id = git_ops.commit("Initial commit").unwrap();

        assert!(!commit_id.is_zero());
        assert_eq!(git_ops.last_commit_message().unwrap(), "Initial commit");
    }

    #[test]
    fn test_has_changes() {
        let temp_dir = TempDir::new().unwrap();
        let git_ops = GitOperations::new(temp_dir.path().to_path_buf());

        git_ops.init().unwrap();

        assert!(!git_ops.has_changes().unwrap());

        std::fs::write(temp_dir.path().join("test.txt"), "Hello").unwrap();

        assert!(git_ops.has_changes().unwrap());

        git_ops.add_all().unwrap();
        git_ops.commit("Add test file").unwrap();

        assert!(!git_ops.has_changes().unwrap());
    }

    #[test]
    fn test_current_branch() {
        let temp_dir = TempDir::new().unwrap();
        let git_ops = GitOperations::new(temp_dir.path().to_path_buf());

        git_ops.init().unwrap();

        std::fs::write(temp_dir.path().join("test.txt"), "Hello").unwrap();
        git_ops.add_all().unwrap();
        git_ops.commit("Initial commit").unwrap();

        let branch = git_ops.current_branch().unwrap();
        assert!(branch == "main" || branch == "master");
    }
}