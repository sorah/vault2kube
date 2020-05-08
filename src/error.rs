use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Some rules are failed to run")]
    RuleExecutionFailed,
    #[error("Unsupported kind for rollout")]
    UnsupportedRolloutKind,
}
