use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub path: PathBuf,
}

pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<DirEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    /// Filename input for save mode
    pub filename_input: Option<String>,
}

impl FileBrowser {
    pub fn new() -> Self {
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let mut browser = Self {
            current_dir: current_dir.clone(),
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            filename_input: None,
        };
        browser.read_dir();
        browser
    }

    pub fn new_save(default_filename: &str) -> Self {
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let mut browser = Self {
            current_dir: current_dir.clone(),
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            filename_input: Some(default_filename.to_string()),
        };
        browser.read_dir();
        browser
    }

    pub fn is_save_mode(&self) -> bool {
        self.filename_input.is_some()
    }

    /// Returns the full path for saving (current_dir + filename)
    pub fn save_path(&self) -> Option<PathBuf> {
        self.filename_input.as_ref().and_then(|name| {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(self.current_dir.join(trimmed))
            }
        })
    }

    pub fn read_dir(&mut self) {
        let mut entries = Vec::new();

        // Parent directory entry (unless at root)
        if self.current_dir.parent().is_some() {
            entries.push(DirEntry {
                name: "..".to_string(),
                is_dir: true,
                path: self.current_dir.parent().unwrap().to_path_buf(),
            });
        }

        if let Ok(read_dir) = fs::read_dir(&self.current_dir) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in read_dir.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                // Skip hidden files
                if name.starts_with('.') {
                    continue;
                }
                let is_dir = path.is_dir();
                let item = DirEntry { name, is_dir, path };
                if is_dir {
                    dirs.push(item);
                } else {
                    files.push(item);
                }
            }

            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            entries.extend(dirs);
            entries.extend(files);
        }

        self.entries = entries;
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn enter(&mut self) -> Option<PathBuf> {
        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                self.current_dir = entry.path.clone();
                self.read_dir();
                None
            } else {
                Some(entry.path.clone())
            }
        } else {
            None
        }
    }

    pub fn go_parent(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.read_dir();
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.adjust_scroll();
        }
    }

    pub fn move_down(&mut self) {
        if !self.entries.is_empty() && self.selected < self.entries.len() - 1 {
            self.selected += 1;
            self.adjust_scroll();
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.selected = self.selected.saturating_sub(page_size);
        self.adjust_scroll();
    }

    pub fn page_down(&mut self, page_size: usize) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + page_size).min(self.entries.len() - 1);
            self.adjust_scroll();
        }
    }

    fn adjust_scroll(&mut self) {
        // Will be called with visible_height from the UI later;
        // for now keep a reasonable default
        let visible = 20usize;
        self.adjust_scroll_with_height(visible);
    }

    pub fn adjust_scroll_with_height(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected - visible_height + 1;
        }
    }
}
