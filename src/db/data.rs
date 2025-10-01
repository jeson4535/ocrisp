pub struct Data {}

impl Data {
    /// check if PDF is in data folder
    pub fn is_in(pdf_name: &str) -> bool {
        let file_path = format!("data/{}", pdf_name);
        let path = std::path::Path::new(&file_path);
        path.exists() && path.is_file()
    }
    
    pub fn count_pdfs() -> u64 {
        let data_dir = std::path::Path::new("data");
    
        if !data_dir.exists() || !data_dir.is_dir() {
            return 0;
        }
    
        match std::fs::read_dir(data_dir) {
            Ok(entries) => entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry.path().is_file()
                        && entry
                            .path()
                            .extension()
                            .map_or(false, |ext| ext.to_ascii_lowercase() == "pdf")
                })
                .count() as u64,
            Err(_) => 0,
        }
    }
    
    pub fn list_pdfs() -> Vec<std::path::PathBuf> {
        let data_dir = std::path::Path::new("data");

        if !data_dir.exists() || !data_dir.is_dir() {
            return Vec::new();
        }

        match std::fs::read_dir(data_dir) {
            Ok(entries) => entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file()
                        && path
                            .extension()
                            .map_or(false, |ext| ext.eq_ignore_ascii_case("pdf"))
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }
}