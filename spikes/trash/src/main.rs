fn main() {
    println!("rscleaner-spike-trash");
}

#[cfg(test)]
mod tests {
    #[test]
    fn trash_moves_a_temp_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let file = dir.path().join("probe.txt");
        std::fs::write(&file, b"probe").expect("write");
        trash::delete(&file).expect("trash");
        assert!(!file.exists());
    }
}
