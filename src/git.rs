use std::{
    path::Path,
    process::{Command, Stdio},
};

pub struct GitManager<'p> {
    pub dir: &'p Path,
}

impl<'p> GitManager<'p> {
    /// Creates a new `GitManager`.
    ///
    /// # Parameters
    /// - `dir`: The directory where the git repository is initialized.
    ///
    /// # Returns
    /// The new manager.
    pub fn new(dir: &'p Path) -> Self {
        Self { dir }
    }

    /// Pulls files from the remote repository. Equivalent to `git pull`.
    ///
    /// # Returns
    /// Whether the process succeeded.
    pub fn pull_files(&self) -> bool {
        match Command::new("git")
            .arg("pull")
            .current_dir(self.dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(o) => o.success(),
            Err(_) => false,
        }
    }

    /// Adds all files to be staged to source control. Equivalent to
    /// `git commit -A`.
    ///
    /// # Returns
    /// Whether the process succeeded.
    pub fn add_all_files(&self) -> bool {
        match Command::new("git")
            .arg("add")
            .arg("-A")
            .current_dir(self.dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(o) => o.success(),
            Err(_) => false,
        }
    }

    /// Commits the files to source control. Equivalent to
    /// `git commit -m "<msg>"`.
    ///
    /// # Parameters
    /// - `commit_msg`: The commit message.
    ///
    /// # Returns
    /// Whether the process succeeded.
    pub fn commit_files(&self, commit_msg: &str) -> bool {
        match Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(commit_msg)
            .current_dir(self.dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(o) => o.success(),
            Err(_) => false,
        }
    }

    /// Pushes the files to the remote repository.
    ///
    /// # Returns
    /// Whether the process succeeded.
    pub fn push_files(&self) -> bool {
        match Command::new("git")
            .arg("push")
            .current_dir(self.dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(o) => o.success(),
            Err(_) => false,
        }
    }
}
