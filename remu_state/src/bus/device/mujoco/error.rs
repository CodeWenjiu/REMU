/// Errors from locating the engine, loading models, or transferring state.
#[derive(Debug, thiserror::Error)]
pub(crate) enum MujocoError {
    /// libmujoco could not be located, loaded, or is too old.
    #[error("mujoco engine unavailable: {0}")]
    Engine(String),

    /// The engine rejected a model; the message is MuJoCo's own.
    #[error("model load failed: {0}")]
    Xml(String),

    /// A state transfer was rejected (wrong buffer size, unknown name, ...).
    #[error("state error: {0}")]
    State(String),
}
