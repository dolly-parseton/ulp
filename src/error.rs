use std::{error, fmt, path::PathBuf};

#[derive(Debug)]
pub enum CustomError {
    ElasticError(Box<dyn error::Error>),
    ParserInitialiseError(Box<dyn error::Error>),
    ParserRunError(Box<dyn error::Error>),
    TypeMapError(Box<dyn error::Error>),
    TypeCastError(Box<dyn error::Error>),
    TaskCreationError {
        err: Box<dyn error::Error>,
        job_id: uuid::Uuid,
        path: PathBuf,
    },
    StatGenerationError(Box<dyn error::Error>),
}
impl std::error::Error for CustomError {}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CustomError::ElasticError(e) => {
                write!(
                    f,
                    "ElasticError (Failed to handle elastic ingestion): {}",
                    e
                )
            }
            CustomError::ParserInitialiseError(e) => {
                write!(
                    f,
                    "ParserInitialiseError (Failed to initialise parser): {}",
                    e
                )
            }
            CustomError::ParserRunError(e) => {
                write!(f, "ParserRunError (Failed to run parser): {}", e)
            }
            CustomError::TypeMapError(e) => {
                write!(f, "TypeMapError (Failed to map Json object): {}", e)
            }
            CustomError::StatGenerationError(e) => {
                write!(
                    f,
                    "StatGenerationError (Failed to generate stats for parsed file): {}",
                    e
                )
            }
            CustomError::TypeCastError(e) => {
                write!(
                    f,
                    "TypeCastError (Failed to cast Json object field types): {}",
                    e
                )
            }
            CustomError::TaskCreationError { err, job_id, path } => {
                write!(
                    f,
                    "TaskError (An error occured whilst handling a Task), for job {} on path {}: {}",
                    job_id, path.display(), err
                )
            }
        }
    }
}
