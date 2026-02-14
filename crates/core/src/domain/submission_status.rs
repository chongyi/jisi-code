#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubmissionStatus {
    Pending,
    Running,
    Accepted,
    WrongAnswer,
    TimeLimitExceeded,
    RuntimeError,
    CompileError,
    InternalError,
}
