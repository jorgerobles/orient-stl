pub mod types;
pub mod island;
pub mod volume;
pub mod placement;
pub mod raft;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_compiles() {
        // Basic smoke test that the crate compiles
        let _config = types::SupportConfig::default();
    }
}
