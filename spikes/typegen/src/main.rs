fn main() {
    println!("rscleaner-spike-typegen");
}

#[cfg(test)]
mod tests {
    use ts_rs::{Config, TS};

    #[derive(TS)]
    #[allow(dead_code)]
    struct SampleEvent {
        rule_id: String,
        removed: u64,
    }

    #[test]
    fn ts_contains_the_event_type() {
        let declaration = SampleEvent::decl(&Config::new());
        assert!(declaration.contains("SampleEvent"));
        assert!(declaration.contains("rule_id"));
    }
}
